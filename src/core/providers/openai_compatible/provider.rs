use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use crate::core::client::{AiClient, ClientConfig};
use crate::core::transport::retry::RetryPolicy;
use crate::core::providers::openai_compatible::request::OpenAiRequest;
use crate::core::providers::openai_compatible::response::OpenAiResponse;
use crate::core::providers::provider::Provider;
use crate::core::providers::types::{CompletionRequest, CompletionResponse};

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
}