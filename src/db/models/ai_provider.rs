use chrono::{DateTime, Utc};
pub struct AiProvider {
    pub id: i32,
    pub name: String,
    pub avatar: String,
    pub api: String,
    pub key: Option<String>,
    pub timeout: i32,//ms default 5000
    pub status: i8,// default 1
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: Option<String>,
}