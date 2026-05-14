use chrono::{DateTime, Utc};
pub struct MCPTool {
    pub id: i32,
    pub provider_id: i32,
    pub name: String,
    pub tool_code: String,
    pub description: String,
    pub tag: String,
    pub parameters: String,
    pub response_format: String,
    pub status: i8,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: String,
}