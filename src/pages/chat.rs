use ratatui_kit::{component, element, AnyElement, ElementExt, Hooks};
use ratatui_kit::components::View;
use ratatui_kit::ratatui::{TerminalOptions, Viewport};
use ratatui_kit::ratatui::layout::{Direction, Constraint};
use ratatui_kit::ratatui::prelude::Color;
use ratatui_kit::ratatui::style::Style;
use crate::components::input_bar::InputBar;
use crate::components::status_bar::StatusBar;

#[component]
pub fn ChatPage(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    element!(
        View(flex_direction: Direction::Vertical,) {
            View(height: Constraint::Length(10),) {
                InputBar()
            }
            View(height: Constraint::Length(4),) {
                StatusBar(
                    project_path: Some("/home/project/OmegaProject/".into()),
                    context_length: 123987usize,
                    max_context_length: 1000000usize,
                    style: Style::default(),
                    progress_style: Style::default().fg(Color::Green),
                    border_style: Style::default(),
                )
            }
        }
    )
}