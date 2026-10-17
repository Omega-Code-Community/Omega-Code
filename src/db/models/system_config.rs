use chrono::{DateTime, Utc};
use rusqlite::{
    Result as RusqliteResult,
    Row,
    ToSql,
};

use super::parse_db_time;
use crate::core::db::Model;

/// 系统配置表（`system_config`）。
///
/// 对应《系统功能设计与数据库设计文档》四.15：以字符串 `key` 为主键的键值配置，
/// 支持类型标注、分组标签与排序，供 CLI/Control Plane 读取系统级默认配置。
#[derive(Clone, Debug)]
pub struct SystemConfig {
    /// 配置键（主键）。
    pub key: String,
    /// 配置值。
    pub value: String,
    /// 值类型标注（string/int/bool/json 等，可空）。
    pub key_type: Option<String>,
    /// 配置说明（可空）。
    pub description: Option<String>,
    /// 分组标签（可空）。
    pub tag: Option<String>,
    /// 排序序号。
    pub sort: i32,
    pub create_time: DateTime<Utc>,
    pub update_time: DateTime<Utc>,
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
            key_type TEXT,
            description TEXT,
            tag TEXT,
            sort INTEGER NOT NULL DEFAULT 0,
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
            key: row.get("key")?,
            value: row.get("value")?,
            key_type: row.get("key_type")?,
            description: row.get("description")?,
            tag: row.get("tag")?,
            sort: row.get("sort")?,
            create_time,
            update_time,
            remark: row.get("remark")?,
        })
    }

    fn insert_columns() -> &'static str {
        "key, value, key_type, description, tag, sort, remark"
    }

    fn insert_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.key.clone()),
            Box::new(self.value.clone()),
            Box::new(self.key_type.clone()),
            Box::new(self.description.clone()),
            Box::new(self.tag.clone()),
            Box::new(self.sort),
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
        update_time = CURRENT_TIMESTAMP,
        remark = ?
        "#
            .trim()
            .to_string()
    }

    fn update_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.value.clone()),
            Box::new(self.key_type.clone()),
            Box::new(self.description.clone()),
            Box::new(self.tag.clone()),
            Box::new(self.sort),
            Box::new(self.remark.clone()),
        ]
    }

    fn primary_key_column() -> &'static str {
        "key"
    }

    fn primary_key_value(&self) -> Box<dyn ToSql> {
        Box::new(self.key.clone())
    }
}
