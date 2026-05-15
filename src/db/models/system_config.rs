use chrono::{DateTime, NaiveDateTime, Utc};
use rusqlite::{
    Result as RusqliteResult,
    Row,
    ToSql,
};

use crate::core::db::Model;

#[derive(Clone, Debug)]
pub struct SystemConfig {
    pub key: String,
    pub value: String,
    pub key_type: String,
    pub description: String,
    pub tag: String,
    pub sort: i32,
    pub status: i8,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub remark: Option<String>,
}

impl Model for SystemConfig {
    fn table_name() -> &'static str {
        "system_config"
    }

    fn create_table_sql() -> &'static str {
        r#"
        CREATE TABLE IF NOT EXISTS system_config (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            key_type TEXT NOT NULL,
            description TEXT NOT NULL,
            tag TEXT NOT NULL,
            sort INTEGER NOT NULL DEFAULT 0,
            status INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            remark TEXT
        )
        "#
    }

    fn from_row(row: &Row) -> RusqliteResult<Self> {
        let created_at_str: String =
            row.get("created_at")?;

        let updated_at_str: String =
            row.get("updated_at")?;

        let created_at =
            NaiveDateTime::parse_from_str(
                &created_at_str,
                "%Y-%m-%d %H:%M:%S",
            )
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;

        let updated_at =
            NaiveDateTime::parse_from_str(
                &updated_at_str,
                "%Y-%m-%d %H:%M:%S",
            )
                .map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;

        Ok(Self {
            key: row.get("key")?,

            value: row.get("value")?,

            key_type: row.get("key_type")?,

            description: row.get("description")?,

            tag: row.get("tag")?,

            sort: row.get("sort")?,

            status: row.get("status")?,

            created_at:
            DateTime::<Utc>::from_naive_utc_and_offset(
                created_at,
                Utc,
            ),

            updated_at:
            DateTime::<Utc>::from_naive_utc_and_offset(
                updated_at,
                Utc,
            ),

            remark: row.get("remark")?,
        })
    }

    fn insert_columns() -> &'static str {
        "key, value, key_type, description, tag, sort, status, remark"
    }

    fn insert_values(
        &self,
    ) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.key.clone()),
            Box::new(self.value.clone()),
            Box::new(self.key_type.clone()),
            Box::new(self.description.clone()),
            Box::new(self.tag.clone()),
            Box::new(self.sort),
            Box::new(self.status),
            Box::new(self.remark.clone()),
        ]
    }

    fn update_set_clause(&self) -> String {
        r#"
        value = ?,
        key_type = ?,
        description = ?,
        tag = ?,
        sort = ?,
        status = ?,
        updated_at = CURRENT_TIMESTAMP,
        remark = ?
        "#
            .trim()
            .to_string()
    }

    fn update_values(
        &self,
    ) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.value.clone()),
            Box::new(self.key_type.clone()),
            Box::new(self.description.clone()),
            Box::new(self.tag.clone()),
            Box::new(self.sort),
            Box::new(self.status),
            Box::new(self.remark.clone()),
        ]
    }

    fn primary_key_column() -> &'static str {
        "key"
    }

    fn primary_key_value(
        &self,
    ) -> Box<dyn ToSql> {
        Box::new(self.key.clone())
    }
}