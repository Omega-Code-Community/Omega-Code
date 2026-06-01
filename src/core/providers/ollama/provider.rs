use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use crate::core::client::{AiClient, ClientConfig};
use crate::core::transport::retry::RetryPolicy;
use crate::core::providers::provider::Provider;
use crate::core::providers::types::{CompletionRequest, CompletionResponse};

pub struct OllamaProvider {
    client: AiClient,
    base_url: String,
}

impl OllamaProvider {
    pub fn new(base_url: String) -> Result<Self> {
        let config = ClientConfig::default();
        let retry = RetryPolicy::default();
        let client = AiClient::new(config, retry)?;
        
        Ok(Self {
            client,
            base_url,
        })
    }
}

#[async_trait]
impl Provider for OllamaProvider {
    fn client(&self) -> &AiClient {
        &self.client
    }
    
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        
        let url = format!("{}/api/chat", self.base_url);
        
        let response: serde_json::Value = self.client
            .post_json(&url, headers, &serde_json::json!({
                "model": request.model,
                "messages": request.messages,
                "stream": request.stream,
                "temperature": request.temperature,
                "max_tokens": request.max_tokens,
            }))
            .await
            .map_err(|e| anyhow::anyhow!("Ollama API error: {}", e))?;

        let content = response["message"]["content"].as_str().unwrap_or("").to_string();
        let finish_reason = match response["done"].as_bool() {
            Some(true) => Some(crate::core::providers::types::FinishReason::Stop),
            _ => None,
        };

        Ok(CompletionResponse {
            content,
            finish_reason,
            usage: None,
        })
    }
}