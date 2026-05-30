use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use crate::core::client::{AiClient, ClientConfig};
use crate::core::transport::retry::RetryPolicy;
use crate::core::providers::claude_messages::request::ClaudeRequest;
use crate::core::providers::claude_messages::response::ClaudeResponse;
use crate::core::providers::provider::Provider;
use crate::core::providers::types::{CompletionRequest, CompletionResponse};

pub struct ClaudeProvider {
    client: AiClient,
    api_key: String,
}

impl ClaudeProvider {
    pub fn new(api_key: String) -> Result<Self> {
        let config = ClientConfig::default();
        let retry = RetryPolicy::default();
        let client = AiClient::new(config, retry)?;
        
        Ok(Self {
            client,
            api_key,
        })
    }
}

#[async_trait]
impl Provider for ClaudeProvider {
    fn client(&self) -> &AiClient {
        &self.client
    }
    
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let claude_request = ClaudeRequest::from_completion_request(request);
        
        let mut headers = HashMap::new();
        headers.insert("x-api-key".to_string(), self.api_key.clone());
        headers.insert("anthropic-version".to_string(), "2023-06-01".to_string());
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        
        let url = "https://api.anthropic.com/v1/messages";
        
        let response: ClaudeResponse = self.client
            .post_json(url, headers, &claude_request)
            .await
            .map_err(|e| anyhow::anyhow!("Claude API error: {}", e))?;

        Ok(CompletionResponse::from(response))
    }
}