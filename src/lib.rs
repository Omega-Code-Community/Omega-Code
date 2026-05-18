#[macro_use]
extern crate rust_i18n;

use crate::core::db::DatabaseManager;
use crate::pages::layout::App;
use log::info;
use ratatui_kit::crossterm::event::{Event, KeyCode, KeyEventKind};
use ratatui_kit::prelude::RouterProvider;
use ratatui_kit::{AnyElement, ElementExt, Hooks, UseEvents, UseRouter, component, element};
use rust_i18n::t;
use std::io;

pub mod components;
pub mod core;
pub mod db;
pub mod health;
pub mod pages;
pub mod platform;
pub mod router;
pub mod runtime;
pub mod services;
pub mod state;
pub mod utils;

use crate::router::app_routes;

i18n!("locales", fallback = "en");

#[tokio::main]
pub async fn run() -> anyhow::Result<()> {
    info!("{}", t!("logger_is_initialized"));
    info!("{}", t!("test_message", name = "OmegaCode"));
    info!("{}", t!("current_locale", locale_name = "en"));
    let db = DatabaseManager::new()?;
    db.health_check()?;

    element!(Root)
        .into_any()
        .fullscreen()
        .await
        .expect("Failed to run the application");

    Ok(())
}

#[component]
pub fn Root(_hooks: Hooks) -> impl Into<AnyElement<'static>> {
    // 严格按照官方文档：RouterProvider 放最顶层
    element!(
        RouterProvider(
            routes: app_routes(),
            index_path: "/router",
        )
    )
}


