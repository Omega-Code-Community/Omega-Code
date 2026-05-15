use chrono::{DateTime, Utc};
pub struct AgentMCPRel {
    pub id: i32,
    pub agent_id: i32,
    pub mcp_tool_id: i32,
    pub status: i8,// default 1
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: Option<String>,
}