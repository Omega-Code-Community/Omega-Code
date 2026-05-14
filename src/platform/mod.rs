pub fn get_platform_user_dir() -> String {
    #[cfg(target_os = "windows")]
    let user_dir = std::env::var("USERPROFILE").unwrap_or_default();

    #[cfg(target_os = "linux")]
    let user_dir = std::env::var("HOME").unwrap_or_default();

    #[cfg(target_os = "macos")]
    let user_dir = std::env::var("HOME").unwrap_or_default();

    user_dir
}

pub fn get_platform_app_dir() -> String {
    let user_dir = get_platform_user_dir();
    let app_dir = format!("{}/.OmegaCode", user_dir);
    std::fs::create_dir_all(&app_dir).unwrap();
    app_dir
}

#[macro_export]
macro_rules! app_dir {
    () => {
        $crate::platform::get_platform_app_dir()
    };
}