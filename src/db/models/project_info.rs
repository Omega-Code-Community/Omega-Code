use chrono::{DateTime, Utc};
use rusqlite::{
    Result as RusqliteResult,
    Row,
    ToSql,
};

use super::parse_db_time;
use crate::core::db::Model;

/// 项目信息表（`project_info`）。
///
/// 对应《系统功能设计与数据库设计文档》四.13：以工作区绝对路径 `path` 唯一标识一个项目，
/// 记录项目自身的时间信息（`create_at`/`updata_at`，字段名沿用设计文档拼写）与逻辑删除标记。
/// 注意：`create_at`/`updata_at` 为项目业务时间，与审计字段 `create_time`/`update_time` 并存。
#[derive(Clone, Debug)]
pub struct ProjectInfo {
    pub id: i32,
    /// 项目名称。
    pub name: String,
    /// 项目工作区绝对路径（唯一）。
    pub path: String,
    /// 项目创建时间（业务字段，字符串原样存储）。
    pub create_at: Option<String>,
    /// 项目更新时间（业务字段，字符串原样存储；字段名沿用设计文档拼写）。
    pub updata_at: Option<String>,
    /// 逻辑删除标记：0 正常 / 1 已删除。
    pub is_deleted: i8,
    pub create_time: DateTime<Utc>,
    pub update_time: DateTime<Utc>,
    pub remark: Option<String>,
}

impl Model for ProjectInfo {
    fn table_name() -> &'static str {
        "project_info"
    }

    fn create_table_sql() -> &'static str {
        r#"
        CREATE TABLE IF NOT EXISTS project_info (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            path TEXT NOT NULL,
            create_at TEXT,
            updata_at TEXT,
            is_deleted INTEGER NOT NULL DEFAULT 0,
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
            name: row.get("name")?,
            path: row.get("path")?,
            create_at: row.get("create_at")?,
            updata_at: row.get("updata_at")?,
            is_deleted: row.get("is_deleted")?,
            create_time,
            update_time,
            remark: row.get("remark")?,
        })
    }

    fn insert_columns() -> &'static str {
        "name, path, create_at, updata_at, is_deleted, remark"
    }

    fn insert_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.name.clone()),
            Box::new(self.path.clone()),
            Box::new(self.create_at.clone()),
            Box::new(self.updata_at.clone()),
            Box::new(self.is_deleted),
            Box::new(self.remark.clone()),
        ]
    }

    fn update_set_clause(&self) -> String {
        r#"
        name = ?,
        path = ?,
        create_at = ?,
        updata_at = ?,
        is_deleted = ?,
        update_time = CURRENT_TIMESTAMP,
        remark = ?
        "#
            .trim()
            .to_string()
    }

    fn update_values(&self) -> Vec<Box<dyn ToSql>> {
        vec![
            Box::new(self.name.clone()),
            Box::new(self.path.clone()),
            Box::new(self.create_at.clone()),
            Box::new(self.updata_at.clone()),
            Box::new(self.is_deleted),
            Box::new(self.remark.clone()),
        ]
    }

    fn primary_key_value(&self) -> Box<dyn ToSql> {
        Box::new(self.id)
    }
}
