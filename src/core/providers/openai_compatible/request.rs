use serde::Serialize;
use crate::core::providers::Role;
use crate::core::providers::types::{CompletionRequest, Message};

#[derive(Serialize)]
pub struct OpenAiRequest {
    pub model: String,
    pub messages: Vec<OpenAiMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    pub stream: bool,
}

#[derive(Serialize)]
pub struct OpenAiMessage {
    pub role: String,
    pub content: String,
}

impl OpenAiRequest {
    pub fn from_completion_request(request: CompletionRequest) -> Self {
        Self {
            model: request.model,
            messages: request.messages.into_iter().map(OpenAiMessage::from).collect(),
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            stream: request.stream,
        }
    }
}

impl From<Message> for OpenAiMessage {
    fn from(value: Message) -> Self {
        Self {
            role: match value.role {
                Role::System => "system".to_string(),
                Role::User => "user".to_string(),
                Role::Assistant => "assistant".to_string(),
            },
            content: value.content,
        }
    }
}