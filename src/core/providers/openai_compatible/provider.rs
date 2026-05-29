use anyhow::Result;
use async_trait::async_trait;
use reqwest::Client;
use crate::core::providers::openai_compatible::request::OpenAiRequest;
use crate::core::providers::openai_compatible::response::OpenAiResponse;
use crate::core::providers::provider::Provider;
use crate::core::providers::types::{CompletionRequest, CompletionResponse};

pub struct OpenAiCompatibleProvider {
    client: Client,

    api_key: String,

    base_url: String,
}

impl OpenAiCompatibleProvider {
    pub fn new(
        api_key: String,
        base_url: String,
    ) -> Self {
        Self {
            client: Client::new(),
            api_key,
            base_url,
        }
    }
}

#[async_trait]
impl Provider for OpenAiCompatibleProvider {
    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse> {
        let body = OpenAiRequest::from(request);

        let resp = self
            .client
            .post(format!(
                "{}/chat/completions",
                self.base_url
            ))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        let response: OpenAiResponse =
            resp.json().await?;

        Ok(response.into())
    }
}