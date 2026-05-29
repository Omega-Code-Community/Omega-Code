use async_trait::async_trait;
use reqwest::Client;
use crate::core::providers::gemini::request::GeminiRequest;
use crate::core::providers::gemini::response::GeminiResponse;
use crate::core::providers::provider::Provider;
use crate::core::providers::types::{CompletionRequest, CompletionResponse};

pub struct GeminiProvider {
    client: Client,

    api_key: String,
}

#[async_trait]
impl Provider for GeminiProvider {
    async fn complete(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse> {

        let model =
            request.model.clone();

        let body =
            GeminiRequest::from(request);

        let resp = self
            .client
            .post(format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
                model,
                self.api_key
            ))
            .json(&body)
            .send()
            .await?;

        let response: GeminiResponse =
            resp.json().await?;

        Ok(response.into())
    }
}