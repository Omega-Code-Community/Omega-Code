use ratatui_kit::{Component, ComponentDrawer, ComponentUpdater, Hooks, Props, UseEvents, UseExit, UseFuture, UseRouter, UseState};

use ratatui_kit::crossterm::event::{Event, KeyCode, KeyEventKind};

use ratatui_kit::ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};
use crate::runtime::download::DownloadProgress;
use crate::runtime::extract::ExtractProgress;
use crate::runtime::runtime::{ensure_jre, ensure_python};
use crate::services::welcome as panel_service;

#[derive(Props)]
pub struct WelcomeProps {
    pub version: Option<String>,
    pub style: Style,
    pub menu_style: Style,
    pub show_menu: bool,
}

impl Default for WelcomeProps {
    fn default() -> Self {
        Self {
            version: None,
            style: Style::default(),
            menu_style: Style::default(),
            show_menu: true,
        }
    }
}

pub struct Welcome {
    selected_index: usize,

    style: Style,
    menu_style: Style,
    version: Option<String>,
    show_menu: bool,
    /// 菜单模式下的状态提示（如打开控制台面板的结果）。
    menu_message: String,

    jre_ready: bool,
    python_ready: bool,
    is_installing: bool,
    install_message: String,

    stage: String,
    percent: f64,
    downloaded: u64,
    total: u64,
    speed: f64,
}

impl Component for Welcome {
    type Props<'a> = WelcomeProps;

    fn new(props: &Self::Props<'_>) -> Self {
        Self {
            selected_index: 0,

            style: props.style,
            menu_style: props.menu_style,
            version: props.version.clone(),
            show_menu: props.show_menu,
            menu_message: String::new(),

            jre_ready: false,
            python_ready: false,
            is_installing: false,
            install_message: "".to_string(),

            stage: String::new(),
            percent: 0.0,
            downloaded: 0,
            total: 0,
            speed: 0.0,
        }
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        mut hooks: Hooks,
        updater: &mut ComponentUpdater,
    ) {
        // 手写 `impl Component`（非 `#[component]` 宏）不会自动注入上下文栈：
        // 宏生成的 `update` 会先执行 `hooks.with_context_stack(updater.component_context_stack())`，
        // 而此处 `hooks` 由框架构造时 `context` 为 `None`。若不手动挂载即调用
        // `use_navigate`/`use_exit`/`use_context*` 等依赖上下文的钩子，会触发
        // `use_context.rs` 的 "context not available" 崩溃。故在任何钩子调用前先挂载上下文栈。
        let mut hooks = hooks.with_context_stack(updater.component_context_stack());

        self.style = props.style;
        self.menu_style = props.menu_style;
        self.version = props.version.clone();

        let jre_ready = hooks.use_state(|| self.jre_ready);
        let python_ready = hooks.use_state(|| self.python_ready);
        let is_installing = hooks.use_state(|| self.is_installing);
        let install_message = hooks.use_state(|| self.install_message.clone());
        let mut install_started = hooks.use_state(|| false);

        let stage = hooks.use_state(|| self.stage.clone());
        let percent = hooks.use_state(|| self.percent);
        let downloaded = hooks.use_state(|| self.downloaded);
        let total = hooks.use_state(|| self.total);
        let speed = hooks.use_state(|| self.speed);

        let mut state_index = hooks.use_state(|| self.selected_index);
        let menu_message = hooks.use_state(|| self.menu_message.clone());

        // 导航与退出句柄：须在条件性 use_future 之前无条件获取，以保持钩子调用顺序稳定。
        let mut navigate = hooks.use_navigate();
        let mut exit = hooks.use_exit();

        hooks.use_events(move |event| {
            if jre_ready.get() && python_ready.get() {
                if let Event::Key(key) = event {

                    if key.kind != KeyEventKind::Press {
                        return;
                    }

                    match key.code {
                        KeyCode::Up => {
                            state_index.set(
                                state_index.get().saturating_sub(1)
                            );
                        }

                        KeyCode::Down => {
                            let curr = state_index.get();
                            state_index.set((curr + 1).min(2));
                        }

                        KeyCode::Enter => match state_index.get() {
                            // 打开控制台面板：后台启动 java -jar 并在浏览器打开（缺 jar/JRE 时提示）。
                            0 => {
                                let mut msg = menu_message.clone();
                                std::thread::spawn(move || {
                                    let text = match panel_service::open_console_panel() {
                                        Ok(outcome) => outcome.message(),
                                        Err(e) => format!("cannot open console panel: {e}"),
                                    };
                                    msg.set(text);
                                });
                            }
                            // 开启新项目：进入聊天页（replace 不增长历史栈，规避 RouterHistory::push 越界缺陷）。
                            1 => navigate.replace("/chat"),
                            // 退出程序。
                            2 => exit(),
                            _ => {}
                        },

                        _ => {}
                    }
                }
            }
        });

        if !install_started.get() && !jre_ready.get() && !is_installing.get() {
            install_started.set(true);

            let mut jre_ready_clone = jre_ready.clone();
            let mut python_ready_clone = python_ready.clone();
            let mut is_installing_clone = is_installing.clone();
            let mut install_message_clone = install_message.clone();

            let mut dl_stage = stage.clone();
            let mut dl_percent = percent.clone();
            let mut dl_downloaded = downloaded.clone();
            let mut dl_total = total.clone();
            let mut dl_speed = speed.clone();

            let mut ex_stage = stage.clone();
            let mut ex_percent = percent.clone();
            let mut ex_downloaded = downloaded.clone();
            let mut ex_total = total.clone();

            hooks.use_future(async move {
                // JRE
                install_message_clone.set(t!("welcome.install_message1").to_string());
                let jre_ok = ensure_jre(
                    |p: DownloadProgress| {
                        dl_stage.set(t!("welcome.install_message5").to_string());
                        dl_percent.set(p.percent);
                        dl_downloaded.set(p.downloaded);
                        dl_total.set(p.total);
                        dl_speed.set(p.speed);
                    },
                    |p: ExtractProgress| {
                        ex_stage.set(t!("welcome.install_message6").to_string());
                        let pct = if p.total > 0 {
                            p.current as f64 / p.total as f64 * 100.0
                        } else {
                            0.0
                        };
                        ex_percent.set(pct);
                        ex_downloaded.set(p.current);
                        ex_total.set(p.total);
                    },
                )
                    .await
                    .is_ok();

                jre_ready_clone.set(jre_ok);
                if !jre_ok {
                    install_message_clone.set(t!("welcome.install_message3").to_string());
                    is_installing_clone.set(false);
                    return;
                }

                // Python
                install_message_clone.set(t!("welcome.install_message2").to_string());
                let python_ok = ensure_python(
                    |p: DownloadProgress| {
                        dl_stage.set(t!("welcome.install_message7").to_string());
                        dl_percent.set(p.percent);
                        dl_downloaded.set(p.downloaded);
                        dl_total.set(p.total);
                        dl_speed.set(p.speed);
                    },
                    |p: ExtractProgress| {
                        ex_stage.set(t!("welcome.install_message8").to_string());
                        let pct = if p.total > 0 {
                            p.current as f64 / p.total as f64 * 100.0
                        } else {
                            0.0
                        };
                        ex_percent.set(pct);
                        ex_downloaded.set(p.current);
                        ex_total.set(p.total);
                    },
                )
                    .await
                    .is_ok();

                python_ready_clone.set(python_ok);
                if python_ok {
                    install_message_clone.set(t!("welcome.install_message9").to_string());
                } else {
                    install_message_clone.set(t!("welcome.install_message4").to_string());
                }

                is_installing_clone.set(false);
            });
        }


        // 更新 self 的字段
        self.jre_ready = jre_ready.get();
        self.python_ready = python_ready.get();
        self.is_installing = is_installing.get();
        self.install_message = install_message.read().clone();
        // 正确更新 show_menu
        self.show_menu = jre_ready.get() && python_ready.get();

        self.selected_index = state_index.get();
        self.menu_message = menu_message.read().clone();

        self.stage = stage.read().clone();
        self.percent = percent.get();
        self.downloaded = downloaded.get();
        self.total = total.get();
        self.speed = speed.get();
    }

    fn draw(&mut self, drawer: &mut ComponentDrawer<'_, '_>) {
        let area = drawer.area;
        let buf = drawer.buffer_mut();

        let ascii_lines = [
            " ██████╗ ███╗   ███╗███████╗ ██████╗  █████╗",
            "██╔═══██╗████╗ ████║██╔════╝██╔════╝ ██╔══██╗",
            "██║   ██║██╔████╔██║█████╗  ██║  ███╗███████║",
            "██║   ██║██║╚██╔╝██║██╔══╝  ██║   ██║██╔══██║",
            "╚██████╔╝██║ ╚═╝ ██║███████╗╚██████╔╝██║  ██║",
            " ╚═════╝ ╚═╝     ╚═╝╚══════╝ ╚═════╝ ╚═╝  ╚═╝",
            "",
            " ██████╗ ██████╗ ██████╗ ███████╗",
            "██╔════╝██╔═══██╗██╔══██╗██╔════╝",
            "██║     ██║   ██║██║  ██║█████╗  ",
            "██║     ██║   ██║██║  ██║██╔══╝  ",
            "╚██████╗╚██████╔╝██████╔╝███████╗",
            " ╚═════╝ ╚═════╝ ╚═════╝ ╚══════╝",
        ];

        let version = self.version.clone().unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
        let welcome_text = t!("welcome.text");
        let menu_items = [
            t!("welcome.menu_items.0"),
            t!("welcome.menu_items.1"),
            t!("welcome.menu_items.2"),
        ];

        let mut lines = Vec::new();
        for line in ascii_lines {
            lines.push(Line::from(Span::styled(line, self.style)));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            t!("welcome.version", version = version),
            self.style.add_modifier(Modifier::DIM),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(welcome_text, self.style)));
        lines.push(Line::from(""));

        if !self.show_menu {
            lines.push(Line::from(Span::styled(
                &self.install_message,
                self.style.add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));

            if !self.stage.is_empty() {
                lines.push(Line::from(Span::styled(
                    &self.stage,
                    self.style.add_modifier(Modifier::BOLD),
                )));

                let percent = self.percent.clamp(0.0, 100.0);
                let w = 30;
                let fill = ((percent / 100.0 * w as f64) as usize).min(w);
                let bar = format!("[{}{}]", "█".repeat(fill), "░".repeat(w - fill));

                let mut spans = vec![
                    Span::styled(bar, self.menu_style),
                    Span::raw(format!(" {:.2}%", percent)),
                ];

                if self.stage.contains("Downloading") {
                    let mb = self.downloaded as f64 / 1024.0 / 1024.0;
                    let total_mb = self.total as f64 / 1024.0 / 1024.0;
                    let speed_mb = self.speed / 1024.0 / 1024.0;

                    spans.push(Span::raw(format!("  ({:.2} MB/{:.2} MB)", mb, total_mb)));
                    spans.push(Span::raw(format!("  {:.2} MB/s", speed_mb)));
                } else {
                    spans.push(Span::raw(format!("  {} / {}", self.downloaded, self.total)));
                }

                lines.push(Line::from(spans));
            }
        }

        lines.push(Line::from(""));

        if self.show_menu {
            lines.push(Line::from(Span::styled(
                t!("welcome.menu"),
                self.style.add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));

            let max = menu_items.len() - 1;
            self.selected_index = self.selected_index.min(max);

            for (i, item) in menu_items.iter().enumerate() {
                let selected = i == self.selected_index;
                let prefix = if selected { "→ " } else { "  " };
                let style = if selected {
                    self.menu_style.add_modifier(Modifier::BOLD | Modifier::REVERSED)
                } else {
                    self.style
                };

                lines.push(Line::from(Span::styled(
                    format!("{prefix}• {item}"),
                    style,
                )));
            }

            if !self.menu_message.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    self.menu_message.clone(),
                    self.style.add_modifier(Modifier::DIM),
                )));
            }
        }

        Paragraph::new(lines).render(area, buf);
    }
}