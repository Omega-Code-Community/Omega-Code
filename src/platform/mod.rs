pub mod detect;
use anyhow::{anyhow, Result};
use std::path::PathBuf;

/// 获取用户目录
pub fn get_platform_user_dir() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    let user_dir = std::env::var("USERPROFILE")
        .map_err(|e| anyhow!("Failed to get USERPROFILE: {}", e))?;

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let user_dir = std::env::var("HOME")
        .map_err(|e| anyhow!("Failed to get HOME: {}", e))?;

    Ok(PathBuf::from(user_dir))
}

/// 获取系统 temp 目录
pub fn get_os_temp_dir() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let temp_dir = std::env::var("TEMP")
            .map_err(|e| anyhow!("Failed to get TEMP: {}", e))?;

        Ok(PathBuf::from(temp_dir))
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        Ok(PathBuf::from("/tmp"))
    }

    #[cfg(not(any(
        target_os = "windows",
        target_os = "linux",
        target_os = "macos"
    )))]
    {
        Err(anyhow!("Unsupported operating system"))
    }
}

/// 获取应用目录
pub fn get_platform_app_dir() -> Result<PathBuf> {
    let app_dir = get_platform_user_dir()?.join(".omega");

    std::fs::create_dir_all(&app_dir).map_err(|e| {
        anyhow!(
            "Failed to create app directory '{}': {}",
            app_dir.display(),
            e
        )
    })?;

    Ok(app_dir)
}

/// 获取 runtime 目录
pub fn get_app_runtime_dir() -> Result<PathBuf> {
    let runtime_dir = get_platform_app_dir()?.join("runtime");

    std::fs::create_dir_all(&runtime_dir).map_err(|e| {
        anyhow!(
            "Failed to create runtime directory '{}': {}",
            runtime_dir.display(),
            e
        )
    })?;

    Ok(runtime_dir)
}

/// 获取数据库路径
pub fn get_database_path() -> Result<PathBuf> {
    Ok(
        get_platform_app_dir()?
            .join("omega_code.db")
    )
}

#[macro_export]
macro_rules! app_dir {
    () => {{
        match $crate::platform::get_platform_app_dir() {
            Ok(dir) => dir,
            Err(e) => {
                eprintln!("Failed to get app directory: {}", e);
                std::process::exit(1);
            }
        }
    }};
}