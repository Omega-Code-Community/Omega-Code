//! 聊天页面：把顶层 Agent 接入 TUI。
//!
//! 交互模型（Claude Code 风格）：
//! - 已完成的消息通过 `insert_before` 刷入终端原生滚动区（可向上滚动查看历史）；
//! - 当前正在流式生成的回复渲染在页面底部的「实时区」，随 token 到达不断重绘；
//! - 输入框回车提交后，使用 `tokio::spawn` 驱动一轮 Agent 对话，事件通过 mpsc 通道回传。

use std::sync::Arc;

use ratatui_kit::components::{Border, Text, TextArea, TextParagraph, View};
use ratatui_kit::crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui_kit::ratatui::layout::{Constraint, Direction, Position};
use ratatui_kit::ratatui::prelude::Color;
use ratatui_kit::ratatui::style::{Style, Stylize};
use ratatui_kit::ratatui::text::Line;
use ratatui_kit::ratatui::widgets::Paragraph;
use ratatui_kit::{
    component, element, AnyElement, Hooks, InsertBeforeHandler, State, SystemContext,
    UseContext, UseEvents, UseInsertBefore, UseState, UseTerminalSize,
};
use tokio::sync::{mpsc, Mutex};

use crate::components::status_bar::StatusBar;
use crate::core::agent::{Agent, AgentEvent};
use crate::core::config::AppConfig;

/// Agent 持有单元：初始化可能失败，失败时保留错误信息用于展示。
struct AgentCell {
    agent: Option<Agent>,
    error: Option<String>,
    /// 当前工作目录，用于状态栏展示。
    working_dir: String,
}

/// 依据环境配置构建 Agent（并尽力开启会话持久化）。
fn init_agent() -> AgentCell {
    let fallback_dir = || {
        std::env::current_dir()
            .map(|p| p.display().to_string())
            .unwrap_or_default()
    };

    match AppConfig::from_env() {
        Ok(cfg) => {
            let working_dir = cfg.working_dir.display().to_string();
            match Agent::from_config(&cfg) {
                Ok(mut agent) => {
                    // 持久化为尽力而为：失败仅告警，不阻断对话。
                    if let Err(e) = agent.enable_persistence_auto(Some("TUI Chat")) {
                        log::warn!("Failed to enable chat persistence: {e}");
                    }
                    AgentCell {
                        agent: Some(agent),
                        error: None,
                        working_dir,
                    }
                }
                Err(e) => AgentCell {
                    agent: None,
                    error: Some(e.to_string()),
                    working_dir,
                },
            }
        }
        Err(e) => AgentCell {
            agent: None,
            error: Some(e.to_string()),
            working_dir: fallback_dir(),
        },
    }
}

/// 估算文本在给定终端宽度下换行后占用的行数。
fn wrapped_rows(text: &str, width: u16) -> usize {
    let w = if width == 0 { 80 } else { width as usize };
    text.split('\n')
        .map(|line| {
            let n = line.chars().count();
            if n == 0 {
                1
            } else {
                (n + w - 1) / w
            }
        })
        .sum()
}

/// 在后台任务中执行一轮 Agent 对话，并把事件映射为 UI 更新。
#[allow(clippy::too_many_arguments)]
fn spawn_turn(
    text: String,
    agent_cell: Arc<Mutex<AgentCell>>,
    mut streaming: State<String>,
    mut status: State<String>,
    mut busy: State<bool>,
    insert_before: InsertBeforeHandler,
    term_w: u16,
) {
    tokio::spawn(async move {
        let (tx, mut rx) = mpsc::unbounded_channel::<AgentEvent>();

        // 单独的任务持有 Agent 锁并执行对话，主任务并发消费事件以实现实时刷新。
        let cell = agent_cell.clone();
        let input = text.clone();
        let turn = tokio::spawn(async move {
            let mut guard = cell.lock().await;
            match guard.agent.as_mut() {
                Some(agent) => {
                    let _ = agent.run_turn(&input, tx).await;
                }
                None => {
                    let err = guard
                        .error
                        .clone()
                        .unwrap_or_else(|| "Agent not initialized".to_string());
                    let _ = tx.send(AgentEvent::Error(err));
                }
            }
            // tx 在此处被释放，rx 随之结束。
        });

        let mut acc = String::new();
        while let Some(event) = rx.recv().await {
            match event {
                AgentEvent::Token(t) => {
                    acc.push_str(&t);
                    streaming.set(acc.clone());
                }
                AgentEvent::ToolCall { name, .. } => {
                    status.set(format!("calling tool: {name} ..."));
                }
                AgentEvent::ToolResult { name, .. } => {
                    status.set(format!("tool {name} finished"));
                }
                AgentEvent::Error(e) => {
                    status.set(format!("error: {e}"));
                }
                AgentEvent::Done => break,
            }
        }
        let _ = turn.await;

        // 将完成的回复刷入终端滚动区，并清空实时区。
        let trimmed = acc.trim_end();
        if !trimmed.is_empty() {
            let height = wrapped_rows(trimmed, term_w).min(1000) as u16;
            insert_before
                .render_before(Paragraph::new(trimmed.to_string()), height)
                .finish();
        }

        streaming.set(String::new());
        status.set(String::new());
        busy.set(false);
    });
}

#[component]
pub fn ChatPage(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    // ---- 状态（hook 顺序在所有渲染中保持一致）----
    let mut input = hooks.use_state(String::new);
    let streaming = hooks.use_state(String::new);
    let mut status = hooks.use_state(String::new);
    let mut busy = hooks.use_state(|| false);
    let mut should_exit = hooks.use_state(|| false);
    let agent_cell = hooks.use_state(|| Arc::new(Mutex::new(init_agent())));
    let (term_w, term_h) = hooks.use_terminal_size();
    let insert_before = hooks.use_insert_before();

    // 退出处理（非 hook）。
    {
        let mut system_ctx = hooks.use_context_mut::<SystemContext>();
        if should_exit.get() {
            system_ctx.exit();
        }
    }

    // 读取初始化错误与工作目录（非阻塞）。
    let (init_error, working_dir) = {
        let arc = agent_cell.read().clone();
        match arc.try_lock() {
            Ok(cell) => (cell.error.clone(), cell.working_dir.clone()),
            Err(_) => (None, String::new()),
        }
    };

    // 实时区展示内容：优先流式文本，其次状态提示，最后初始化错误。
    let stream_txt = streaming.read().clone();
    let status_txt = status.read().clone();
    let live_text = if !stream_txt.is_empty() {
        stream_txt
    } else if busy.get() {
        if status_txt.is_empty() {
            "thinking...".to_string()
        } else {
            status_txt
        }
    } else if let Some(err) = &init_error {
        format!("Agent unavailable: {err}")
    } else {
        String::new()
    };

    // 计算实时区滚动偏移，使最新内容贴底显示。
    let live_height = term_h.saturating_sub(11).max(1) as usize;
    let live_rows = wrapped_rows(&live_text, term_w);
    let scroll_y = live_rows.saturating_sub(live_height) as u16;

    // ---- 键盘事件 ----
    // State 为 Copy，move 闭包按副本捕获，可在 FnMut 中直接修改（同一底层存储）。
    hooks.use_events(move |event| {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return;
            }
            match key.code {
                KeyCode::Esc => {
                    should_exit.set(true);
                }
                KeyCode::Enter => {
                    let text = input.read().trim().to_string();
                    if text.is_empty() || busy.get() {
                        return;
                    }

                    // 回显用户消息到滚动区。
                    let echoed = format!("> {text}");
                    let height = wrapped_rows(&echoed, term_w).min(1000) as u16;
                    insert_before
                        .render_before(
                            Paragraph::new(Line::styled(echoed, Style::default().fg(Color::Cyan))),
                            height,
                        )
                        .finish();

                    input.set(String::new());
                    busy.set(true);
                    status.set("thinking...".to_string());

                    spawn_turn(
                        text,
                        agent_cell.read().clone(),
                        streaming,
                        status,
                        busy,
                        insert_before.clone(),
                        term_w,
                    );
                }
                _ => {}
            }
        }
    });

    // ---- 渲染 ----
    let mut input_cb = input;

    element!(
        View(flex_direction: Direction::Vertical,) {
            View(height: Constraint::Min(3),) {
                Text(
                    text: TextParagraph::from(live_text),
                    style: Style::default(),
                    wrap: Some(false),
                    scroll: Position::new(0, scroll_y),
                )
            }
            View(height: Constraint::Length(6),) {
                Border(
                    height: Constraint::Length(6),
                    style: Style::default().green(),
                    top_title: Line::styled("Composer", Style::default().yellow()).centered(),
                ) {
                    TextArea(
                        value: input.read().to_string(),
                        is_focus: true,
                        on_change: move |new_value: String| {
                            input_cb.set(new_value);
                        },
                        multiline: false,
                        cursor_style: Style::default().on_green(),
                        placeholder: Some("Type a message... (Enter to send, Esc to quit)".to_string()),
                        placeholder_style: Style::default().green(),
                    )
                }
            }
            View(height: Constraint::Length(4),) {
                StatusBar(
                    project_path: if working_dir.is_empty() {
                        None
                    } else {
                        Some(working_dir.clone().into())
                    },
                    context_length: live_rows,
                    max_context_length: 1000000usize,
                    style: Style::default(),
                    progress_style: Style::default().fg(Color::Green),
                    border_style: Style::default(),
                )
            }
        }
    )
}
