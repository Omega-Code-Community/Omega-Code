#[macro_use]
extern crate OmegaCode;
#[macro_use]
extern crate log;
#[macro_use]
extern crate flexi_logger;

use flexi_logger::{
    Cleanup, Criterion, Duplicate, FileSpec, Logger, Naming, WriteMode, detailed_format,
};

fn main() {
    init_logger();
    if let Err(e) = OmegaCode::run() {
        error!("Application error: {}", e);
        // Print full error chain
        let mut current = e.source();
        while let Some(cause) = current {
            error!("Caused by: {}", cause);
            current = cause.source();
        }
        std::process::exit(1);
    }
}

fn init_logger() {
    let _logger = Logger::try_with_str("trace")
        .unwrap()
        .log_to_file(
            FileSpec::default()
                .directory(app_dir!() + "/logs")
                .basename("omega")
                .suffix("log"),
        )
        .rotate(
            Criterion::Size(10_000_000),
            Naming::Numbers,
            Cleanup::KeepLogFiles(3),
        )
        .write_mode(WriteMode::Direct)
        .duplicate_to_stderr(Duplicate::Debug) // Logger level, production env should be Info level
        .format_for_files(detailed_format)
        .start()
        .unwrap();
}
