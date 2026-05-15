use chrono::{DateTime, Utc};
pub struct ChatSession {
    pub id: i32,
    pub chat_uuid: String,
    pub title: Option<String>,
    pub llm_id: i32,
    pub status: i8,// default 1
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: Option<String>,
}