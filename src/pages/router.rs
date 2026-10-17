use ratatui_kit::{component, element, AnyElement, Hooks, UseEvents, UseRouter};
use ratatui_kit::components::View;
use ratatui_kit::crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui_kit::ratatui::layout::{ Constraint};
use ratatui_kit::ratatui::prelude::Stylize;
use ratatui_kit::ratatui::style::Style;
use ratatui_kit::ratatui::text::Line;

#[component]
pub fn RouterPage(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut navigate = hooks.use_navigate();

    hooks.use_events(move |event| {
        if let Event::Key(key_event) = event {
            if key_event.kind == KeyEventKind::Press {
                match key_event.code {
                    // 用 replace 而非 push：ratatui-kit 0.5.9 的 RouterHistory::push 在
                    // 历史长度达到 max_length 后会 pop_front 却不同步回退 current，
                    // 导致 insert(current) 越界 panic（index out of bounds）。
                    // 调试路由页只需切换当前页，replace 不增长历史栈，天然规避该缺陷。
                    KeyCode::Char('1') => navigate.replace("/welcome"),
                    KeyCode::Char('2') => navigate.replace("/chat"),
                    KeyCode::Char('3') => navigate.replace("/router"),
                    _ => {}
                }
            }
        }
    });

    // 本页为「开发调试路由页」：仅在开发环境（debug 构建或设置 OMEGA_DEV）作为入口，
    // 由 `crate::router::initial_route()` 决定；生产环境入口直接为 /welcome 或 /chat，
    // 永不进入本页，故下方 1/2/3 手动路由不对生产环境暴露
    // （见《系统功能设计》路由约束：路由调试函数禁止在生产环境调用）。

    element!(
        View(height: Constraint::Length(10),) {
            $Line::styled(
                    format!("Use Number 1/2/3 Keys to route"),
                    Style::default().fg(ratatui_kit::ratatui::style::Color::Blue).bold(),
                )
                .centered()
                .bold()
        }
    )
}