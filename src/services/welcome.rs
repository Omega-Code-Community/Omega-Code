//! 欢迎页服务：启动本地「控制台面板」并在浏览器中打开。
//!
//! 控制台面板（Spring Boot 后端 + Vue 前端）由 `build.sh` 打包为与 `OmegaCode`
//! 可执行文件同目录的 `console_panel.jar`，运行于 <http://localhost:5200>。本模块
//! 负责：探测面板是否已运行、定位 jar 与已安装的 JRE、以 `java -jar` 后台启动面板，
//! 并调用系统默认浏览器打开面板地址。
//!
//! 设计约束：所有探测/启动均为「尽力而为」——缺 jar 或缺 JRE 时返回带清晰原因的错误，
//! 由 TUI 以状态行提示用户，绝不 panic、不阻断欢迎页其余交互。

use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result};

use crate::platform::detect::{current_platform, Os};
use crate::runtime::runtime::installed_java;

/// 控制台面板服务端口。
pub const CONSOLE_PANEL_PORT: u16 = 5200;
/// 控制台面板访问地址。
pub const CONSOLE_PANEL_URL: &str = "http://localhost:5200";
/// 打包后的面板 jar 文件名（与可执行文件同目录）。
pub const CONSOLE_PANEL_JAR: &str = "console_panel.jar";
/// 等待面板端口就绪的轮询次数（每次 500ms，合计约 30s）。
const PANEL_READY_POLLS: u32 = 60;

/// 面板启动结果，用于 TUI 状态提示。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PanelOutcome {
    /// 面板已在运行，直接打开浏览器。
    AlreadyRunning,
    /// 面板未运行，已后台启动 `java -jar`，正等待端口就绪后自动打开浏览器。
    Starting,
}

impl PanelOutcome {
    /// 面向用户的一句话状态描述。
    pub fn message(&self) -> String {
        match self {
            PanelOutcome::AlreadyRunning => {
                format!("console panel already running — opened {CONSOLE_PANEL_URL}")
            }
            PanelOutcome::Starting => {
                format!("starting console panel — will open {CONSOLE_PANEL_URL} when ready")
            }
        }
    }
}

/// 定位 `console_panel.jar`：优先可执行文件同目录，其次当前工作目录。
///
/// 开发环境（`cargo run`）通常两处都没有该 jar，返回 `None`。
pub fn locate_console_jar() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let candidate = dir.join(CONSOLE_PANEL_JAR);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    let cwd = PathBuf::from(CONSOLE_PANEL_JAR);
    if cwd.is_file() {
        return Some(cwd);
    }
    None
}

/// 探测指定端口是否已可连接（用于判断面板是否已运行）。
fn port_open(port: u16) -> bool {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpStream::connect_timeout(&addr, Duration::from_millis(300)).is_ok()
}

/// 探测控制台面板是否已在运行。
pub fn panel_running() -> bool {
    port_open(CONSOLE_PANEL_PORT)
}

/// 依据操作系统构造「打开浏览器」所需的可执行程序与参数（纯函数，可测）。
///
/// Windows 使用 `cmd /C start "" <url>`（空标题占位），macOS 使用 `open`，
/// Linux 使用 `xdg-open`。
pub fn browser_command(os: Os, url: &str) -> (&'static str, Vec<String>) {
    match os {
        Os::Windows => (
            "cmd",
            vec![
                "/C".to_string(),
                "start".to_string(),
                String::new(),
                url.to_string(),
            ],
        ),
        Os::MacOS => ("open", vec![url.to_string()]),
        Os::Linux => ("xdg-open", vec![url.to_string()]),
    }
}

/// 用系统默认浏览器打开给定 URL（非阻塞 `spawn`）。
pub fn open_in_browser(url: &str) -> Result<()> {
    let os = current_platform().os;
    let (program, args) = browser_command(os, url);
    Command::new(program)
        .args(&args)
        .spawn()
        .with_context(|| format!("failed to launch browser via '{program}'"))?;
    Ok(())
}

/// 打开控制台面板：若未运行则后台启动 `java -jar`，随后打开浏览器。
///
/// 返回 [`PanelOutcome`] 描述所采取的动作。缺 jar 或缺 JRE 时返回错误（附清晰原因），
/// 调用方（欢迎页）据此提示用户，不影响其余菜单项。
pub fn open_console_panel() -> Result<PanelOutcome> {
    if panel_running() {
        open_in_browser(CONSOLE_PANEL_URL)?;
        return Ok(PanelOutcome::AlreadyRunning);
    }

    let jar = locate_console_jar().context(
        "console_panel.jar not found next to the executable (build the bundle with ./build.sh first)",
    )?;
    let java = installed_java().context(
        "JRE not installed; finish the runtime installation on the welcome screen first",
    )?;

    Command::new(&java)
        .arg("-jar")
        .arg(&jar)
        .spawn()
        .with_context(|| format!("failed to start console panel via '{}'", java.display()))?;

    // 后台轮询端口就绪再打开浏览器，避免阻塞 TUI；超时也尝试打开一次（面板可能启动较慢）。
    std::thread::spawn(|| {
        for _ in 0..PANEL_READY_POLLS {
            if panel_running() {
                let _ = open_in_browser(CONSOLE_PANEL_URL);
                return;
            }
            std::thread::sleep(Duration::from_millis(500));
        }
        let _ = open_in_browser(CONSOLE_PANEL_URL);
    });

    Ok(PanelOutcome::Starting)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn browser_command_matches_platform_conventions() {
        let (program, args) = browser_command(Os::Windows, "http://localhost:5200");
        assert_eq!(program, "cmd");
        assert_eq!(
            args,
            vec![
                "/C".to_string(),
                "start".to_string(),
                String::new(),
                "http://localhost:5200".to_string()
            ]
        );

        let (program, args) = browser_command(Os::MacOS, "http://x");
        assert_eq!(program, "open");
        assert_eq!(args, vec!["http://x".to_string()]);

        let (program, args) = browser_command(Os::Linux, "http://x");
        assert_eq!(program, "xdg-open");
        assert_eq!(args, vec!["http://x".to_string()]);
    }

    #[test]
    fn port_open_detects_active_listener() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        // 监听中：应探测为开启。
        assert!(port_open(port));
        // 显式持有 listener 至断言之后，避免端口在断言前被释放/复用。
        drop(listener);
    }

    #[test]
    fn panel_outcome_messages_mention_url() {
        assert!(PanelOutcome::AlreadyRunning.message().contains(CONSOLE_PANEL_URL));
        assert!(PanelOutcome::Starting.message().contains(CONSOLE_PANEL_URL));
    }

    #[test]
    fn locate_console_jar_does_not_panic() {
        // 开发环境下通常无 jar；仅验证调用安全（返回 Option，不 panic）。
        let _ = locate_console_jar();
    }
}
