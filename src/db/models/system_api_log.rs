use chrono::{DateTime, Utc};
use rusqlite::{
    Result as RusqliteResult,
    Row,
    ToSql,
};

use crate::core::db::Model;

#[derive(Clone, Debug)]
pub struct SystemApiLog {
    pub id: i32,
    pub api_path: String,
    pub method: String,
    pub status_code: i32,
    pub response_time: i64,
    pub request_data: String,
    pub response_data: String,
    pub created_at: DateTime<Utc>,
    pub remark: Option<String>,
}

impl Model for SystemApiLog {
    fn table_name() -> &'static str {
        "system_api_log"
    }

    fn create_table_sql() -> &'static str {
        r#"
        CREATE TABLE IF NOT EXISTS system_api_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            api_path TEXT NOT NULL,
            method TEXT NOT NULL,
            status_code INTEGER NOT NULL,
            response_time INTEGER NOT NULL,
            request_data TEXT NOT NULL,
            response_data TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
            remark TEXT
        )
        "#
    }

    fn from_row(row: &Row) -> RusqliteResult<Self> {
        let created_at: String = row.get("created_at")?;

        Ok(Self {
            id: row.get("id")?,
            api_path: row.get("api_path")?,
            method: row.get("method")?,
            status_code: row.get("status_code")?,
            response_time: row.get("response_time")?,
            request_data: row.get("request_data")?,
            response_data: row.get("response_data")?,
            created_at: DateTime::parse_from_rfc3339(&created_at)
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
        "api_path, method, status_code, response_time, request_data, response_data, remark"
    }

    fn insert_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.api_path.clone()),
            Box::new(self.method.clone()),
            Box::new(self.status_code),
            Box::new(self.response_time),
            Box::new(self.request_data.clone()),
            Box::new(self.response_data.clone()),
            Box::new(self.remark.clone()),
        ]
    }

    fn update_set_clause(&self) -> String {
        r#"
        api_path = ?,
        method = ?,
        status_code = ?,
        response_time = ?,
        request_data = ?,
        response_data = ?,
        remark = ?
        "#
            .trim()
            .to_string()
    }

    fn update_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.api_path.clone()),
            Box::new(self.method.clone()),
            Box::new(self.status_code),
            Box::new(self.response_time),
            Box::new(self.request_data.clone()),
            Box::new(self.response_data.clone()),
            Box::new(self.remark.clone()),
        ]
    }

    fn primary_key_value(&self) -> Box<dyn ToSql> {
        Box::new(self.id)
    }
}
