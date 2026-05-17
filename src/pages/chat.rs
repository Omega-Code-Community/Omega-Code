use ratatui_kit::{component, element, AnyElement, ElementExt, Hooks};
use ratatui_kit::components::View;
use ratatui_kit::ratatui::{TerminalOptions, Viewport};
use ratatui_kit::ratatui::layout::Direction;
use crate::components::input_bar::InputBar;

#[component]
pub fn ChatPage(mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    element!(ratatui_kit::components::textarea::TextArea)
}