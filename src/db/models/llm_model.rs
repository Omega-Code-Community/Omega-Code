use chrono::{DateTime, Utc};
use rusqlite::{
    Result as RusqliteResult,
    Row,
    ToSql,
};

use crate::core::db::Model;

#[derive(Clone, Debug)]
pub struct LLMModel {
    pub id: i32,
    pub provider_id: i32,
    pub name: Option<String>,
    pub avatar: String,
    pub model_code: String,
    pub max_tokens: i32, // default 0
    pub max_response_tokens: i32, // default 0
    pub weight: i32, // default 0
    pub call_count: i64, // default 0
    pub status: i8, // default 1
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: Option<String>,
}

impl Model for LLMModel {
    fn table_name() -> &'static str {
        "llm_model"
    }

    fn create_table_sql() -> &'static str {
        r#"
        CREATE TABLE IF NOT EXISTS llm_model (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            provider_id INTEGER NOT NULL,
            name TEXT,
            avatar TEXT NOT NULL,
            model_code TEXT NOT NULL,
            max_tokens INTEGER NOT NULL DEFAULT 0,
            max_response_tokens INTEGER NOT NULL DEFAULT 0,
            weight INTEGER NOT NULL DEFAULT 0,
            call_count INTEGER NOT NULL DEFAULT 0,
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
            avatar: row.get("avatar")?,
            model_code: row.get("model_code")?,
            max_tokens: row.get("max_tokens")?,
            max_response_tokens: row.get("max_response_tokens")?,
            weight: row.get("weight")?,
            call_count: row.get("call_count")?,
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
        "provider_id, name, avatar, model_code, max_tokens, max_response_tokens, weight, call_count, status, remark"
    }

    fn insert_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.provider_id),
            Box::new(self.name.clone()),
            Box::new(self.avatar.clone()),
            Box::new(self.model_code.clone()),
            Box::new(self.max_tokens),
            Box::new(self.max_response_tokens),
            Box::new(self.weight),
            Box::new(self.call_count),
            Box::new(self.status),
            Box::new(self.remark.clone()),
        ]
    }

    fn update_set_clause(&self) -> String {
        r#"
        provider_id = ?,
        name = ?,
        avatar = ?,
        model_code = ?,
        max_tokens = ?,
        max_response_tokens = ?,
        weight = ?,
        call_count = ?,
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
            Box::new(self.avatar.clone()),
            Box::new(self.model_code.clone()),
            Box::new(self.max_tokens),
            Box::new(self.max_response_tokens),
            Box::new(self.weight),
            Box::new(self.call_count),
            Box::new(self.status),
            Box::new(self.remark.clone()),
        ]
    }

    fn primary_key_value(&self) -> Box<dyn ToSql> {
        Box::new(self.id)
    }
}
