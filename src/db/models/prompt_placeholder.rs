use chrono::{DateTime, Utc};
pub struct PromptPlaceholder {
    pub id: i32,
    pub prompt_id: i32,
    pub key: String,
    pub label: String,
    pub value: String,
    pub default: String,
    pub input_type: String,
    pub required: i8, // default 1
    pub status: i8,// default 1
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: Option<String>,
}