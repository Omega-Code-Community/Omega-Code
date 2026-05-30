use serde::Deserialize;
use crate::core::providers::types::{CompletionResponse, FinishReason, Usage};

#[derive(Deserialize)]
pub struct ClaudeResponse {
    pub content: Vec<ClaudeContent>,
    pub stop_reason: Option<String>,
    pub usage: Option<ClaudeUsage>,
}

#[derive(Deserialize)]
pub struct ClaudeContent {
    pub text: String,
}

#[derive(Deserialize)]
pub struct ClaudeUsage {
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}

impl From<ClaudeResponse> for CompletionResponse {
    fn from(value: ClaudeResponse) -> Self {
        let content = value
            .content
            .first()
            .map(|c| c.text.clone())
            .unwrap_or_default();

        let finish_reason = value.stop_reason.and_then(|r| match r.as_str() {
            "end_turn" => Some(FinishReason::Stop),
            "max_tokens" => Some(FinishReason::Length),
            _ => Some(FinishReason::Error),
        });

        let usage = value.usage.map(|u| Usage {
            prompt_tokens: u.input_tokens,
            completion_tokens: u.output_tokens,
            total_tokens: None,
        });

        CompletionResponse {
            content,
            finish_reason,
            usage,
        }
    }
}