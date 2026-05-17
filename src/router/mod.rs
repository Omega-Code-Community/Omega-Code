use ratatui_kit::prelude::*;
use crate::pages::welcome::WelcomePage;
// use crate::pages::chat::chat_page;
// use crate::pages::context::context_page;
pub fn app_routes() -> Vec<Route> {
    let routes = routes!{
        "/welcome" => WelcomePage,
    };
    routes
}