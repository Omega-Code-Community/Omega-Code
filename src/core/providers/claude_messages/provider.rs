use async_trait::async_trait;
use reqwest::Client;
use crate::core::providers::claude_messages::request::ClaudeRequest;
use crate::core::providers::claude_messages::response::ClaudeResponse;
use crate::core::providers::provider::Provider;
use crate::core::providers::types::{CompletionRequest, CompletionResponse};

pub struct ClaudeProvider {
    client: Client,

    api_key: String,
}

impl ClaudeProvider {
    pub fn new(
        api_key: String,
    ) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }
}

#[async_trait]
impl Provider for ClaudeProvider {
    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse> {

        let body =
            ClaudeRequest::from(request);

        let resp = self
            .client
            .post(
                "https://api.anthropic.com/v1/messages"
            )
            .header(
                "x-api-key",
                &self.api_key,
            )
            .header(
                "anthropic-version",
                "2023-06-01",
            )
            .json(&body)
            .send()
            .await?;

        let response: ClaudeResponse =
            resp.json().await?;

        Ok(response.into())
    }
}