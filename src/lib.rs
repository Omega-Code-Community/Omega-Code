#[macro_use]
extern crate rust_i18n;

use log::{error, info};
use ratatui_kit::prelude::RouterProvider;
use ratatui_kit::{AnyElement, ElementExt, Hooks, component, element};
use rust_i18n::t;

pub mod components;
pub mod core;
pub mod cli;
pub mod db;
pub mod health;
pub mod pages;
pub mod platform;
pub mod router;
pub mod runtime;
pub mod services;
pub mod state;
pub mod utils;

use crate::router::app_routes;

i18n!("locales", fallback = "en");

#[tokio::main]
pub async fn run() -> anyhow::Result<()> {
    use clap::Parser;

    use crate::cli::{Cli, Command};

    info!("{}", t!("logger_is_initialized"));

    let mut cli = Cli::parse();

    // 解析 Workspace 与启动模式，并注入运行环境（优先级：命令行 > 配置文件 > 系统默认）。
    let workspace = cli.workspace()?;
    // Rust 2024：`set_var` 为 unsafe；此处单线程启动阶段修改，安全可控。
    unsafe {
        std::env::set_var("OMEGA_WORKING_DIR", &workspace);
        std::env::set_var("OMEGA_MODE", cli.mode.as_str());
    }
    info!("workspace = {} (mode = {})", workspace.display(), cli.mode.as_str());

    match cli.command.take() {
        // 无子命令：以解析出的 Workspace 启动交互式 TUI。
        None => launch_tui().await,
        // `chat --chat-uuid <uuid>`（无 clear）：续跑指定会话，注入后进入 TUI。
        Some(Command::Chat(chat)) if chat.chat_uuid.is_some() && chat.action.is_none() => {
            let uuid = chat.chat_uuid.clone().unwrap_or_default();
            unsafe {
                std::env::set_var("OMEGA_CHAT_UUID", &uuid);
            }
            info!("resuming chat session = {}", uuid);
            launch_tui().await
        }
        // 其余管理型子命令：执行后直接返回，不进入 TUI。
        Some(cmd) => cli::commands::dispatch(cmd).await,
    }
}

/// 健康检查通过后启动交互式 TUI。
async fn launch_tui() -> anyhow::Result<()> {
    info!("{}", t!("test_message", name = "OmegaCode"));
    info!("{}", t!("current_locale", locale_name = "en"));

    if !health_check() {
        error!("Health check failed");
        return Err(anyhow::anyhow!("Health check failed"));
    }

    // workspace 模式下自动登记当前打开的项目（失败不阻断 TUI 启动）。
    if let Err(e) = crate::core::project::register_opened_from_env() {
        log::warn!("project register failed: {e}");
    }

    element!(Root)
        .into_any()
        .fullscreen()
        .await
        .expect("Failed to run the application");

    Ok(())
}

#[component]
pub fn Root(_hooks: Hooks) -> impl Into<AnyElement<'static>> {
    element!(
        RouterProvider(
            routes: app_routes(),
            index_path: crate::router::initial_route(),
        )
    )
}

fn health_check() -> bool {
    health::network_checker::check_network() && health::db_checker::check_db().unwrap()
}


