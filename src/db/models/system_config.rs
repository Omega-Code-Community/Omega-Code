use chrono::{DateTime, Utc};
pub struct SystemConfig {
    pub key: i32,
    pub value: String,
    pub key_type: String,
    pub description: String,
    pub tag: String,
    pub sort: i32,
    pub status: i8,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: String,
}