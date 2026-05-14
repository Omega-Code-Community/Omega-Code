use chrono::{DateTime, Utc};
pub struct MCPProvider {
    pub id: i32,
    pub name: String,
    pub url: String,
    pub auth_key: String,
    pub timeout: i32,
    pub health_api: String,
    pub status: i8,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: String,
}