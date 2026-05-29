use crate::core::providers::openai_compatible::request::OpenAiMessage;
use crate::core::providers::types::{Message, Role};

impl From<Message> for OpenAiMessage {
    fn from(value: Message) -> Self {
        Self {
            role: match value.role {
                Role::System => "system",
                Role::User => "user",
                Role::Assistant => "assistant",
            }
                .into(),
            content: value.content,
        }
    }
}