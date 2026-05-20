#[macro_use]
extern crate rust_i18n;

use crate::pages::layout::App;
use log::{error, info};
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

    if !health_check() {
        error!("Health check failed");
        return Err(anyhow::anyhow!("Health check failed"));
    }

    element!(Root)
        .into_any()
        .fullscreen()
        .await
        .expect("Failed to run the application");

    Ok(())
}

#[component]
pub fn Root(_hooks: Hooks) -> impl Into<AnyElement<'static>> {
    element!(
        RouterProvider(
            routes: app_routes(),
            index_path: "/router",
        )
    )
}

fn health_check() -> bool {
    health::network_checker::check_network() && health::db_checker::check_db().unwrap()
}


