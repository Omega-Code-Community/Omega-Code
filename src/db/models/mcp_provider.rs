use chrono::{DateTime, Utc};
use rusqlite::{
    Result as RusqliteResult,
    Row,
    ToSql,
};

use crate::core::db::Model;

#[derive(Clone, Debug)]
pub struct McpProvider {
    pub id: i32,
    pub name: String,
    pub url: String,
    pub auth_key: Option<String>,
    pub timeout: i32, // ms default 5000
    pub health_api: Option<String>,
    pub status: i8, // default 1
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: Option<String>,
}

impl Model for McpProvider {
    fn table_name() -> &'static str {
        "mcp_provider"
    }

    fn create_table_sql() -> &'static str {
        r#"
        CREATE TABLE IF NOT EXISTS mcp_provider (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            url TEXT NOT NULL,
            auth_key TEXT,
            timeout INTEGER NOT NULL DEFAULT 5000,
            health_api TEXT,
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
            name: row.get("name")?,
            url: row.get("url")?,
            auth_key: row.get("auth_key")?,
            timeout: row.get("timeout")?,
            health_api: row.get("health_api")?,
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
        "name, url, auth_key, timeout, health_api, status, remark"
    }

    fn insert_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.name.clone()),
            Box::new(self.url.clone()),
            Box::new(self.auth_key.clone()),
            Box::new(self.timeout),
            Box::new(self.health_api.clone()),
            Box::new(self.status),
            Box::new(self.remark.clone()),
        ]
    }

    fn update_set_clause(&self) -> String {
        r#"
        name = ?,
        url = ?,
        auth_key = ?,
        timeout = ?,
        health_api = ?,
        status = ?,
        updated_at = CURRENT_TIMESTAMP,
        remark = ?
        "#
            .trim()
            .to_string()
    }

    fn update_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.name.clone()),
            Box::new(self.url.clone()),
            Box::new(self.auth_key.clone()),
            Box::new(self.timeout),
            Box::new(self.health_api.clone()),
            Box::new(self.status),
            Box::new(self.remark.clone()),
        ]
    }

    fn primary_key_value(&self) -> Box<dyn ToSql> {
        Box::new(self.id)
    }
}
