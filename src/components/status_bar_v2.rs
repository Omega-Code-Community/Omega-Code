use ratatui_kit::{component, element, AnyElement, ElementExt, Hooks};
use ratatui_kit::components::View;
use ratatui_kit::ratatui::{TerminalOptions, Viewport};
use ratatui_kit::ratatui::layout::{Direction, Constraint, Flex, Margin};
use ratatui_kit::ratatui::prelude::Color;
use ratatui_kit::ratatui::style::Style;
use ratatui_kit::ratatui::text::Line;
use crate::components::input_bar::InputBar;
use crate::components::status_bar::StatusBar;

#[component]
pub fn StatusBarV2(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    element!(
        View(
            flex_direction: Direction::Horizontal,
            justify_content: Flex::SpaceBetween,
            margin: Margin::new(10, 0),
            height: Constraint::Length(1)
        ) {
            // Model name
            View() {
                $Line::styled(format!("Model: {}", todo!()), Style::default().fg(Color::White))
                           }
            // Cache size
            View() {
                $Line::styled(format!("Cache: {}", todo!()), Style::default().fg(Color::White))
            }
            // Context size
            View() {
                $Line::styled(format!("Context: {}", todo!()), Style::default().fg(Color::White))
            }
        }
    )
}