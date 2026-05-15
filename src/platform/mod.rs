use anyhow::Result;

pub fn get_platform_user_dir() -> Result<String> {
    #[cfg(target_os = "windows")]
    let user_dir = std::env::var("USERPROFILE")
        .map_err(|e| anyhow::anyhow!("Failed to get USERPROFILE environment variable: {}", e))?;

    #[cfg(target_os = "linux")]
    let user_dir = std::env::var("HOME")
        .map_err(|e| anyhow::anyhow!("Failed to get HOME environment variable: {}", e))?;

    #[cfg(target_os = "macos")]
    let user_dir = std::env::var("HOME")
        .map_err(|e| anyhow::anyhow!("Failed to get HOME environment variable: {}", e))?;

    Ok(user_dir)
}

pub fn get_platform_app_dir() -> Result<String> {
    let user_dir = get_platform_user_dir()?;
    let app_dir = format!("{}/.omega", user_dir);
    std::fs::create_dir_all(&app_dir)
        .map_err(|e| anyhow::anyhow!("Failed to create app directory '{}': {}", app_dir, e))?;
    Ok(app_dir)
}

pub fn get_database_path() -> Result<String> {
    let app_dir = get_platform_app_dir()?;
    let db_path = format!("{}/omega_code.db", app_dir);
    Ok(db_path)
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