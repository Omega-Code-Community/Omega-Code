#[macro_use]
extern crate OmegaCode;
#[macro_use]
extern crate log;
#[macro_use]
extern crate flexi_logger;
#[macro_use]
extern crate rust_i18n;

use flexi_logger::{
    Cleanup, Criterion, Duplicate, FileSpec, Logger, Naming, WriteMode, detailed_format,
};
use log::{debug, error, info, trace, warn};
use rust_i18n::{set_locale, t};

rust_i18n::i18n!("locales");
fn main() {
    set_locale("en");
    init_logger();
    info!("{}", t!("logger_is_initialized"));
    info!("{}", t!("test_message", name = "OmegaCode"));
    info!("{}", t!("current_locale", locale_name = "en"));
    OmegaCode::run();
}

fn init_logger() {
    let _logger = Logger::try_with_str("trace")
        .unwrap()
        .log_to_file(
            FileSpec::default()
                .directory(app_dir!())
                .basename("omega")
                .suffix("log"),
        )
        .rotate(
            Criterion::Size(10_000_000),
            Naming::Numbers,
            Cleanup::KeepLogFiles(3),
        )
        .write_mode(WriteMode::BufferAndFlush)
        .duplicate_to_stderr(Duplicate::Debug) // Logger level, production env should be Info level
        .format_for_files(detailed_format)
        .start()
        .unwrap();
}
