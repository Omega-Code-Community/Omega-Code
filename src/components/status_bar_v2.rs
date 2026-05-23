use ratatui_kit::{component, element, AnyElement, ElementExt, Hooks, Props, Store, UseState};
use ratatui_kit::components::View;
use ratatui_kit::ratatui::{TerminalOptions, Viewport};
use ratatui_kit::ratatui::layout::{Direction, Constraint, Flex, Margin};
use ratatui_kit::ratatui::prelude::Color;
use ratatui_kit::ratatui::style::Style;
use ratatui_kit::ratatui::text::Line;
use crate::components::input_bar::InputBar;
use crate::components::status_bar::StatusBar;

#[derive(Props, Store)]
pub struct StatusBarV2Props {
    pub model_name: String,
    pub cache_size: f64,
    pub context_size: f64,
    pub context_total: f64,
}

impl Default for StatusBarV2Props {
    fn default() -> Self {
        Self {
            model_name: String::new(),
            cache_size: 0.0,
            context_size: 0.0,
            context_total: 0.0,
        }
    }
}

#[component]
pub fn StatusBarV2(mut hooks: Hooks, props: &StatusBarV2Props) -> impl Into<AnyElement<'static>> {

    let state = hooks.use_state(|| {
        Self {
            _marker: Default::default(),
        }
    });

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