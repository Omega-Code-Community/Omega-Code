use chrono::{DateTime, Utc};
use crate::db::r#enum::PromptRole;
pub struct ChatMessage {
    pub id: i32,
    pub chat_uuid: String,
    pub round_uuid: String,
    pub role: PromptRole,
    pub content: String,
    pub input_tokens: i32, // default 0
    pub output_tokens: i32,// default 0
    pub duration: i32,// default 0
    pub model_config: String,
    pub status: i8,// default 1
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: Option<String>,
}