use ratatui_kit::{component, element, AnyElement};
use ratatui_kit::prelude::View;
use ratatui_kit::ratatui::prelude::Direction;
use crate::components::welcome::Welcome;

#[component]
pub fn WelcomePage() ->  impl Into<AnyElement<'static>> {
    element!(View(
        flex_direction: Direction::Horizontal,
    ) {
        Welcome()
    })
}
