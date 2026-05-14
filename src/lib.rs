use std::io;
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
pub fn run() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::default();
    // components::welcome::output_welcome_screen("123","12");
    let result = app.run(&mut terminal);
    ratatui::restore();
    result
    // Ok(())
}
