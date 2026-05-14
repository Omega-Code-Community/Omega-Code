use chrono::{DateTime, Utc};
pub struct AiProvider {
    pub id: i32,
    pub name: String,
    pub avatar: String,
    pub api: String,
    pub key: String,
    pub timeout: i32,
    pub status: i8,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: String,
}