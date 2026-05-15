use chrono::{DateTime, Utc};
use rusqlite::{
    Result as RusqliteResult,
    Row,
    ToSql,
};

use crate::core::db::Model;

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub id: i32,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub model: Option<String>,
    pub status: i8, // default 1
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: Option<String>,
}

impl Model for ChatMessage {
    fn table_name() -> &'static str {
        "chat_message"
    }

    fn create_table_sql() -> &'static str {
        r#"
        CREATE TABLE IF NOT EXISTS chat_message (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            model TEXT,
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
            session_id: row.get("session_id")?,
            role: row.get("role")?,
            content: row.get("content")?,
            model: row.get("model")?,
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
        "session_id, role, content, model, status, remark"
    }

    fn insert_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.session_id.clone()),
            Box::new(self.role.clone()),
            Box::new(self.content.clone()),
            Box::new(self.model.clone()),
            Box::new(self.status),
            Box::new(self.remark.clone()),
        ]
    }

    fn update_set_clause(&self) -> String {
        r#"
        session_id = ?,
        role = ?,
        content = ?,
        model = ?,
        status = ?,
        updated_at = CURRENT_TIMESTAMP,
        remark = ?
        "#
            .trim()
            .to_string()
    }

    fn update_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.session_id.clone()),
            Box::new(self.role.clone()),
            Box::new(self.content.clone()),
            Box::new(self.model.clone()),
            Box::new(self.status),
            Box::new(self.remark.clone()),
        ]
    }

    fn primary_key_value(&self) -> Box<dyn ToSql> {
        Box::new(self.id)
    }
}
