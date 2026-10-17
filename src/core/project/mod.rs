//! 项目信息自动登记
//!
//! 对应《系统功能设计与数据库设计文档》四.13 `project_info`：以工作区绝对路径 `path`
//! 唯一标识一个项目。CLI-First 下不额外提供命令，改由运行时在 **workspace 模式** 启动
//! TUI 时自动登记当前打开的项目（见 `src/lib.rs::launch_tui`）：
//! - 首次打开：写入 `name`（目录名）+ `path` + `create_at`/`updata_at`；
//! - 重复打开：仅刷新 `updata_at` 并复活 `is_deleted`，`id` 保持稳定。
//!
//! `create_at`/`updata_at` 为项目业务时间，统一用 `%Y-%m-%d %H:%M:%S`（`parse_db_time` 可解析）。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;

use crate::core::db::DatabaseManager;
use crate::db::models::project_info::ProjectInfo;

/// 生成业务时间字符串（`%Y-%m-%d %H:%M:%S`，UTC）。
fn now_str() -> String {
    Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 登记（或刷新）一个项目，返回其 `project_info.id`。
///
/// 以规整后的绝对路径为业务唯一键：命中既有记录则刷新 `updata_at` 并复活 `is_deleted`；
/// 否则新建记录（`name` 取路径末段目录名，缺失时回退整路径串）。
pub fn register(db: &DatabaseManager, path: &Path) -> Result<i32> {
    let abs = normalize(path)?;
    let path_str = abs.to_string_lossy().to_string();
    let name = abs
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| path_str.clone());

    let now = now_str();

    let existing = db.find_where::<ProjectInfo>("path = ?1", &[&path_str])?;
    if let Some(mut row) = existing.into_iter().next() {
        // 重复打开：刷新业务更新时间并复活逻辑删除标记。
        let id = row.id;
        row.updata_at = Some(now);
        row.is_deleted = 0;
        db.update(row)?;
        return Ok(id);
    }

    let row = ProjectInfo {
        id: 0,
        name,
        path: path_str,
        create_at: Some(now.clone()),
        updata_at: Some(now),
        is_deleted: 0,
        create_time: Utc::now(),
        update_time: Utc::now(),
        remark: None,
    };
    let id = db.create(row)?;
    Ok(id as i32)
}

/// 当运行于 workspace 模式时，登记当前打开的工作区。
///
/// 仅在 `OMEGA_MODE == "workspace"` 时实际写库，工作区路径取自 `OMEGA_WORKING_DIR`
/// （回退进程当前目录）；非 workspace 模式返回 `Ok(None)`，不做任何登记。
pub fn register_opened_from_env() -> Result<Option<i32>> {
    let mode = std::env::var("OMEGA_MODE").unwrap_or_default();
    if mode != "workspace" {
        return Ok(None);
    }

    let path = match std::env::var("OMEGA_WORKING_DIR") {
        Ok(dir) if !dir.trim().is_empty() => PathBuf::from(dir),
        _ => std::env::current_dir().context("failed to resolve working dir")?,
    };

    let db = DatabaseManager::new()?;
    register(&db, &path).map(Some)
}

/// 列出全部项目记录。
pub fn list(db: &DatabaseManager) -> Result<Vec<ProjectInfo>> {
    db.read_all::<ProjectInfo>()
}

/// 物理删除一个项目记录，返回受影响行数。
pub fn remove(db: &DatabaseManager, id: i32) -> Result<usize> {
    db.delete::<ProjectInfo>(id)
}

/// 规整为绝对路径（不要求路径存在，仅做规范化）。
fn normalize(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        let base = std::env::current_dir().context("failed to resolve current dir")?;
        Ok(base.join(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::db::Model;

    /// 构造含 `project_info` 表的内存库。
    fn test_db() -> DatabaseManager {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute_batch(ProjectInfo::create_table_sql()).unwrap();
        DatabaseManager { conn }
    }

    #[test]
    fn register_creates_with_dir_name() {
        let db = test_db();
        let path = if cfg!(windows) {
            Path::new("C:\\workspace\\demo")
        } else {
            Path::new("/workspace/demo")
        };

        let id = register(&db, path).unwrap();
        assert!(id > 0);

        let row = db.read_by_id::<ProjectInfo>(id).unwrap().unwrap();
        assert_eq!(row.name, "demo");
        assert!(row.create_at.is_some());
        assert!(row.updata_at.is_some());
        assert_eq!(row.is_deleted, 0);
    }

    #[test]
    fn register_same_path_is_stable() {
        let db = test_db();
        let path = if cfg!(windows) {
            Path::new("C:\\workspace\\demo")
        } else {
            Path::new("/workspace/demo")
        };

        let first = register(&db, path).unwrap();
        let second = register(&db, path).unwrap();
        assert_eq!(first, second);

        let rows = db.read_all::<ProjectInfo>().unwrap();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn register_revives_deleted() {
        let db = test_db();
        let path = if cfg!(windows) {
            Path::new("C:\\workspace\\demo")
        } else {
            Path::new("/workspace/demo")
        };

        let id = register(&db, path).unwrap();
        // 手动置为已删除，再登记应复活。
        let mut row = db.read_by_id::<ProjectInfo>(id).unwrap().unwrap();
        row.is_deleted = 1;
        db.update(row).unwrap();

        register(&db, path).unwrap();
        let row = db.read_by_id::<ProjectInfo>(id).unwrap().unwrap();
        assert_eq!(row.is_deleted, 0);
    }

    #[test]
    fn list_counts_and_remove_deletes() {
        let db = test_db();
        let p1 = if cfg!(windows) {
            Path::new("C:\\workspace\\a")
        } else {
            Path::new("/workspace/a")
        };
        let p2 = if cfg!(windows) {
            Path::new("C:\\workspace\\b")
        } else {
            Path::new("/workspace/b")
        };

        let id1 = register(&db, p1).unwrap();
        register(&db, p2).unwrap();
        assert_eq!(list(&db).unwrap().len(), 2);

        assert_eq!(remove(&db, id1).unwrap(), 1);
        assert_eq!(list(&db).unwrap().len(), 1);
        assert!(db.read_by_id::<ProjectInfo>(id1).unwrap().is_none());
    }
}
