use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use crate::core::client::{AiClient, ClientConfig};
use crate::core::transport::retry::RetryPolicy;
use crate::core::providers::provider::Provider;
use crate::core::providers::types::{CompletionRequest, CompletionResponse};

pub struct GeminiProvider {
    client: AiClient,
    api_key: String,
    model: String,
}

impl GeminiProvider {
    pub fn new(api_key: String, model: String) -> Result<Self> {
        let config = ClientConfig::default();
        let retry = RetryPolicy::default();
        let client = AiClient::new(config, retry)?;
        
        Ok(Self {
            client,
            api_key,
            model,
        })
    }
}

#[async_trait]
impl Provider for GeminiProvider {
    fn client(&self) -> &AiClient {
        &self.client
    }
    
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        
        let url = format!("https://generativelanguage.googleapis.com/v1/models/{model}:generateContent?key={api_key}", 
            model = self.model,
            api_key = self.api_key
        );
        
        let response: serde_json::Value = self.client
            .post_json(&url, headers, &serde_json::json!({
                "contents": request.messages.iter().map(|m| {
                    serde_json::json!({
                        "role": match m.role {
                            crate::core::providers::types::Role::System => "system",
                            crate::core::providers::types::Role::User => "user",
                            crate::core::providers::types::Role::Assistant => "model",
                        },
                        "parts": [serde_json::json!({"text": m.content})]
                    })
                }).collect::<Vec<_>>(),
                "generationConfig": {
                    "temperature": request.temperature.unwrap_or(0.7),
                    "maxOutputTokens": request.max_tokens,
                }
            }))
            .await
            .map_err(|e| anyhow::anyhow!("Gemini API error: {}", e))?;

        let content = response["candidates"][0]["content"]["parts"][0]["text"].as_str().unwrap_or("").to_string();
        
        Ok(CompletionResponse {
            content,
            finish_reason: None,
            usage: None,
        })
    }
}