use chrono::{DateTime, Utc};
use rusqlite::{
    Result as RusqliteResult,
    Row,
    ToSql,
};

use crate::core::db::Model;

#[derive(Clone, Debug)]
pub struct AgentInfo {
    pub id: i32,
    pub name: String,
    pub avatar: String,
    pub model_id: i32,
    pub prompt_id: i32,
    pub status: i8, // default 1
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: Option<String>,
}

impl Model for AgentInfo {
    fn table_name() -> &'static str {
        "agent_info"
    }

    fn create_table_sql() -> &'static str {
        r#"
        CREATE TABLE IF NOT EXISTS agent_info (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            avatar TEXT NOT NULL,
            model_id INTEGER NOT NULL,
            prompt_id INTEGER NOT NULL,
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
            avatar: row.get("avatar")?,
            model_id: row.get("model_id")?,
            prompt_id: row.get("prompt_id")?,
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
        "name, avatar, model_id, prompt_id, status, remark"
    }

    fn insert_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.name.clone()),
            Box::new(self.avatar.clone()),
            Box::new(self.model_id),
            Box::new(self.prompt_id),
            Box::new(self.status),
            Box::new(self.remark.clone()),
        ]
    }

    fn update_set_clause(&self) -> String {
        r#"
        name = ?,
        avatar = ?,
        model_id = ?,
        prompt_id = ?,
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
            Box::new(self.avatar.clone()),
            Box::new(self.model_id),
            Box::new(self.prompt_id),
            Box::new(self.status),
            Box::new(self.remark.clone()),
        ]
    }

    fn primary_key_value(&self) -> Box<dyn ToSql> {
        Box::new(self.id)
    }
}
