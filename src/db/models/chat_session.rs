use chrono::{DateTime, Utc};
pub struct ChatSession {
    pub id: i32,
    pub chat_uuid: String,
    pub title: String,
    pub llm_id: i32,
    pub status: i8,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: String,
}