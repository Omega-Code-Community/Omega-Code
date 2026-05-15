use chrono::{DateTime, Utc};
use rusqlite::{
    Result as RusqliteResult,
    Row,
    ToSql,
};

use crate::core::db::Model;

#[derive(Clone, Debug)]
pub struct McpTool {
    pub id: i32,
    pub provider_id: i32,
    pub name: String,
    pub tool_code: String,
    pub tool_type: String,
    pub description: String,
    pub input_schema: String,
    pub status: i8, // default 1
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
            tool_type TEXT NOT NULL,
            description TEXT NOT NULL,
            input_schema TEXT NOT NULL,
            status INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            remark TEXT
        )
        "#
    }

    fn from_row(row: &Row) -> RusqliteResult<Self> {
        let created_at: String = row.get("created_at")?;
        let updated_at: String = row.get("updated_at")?;

        Ok(Self {
            id: row.get("id")?,
            provider_id: row.get("provider_id")?,
            name: row.get("name")?,
            tool_code: row.get("tool_code")?,
            tool_type: row.get("tool_type")?,
            description: row.get("description")?,
            input_schema: row.get("input_schema")?,
            status: row.get("status")?,
            created_at: DateTime::parse_from_rfc3339(&created_at)
                .map(|v| v.with_timezone(&Utc))
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?,
            updated_at: DateTime::parse_from_rfc3339(&updated_at)
                .map(|v| v.with_timezone(&Utc))
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?,
            remark: row.get("remark")?,
        })
    }

    fn insert_columns() -> &'static str {
        "provider_id, name, tool_code, tool_type, description, input_schema, status, remark"
    }

    fn insert_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.provider_id),
            Box::new(self.name.clone()),
            Box::new(self.tool_code.clone()),
            Box::new(self.tool_type.clone()),
            Box::new(self.description.clone()),
            Box::new(self.input_schema.clone()),
            Box::new(self.status),
            Box::new(self.remark.clone()),
        ]
    }

    fn update_set_clause(&self) -> String {
        r#"
        provider_id = ?,
        name = ?,
        tool_code = ?,
        tool_type = ?,
        description = ?,
        input_schema = ?,
        status = ?,
        updated_at = CURRENT_TIMESTAMP,
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
            Box::new(self.tool_type.clone()),
            Box::new(self.description.clone()),
            Box::new(self.input_schema.clone()),
            Box::new(self.status),
            Box::new(self.remark.clone()),
        ]
    }

    fn primary_key_value(&self) -> Box<dyn ToSql> {
        Box::new(self.id)
    }
}
