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
                    KeyCode::Char('1') => navigate.push("/welcome"),
                    KeyCode::Char('2') => navigate.push("/chat"),
                    KeyCode::Char('3') => navigate.push("/router"),
                    _ => {}
                }
            }
        }
    });

    //todo 这里的逻辑是通过数据库查询是否初始化，从而确定要路由到 Welcome 还是 Chat

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