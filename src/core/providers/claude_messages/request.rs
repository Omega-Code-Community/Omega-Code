use serde::Serialize;
use crate::core::providers::types::{CompletionRequest, Message, Role};

#[derive(Serialize)]
pub struct ClaudeRequest {
    pub model: String,
    pub max_tokens: u32,
    pub messages: Vec<ClaudeMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    pub stream: bool,
}

#[derive(Serialize)]
pub struct ClaudeMessage {
    pub role: String,
    pub content: String,
}

impl ClaudeRequest {
    pub fn from_completion_request(request: CompletionRequest) -> Self {
        let mut system_prompt = None;
        let mut messages = Vec::new();
        
        for msg in request.messages {
            if msg.role == Role::System {
                system_prompt = Some(msg.content);
            } else {
                messages.push(ClaudeMessage::from(msg));
            }
        }
        
        Self {
            model: request.model,
            max_tokens: request.max_tokens.unwrap_or(4096),
            messages,
            system: system_prompt,
            temperature: request.temperature,
            stream: request.stream,
        }
    }
}

impl From<Message> for ClaudeMessage {
    fn from(value: Message) -> Self {
        Self {
            role: match value.role {
                Role::User => "user".to_string(),
                Role::Assistant => "assistant".to_string(),
                Role::System => "user".to_string(),
            },
            content: value.content,
        }
    }
}