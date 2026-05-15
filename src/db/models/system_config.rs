use chrono::{DateTime, Utc};
pub struct SystemConfig {
    pub key: i32,
    pub value: String,
    pub key_type: String,
    pub description: String,
    pub tag: String,
    pub sort: i32, // default 0
    pub status: i8,// default 1
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: Option<String>,
}