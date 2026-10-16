use chrono::{DateTime, Utc};
use rusqlite::{
    Result as RusqliteResult,
    Row,
    ToSql,
};

use super::parse_db_time;
use crate::core::db::Model;

/// MCP 工具表（`mcp_tool`）。
///
/// 对应《系统功能设计与数据库设计文档》四.11：登记某个 MCP Server 暴露的工具，
/// `tool_code` 为调用标识，`parameters` 描述入参 Schema，`response_format` 描述返回结构。
#[derive(Clone, Debug)]
pub struct McpTool {
    pub id: i32,
    /// 所属 MCP 提供方 ID，关联 `mcp_provider.id`。
    pub provider_id: i32,
    /// 工具展示名称。
    pub name: String,
    /// 工具调用标识（唯一编码）。
    pub tool_code: String,
    /// 工具描述（可空）。
    pub description: Option<String>,
    /// 工具标签/分类（可空）。
    pub tag: Option<String>,
    /// 入参 Schema（JSON，可空）。
    pub parameters: Option<String>,
    /// 返回结构说明（JSON，可空）。
    pub response_format: Option<String>,
    /// 状态：1 启用 / 0 停用。
    pub status: i8,
    pub create_time: DateTime<Utc>,
    pub update_time: DateTime<Utc>,
    pub remark: Option<String>,
}

impl Model for McpTool {
    fn table_name() -> &'static str {
        "mcp_tool"
    }

    fn create_table_sql() -> &'static str {
        r#"
        CREATE TABLE IF NOT EXISTS mcp_tool (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            provider_id INTEGER NOT NULL,
            name TEXT NOT NULL,
            tool_code TEXT NOT NULL,
            description TEXT,
            tag TEXT,
            parameters TEXT,
            response_format TEXT,
            status INTEGER NOT NULL DEFAULT 1,
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
            provider_id: row.get("provider_id")?,
            name: row.get("name")?,
            tool_code: row.get("tool_code")?,
            description: row.get("description")?,
            tag: row.get("tag")?,
            parameters: row.get("parameters")?,
            response_format: row.get("response_format")?,
            status: row.get("status")?,
            create_time,
            update_time,
            remark: row.get("remark")?,
        })
    }

    fn insert_columns() -> &'static str {
        "provider_id, name, tool_code, description, tag, parameters, response_format, status, remark"
    }

    fn insert_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.provider_id),
            Box::new(self.name.clone()),
            Box::new(self.tool_code.clone()),
            Box::new(self.description.clone()),
            Box::new(self.tag.clone()),
            Box::new(self.parameters.clone()),
            Box::new(self.response_format.clone()),
            Box::new(self.status),
            Box::new(self.remark.clone()),
        ]
    }

    fn update_set_clause(&self) -> String {
        r#"
        provider_id = ?,
        name = ?,
        tool_code = ?,
        description = ?,
        tag = ?,
        parameters = ?,
        response_format = ?,
        status = ?,
        update_time = CURRENT_TIMESTAMP,
        remark = ?
        "#
            .trim()
            .to_string()
    }

    fn update_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.provider_id),
            Box::new(self.name.clone()),
            Box::new(self.tool_code.clone()),
            Box::new(self.description.clone()),
            Box::new(self.tag.clone()),
            Box::new(self.parameters.clone()),
            Box::new(self.response_format.clone()),
            Box::new(self.status),
            Box::new(self.remark.clone()),
        ]
    }

    fn primary_key_value(&self) -> Box<dyn ToSql> {
        Box::new(self.id)
    }
}
