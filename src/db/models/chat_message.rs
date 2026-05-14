use chrono::{DateTime, Utc};
use crate::db::r#enum::PromptRole;
pub struct ChatMessage {
    pub id: i32,
    pub chat_uuid: String,
    pub round_uuid: String,
    pub role: PromptRole,
    pub content: String,
    pub input_tokens: i32,
    pub output_tokens: i32,
    pub duration: i32,
    pub model_config: String,
    pub status: i8,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: String,
}