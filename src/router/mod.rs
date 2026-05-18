use ratatui_kit::prelude::*;
use crate::pages::chat::ChatPage;
use crate::pages::welcome::WelcomePage;
use crate::pages::router::RouterPage;

pub fn app_routes() -> Vec<Route> {
    let routes = routes!{
        "/welcome" => WelcomePage,
        "/chat" => ChatPage,
        "/router" => RouterPage,
    };
    routes
}