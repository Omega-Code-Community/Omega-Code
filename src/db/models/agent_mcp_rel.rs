use chrono::{DateTime, Utc};
use rusqlite::{
    Result as RusqliteResult,
    Row,
    ToSql,
};

use super::parse_db_time;
use crate::core::db::Model;

/// Agent 与 MCP 工具关联表（`agent_mcp_rel`）。
///
/// 对应《系统功能设计与数据库设计文档》四.12：多对多关联，声明某个 Agent
/// 被授权可调用哪些 MCP 工具，用于工具调度时的权限校验。
#[derive(Clone, Debug)]
pub struct AgentMcpRel {
    pub id: i32,
    /// Agent ID，关联 `agent_info.id`。
    pub agent_id: i32,
    /// MCP 工具 ID，关联 `mcp_tool.id`。
    pub mcp_tool_id: i32,
    pub create_time: DateTime<Utc>,
    pub update_time: DateTime<Utc>,
    pub remark: Option<String>,
}

impl Model for AgentMcpRel {
    fn table_name() -> &'static str {
        "agent_mcp_rel"
    }

    fn create_table_sql() -> &'static str {
        r#"
        CREATE TABLE IF NOT EXISTS agent_mcp_rel (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            agent_id INTEGER NOT NULL,
            mcp_tool_id INTEGER NOT NULL,
            create_time TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            update_time TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            remark TEXT
        )
        "#
    }

    fn from_row(row: &Row) -> RusqliteResult<Self> {
        let create_time = parse_db_time(&row.get::<_, String>("create_time")?)?;
        let update_time = parse_db_time(&row.get::<_, String>("update_time")?)?;

        Ok(Self {
            id: row.get("id")?,
            agent_id: row.get("agent_id")?,
            mcp_tool_id: row.get("mcp_tool_id")?,
            create_time,
            update_time,
            remark: row.get("remark")?,
        })
    }

    fn insert_columns() -> &'static str {
        "agent_id, mcp_tool_id, remark"
    }

    fn insert_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.agent_id),
            Box::new(self.mcp_tool_id),
            Box::new(self.remark.clone()),
        ]
    }

    fn update_set_clause(&self) -> String {
        r#"
        agent_id = ?,
        mcp_tool_id = ?,
        update_time = CURRENT_TIMESTAMP,
        remark = ?
        "#
            .trim()
            .to_string()
    }

    fn update_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.agent_id),
            Box::new(self.mcp_tool_id),
            Box::new(self.remark.clone()),
        ]
    }

    fn primary_key_value(&self) -> Box<dyn ToSql> {
        Box::new(self.id)
    }
}
