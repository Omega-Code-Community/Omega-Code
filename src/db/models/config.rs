use chrono::{DateTime, Utc};
pub struct Config {
    pub key: String,
    pub value: String,
    pub description: String,
    pub key_type: String,
    pub tag: String,
    pub sort: i32,// default 0
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: Option<String>,
}
