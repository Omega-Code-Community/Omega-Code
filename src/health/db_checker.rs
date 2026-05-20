use crate::core::db::DatabaseManager;
use log::error;

pub fn check_db() -> Result<bool, String> {
    let db = DatabaseManager::new()
        .map_err(|e| {
            let err_msg = format!("Failed to create database manager: {}", e);
            error!("{}", err_msg);
            err_msg
        })?;

    db.health_check()
        .map_err(|e| {
            let err_msg = format!("Database health check failed: {}", e);
            error!("{}", err_msg);
            err_msg
        })?;

    Ok(true)
}