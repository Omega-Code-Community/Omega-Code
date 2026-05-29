use crate::core::providers::claude_messages::request::ClaudeMessage;
use crate::core::providers::types::{Message, Role};

impl From<Message> for ClaudeMessage {
    fn from(value: Message) -> Self {
        Self {
            role: match value.role {
                Role::User => "user",
                Role::Assistant => "assistant",

                Role::System => "user",
            }
                .into(),

            content: value.content,
        }
    }
}