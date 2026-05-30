use serde::Deserialize;
use crate::core::providers::types::{CompletionResponse, FinishReason, Usage};

#[derive(Deserialize)]
pub struct OpenAiResponse {
    pub choices: Vec<OpenAiChoice>,
    pub usage: Option<OpenAiUsage>,
}

#[derive(Deserialize)]
pub struct OpenAiChoice {
    pub message: OpenAiAssistantMessage,
    pub finish_reason: Option<String>,
}

#[derive(Deserialize)]
pub struct OpenAiAssistantMessage {
    pub content: String,
}

#[derive(Deserialize)]
pub struct OpenAiUsage {
    pub prompt_tokens: Option<u32>,
    pub completion_tokens: Option<u32>,
    pub total_tokens: Option<u32>,
}

impl From<OpenAiResponse> for CompletionResponse {
    fn from(value: OpenAiResponse) -> Self {
        let content = value
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        let finish_reason = value
            .choices
            .first()
            .and_then(|c| c.finish_reason.as_ref())
            .and_then(|r| match r.as_str() {
                "stop" => Some(FinishReason::Stop),
                "length" => Some(FinishReason::Length),
                _ => Some(FinishReason::Error),
            });

        let usage = value.usage.map(|u| Usage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });

        CompletionResponse {
            content,
            finish_reason,
            usage,
        }
    }
}