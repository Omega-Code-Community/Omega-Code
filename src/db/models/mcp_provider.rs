use chrono::{DateTime, Utc};
pub struct MCPProvider {
    pub id: i32,
    pub name: String,
    pub url: String,
    pub auth_key: Option<String>,
    pub timeout: i32,// ms default 5000
    pub health_api: Option<String>,
    pub status: i8,// default 1
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: Option<String>,
}