use ratatui_kit::prelude::*;
use crate::pages::chat::ChatPage;
use crate::pages::welcome::WelcomePage;
use crate::pages::router::RouterPage;

pub fn app_routes() -> Vec<Route> {
    let routes = routes!{
        "/welcome" => WelcomePage,
        "/chat" => ChatPage,
        "/router" => RouterPage,
    };
    routes
}

/// 是否为开发环境：debug 构建，或显式设置 `OMEGA_DEV` 环境变量。
///
/// 开发环境下入口路由指向 `/router`（手动 1/2/3 调试页）；生产环境永不进入
/// `/router`，满足《系统功能设计》「路由调试页不对生产环境暴露」的约束。
pub fn is_dev() -> bool {
    cfg!(debug_assertions) || std::env::var("OMEGA_DEV").is_ok()
}

/// 应用是否已完成初始化配置：以 `system_config.default_agent_id` 是否已写入为准
/// （由 CLI `agent use` 设置）。数据库不可用或未配置均视为「未初始化」。
///
/// 与 chat 页 `init_agent` 的纯数据库驱动配置来源保持一致：无默认 Agent 时进入
/// 欢迎页引导，已配置时直接进入聊天页。
pub fn is_configured() -> bool {
    use crate::core::db::DatabaseManager;
    use crate::db::models::system_config::SystemConfig;

    let Ok(db) = DatabaseManager::new() else {
        return false;
    };
    db.find_where::<SystemConfig>("key = ?1", &[&"default_agent_id"])
        .ok()
        .and_then(|rows| rows.into_iter().next())
        .map(|c| c.value.trim().parse::<i32>().is_ok())
        .unwrap_or(false)
}

/// 依据「是否开发环境 / 是否已配置」决定入口路由（纯函数，可测）。
pub fn choose_initial_route(dev: bool, configured: bool) -> &'static str {
    if dev {
        "/router"
    } else if configured {
        "/chat"
    } else {
        "/welcome"
    }
}

/// 计算本次启动的入口路由路径（供 `Root` 的 `RouterProvider.index_path` 使用）。
pub fn initial_route() -> &'static str {
    choose_initial_route(is_dev(), is_configured())
}

#[cfg(test)]
mod tests {
    use super::choose_initial_route;

    #[test]
    fn dev_env_always_enters_router() {
        assert_eq!(choose_initial_route(true, false), "/router");
        assert_eq!(choose_initial_route(true, true), "/router");
    }

    #[test]
    fn prod_routes_by_configuration() {
        assert_eq!(choose_initial_route(false, true), "/chat");
        assert_eq!(choose_initial_route(false, false), "/welcome");
    }
}