use crate::core::providers::gemini::request::{GeminiContent, GeminiPart};
use crate::core::providers::types::{Message, Role};

impl From<Message> for GeminiContent {
    fn from(value: Message) -> Self {
        Self {
            role: match value.role {
                Role::System => "user",
                Role::User => "user",
                Role::Assistant => "model",
            }
                .into(),

            parts: vec![GeminiPart {
                text: value.content,
            }],
        }
    }
}