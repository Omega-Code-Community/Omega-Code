use chrono::{DateTime, Utc};
use crate::db::r#enum::PromptRole;
pub struct PromptInfo {
    pub id: i32,
    pub name: String,
    pub content: String,
    pub category: String,
    pub role: PromptRole,
    pub version: String,
    pub version_desc: String,
    pub tag: String,
    pub status: i8,// default 1
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: Option<String>,
}