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

    async fn list_models(&self) -> Result<Vec<String>> {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let url = format!("{}/api/tags", self.base_url);

        let value: serde_json::Value = self
            .client
            .get_json(&url, headers, &HashMap::<String, String>::new())
            .await
            .map_err(|e| anyhow::anyhow!("Ollama tags API error: {}", e))?;

        Ok(parse_model_names(&value))
    }
}

/// 从 Ollama `/api/tags` 响应（`{"models":[{"name":...}]}`）中提取模型名。
fn parse_model_names(value: &serde_json::Value) -> Vec<String> {
    value
        .get("models")
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| item.get("name").and_then(|n| n.as_str()))
                .map(|s| s.to_string())
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::parse_model_names;
    use serde_json::json;

    #[test]
    fn extracts_names_from_models_array() {
        let value = json!({
            "models": [
                {"name": "llama3:latest", "size": 1},
                {"name": "qwen2.5:7b", "size": 2}
            ]
        });
        assert_eq!(parse_model_names(&value), vec!["llama3:latest", "qwen2.5:7b"]);
    }

    #[test]
    fn missing_or_malformed_models_yields_empty() {
        assert!(parse_model_names(&json!({})).is_empty());
        assert!(parse_model_names(&json!({"models": "nope"})).is_empty());
        assert!(parse_model_names(&json!({"models": [{"size": 1}]})).is_empty());
    }
}