use anyhow::Result;
use async_trait::async_trait;
use futures_util::StreamExt;
use std::collections::HashMap;
use crate::core::client::{AiClient, ClientConfig};
use crate::core::transport::retry::RetryPolicy;
use crate::core::providers::openai_compatible::request::OpenAiRequest;
use crate::core::providers::openai_compatible::response::{OpenAiResponse, OpenAiStreamChunk};
use crate::core::providers::provider::Provider;
use crate::core::providers::types::{
    CompletionRequest, CompletionResponse, CompletionStream, FinishReason, StreamDelta,
};

pub struct OpenAiCompatibleProvider {
    client: AiClient,
    api_key: String,
    base_url: String,
}

impl OpenAiCompatibleProvider {
    pub fn new(api_key: String, base_url: String) -> Result<Self> {
        let config = ClientConfig::default();
        let retry = RetryPolicy::default();
        let client = AiClient::new(config, retry)?;
        
        Ok(Self {
            client,
            api_key,
            base_url,
        })
    }
}

#[async_trait]
impl Provider for OpenAiCompatibleProvider {
    fn client(&self) -> &AiClient {
        &self.client
    }
    
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let openai_request = OpenAiRequest::from_completion_request(request);
        
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), format!("Bearer {}", self.api_key));
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        
        let url = format!("{}/chat/completions", self.base_url);
        
        let response: OpenAiResponse = self.client
            .post_json(&url, headers, &openai_request)
            .await
            .map_err(|e| anyhow::anyhow!("OpenAI API error: {}", e))?;

        Ok(CompletionResponse::from(response))
    }

    async fn complete_stream(
        &self,
        mut request: CompletionRequest,
    ) -> Result<CompletionStream> {
        request.stream = true;
        let openai_request = OpenAiRequest::from_completion_request(request);

        let mut headers = HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            format!("Bearer {}", self.api_key),
        );
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let url = format!("{}/chat/completions", self.base_url);

        let sse = self
            .client
            .post_sse(&url, headers, &openai_request)
            .await
            .map_err(|e| anyhow::anyhow!("OpenAI stream error: {}", e))?;

        let stream = sse.filter_map(|event_result| async move {
            match event_result {
                Ok(event) => {
                    let data = event.data.trim();
                    if data.is_empty() || data == "[DONE]" {
                        return None;
                    }
                    match serde_json::from_str::<OpenAiStreamChunk>(data) {
                        Ok(chunk) => {
                            let choice = chunk.choices.first();
                            let content = choice
                                .and_then(|c| c.delta.content.clone())
                                .unwrap_or_default();
                            let finish_reason = choice
                                .and_then(|c| c.finish_reason.as_ref())
                                .map(|r| match r.as_str() {
                                    "stop" => FinishReason::Stop,
                                    "length" => FinishReason::Length,
                                    _ => FinishReason::Error,
                                });
                            if content.is_empty() && finish_reason.is_none() {
                                None
                            } else {
                                Some(Ok(StreamDelta {
                                    content,
                                    finish_reason,
                                }))
                            }
                        }
                        Err(e) => Some(Err(anyhow::anyhow!(
                            "Failed to parse stream chunk: {}",
                            e
                        ))),
                    }
                }
                Err(e) => Some(Err(anyhow::anyhow!("SSE error: {}", e))),
            }
        });

        Ok(Box::pin(stream))
    }
}