#[macro_use]
extern crate rust_i18n;

use std::io;
use log::info;
use rust_i18n::t;
use crate::core::db::DatabaseManager;
use crate::ui::layout::App;

pub mod core;
pub mod runtime;
pub mod services;
pub mod ui;
pub mod utils;
pub mod components;
pub mod health;
pub mod platform;
pub mod db;

i18n!("locales", fallback = "en");
pub fn run() -> anyhow::Result<()> {
    info!("{}", t!("logger_is_initialized"));
    info!("{}", t!("test_message", name = "OmegaCode"));
    info!("{}", t!("current_locale", locale_name = "en"));
    let db = DatabaseManager::new()?;
    db.health_check()?;
    let mut terminal = ratatui::init();
    let mut app = App::default();
    let result = app.run(&mut terminal);
    ratatui::restore();
    result
}
