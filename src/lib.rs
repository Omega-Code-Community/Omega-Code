#[macro_use]
extern crate rust_i18n;

use std::io;
use log::info;
use ratatui_kit::{element, ElementExt};
use ratatui_kit::prelude::RouterProvider;
use rust_i18n::t;
use crate::core::db::DatabaseManager;
use crate::pages::layout::App;

pub mod core;
pub mod runtime;
pub mod services;
pub mod pages;
pub mod utils;
pub mod components;
pub mod health;
pub mod platform;
pub mod db;
pub mod state;
pub mod router;

// use crate::pages::chat::chat_page;
use crate::pages::welcome::WelcomePage;

i18n!("locales", fallback = "en");

#[tokio::main]
pub async fn run() -> anyhow::Result<()> {
    info!("{}", t!("logger_is_initialized"));
    info!("{}", t!("test_message", name = "OmegaCode"));
    info!("{}", t!("current_locale", locale_name = "en"));
    let db = DatabaseManager::new()?;
    db.health_check()?;
    element!(WelcomePage()).fullscreen().await.expect("Failed to render welcome page");
    Ok(())
}
