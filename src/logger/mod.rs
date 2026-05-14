use flexi_logger::{
    Cleanup, Criterion, Duplicate, FileSpec, Logger, LogSpecification, Naming, WriteMode,
};
use log::LevelFilter;

/// 初始化全局日志
/// 同时输出到：控制台(彩色) + 文件(滚动)
pub fn init_logger() -> Result<(), Box<dyn std::error::Error>> {
    let log_spec = LogSpecification::builder()
        .default(LevelFilter::Debug)
        .build();

    // 2. 文件配置：每天一个文件，保留 7 天
    let file_spec = FileSpec::default()
        .directory("logs")
        .basename("omega")
        .suppress_timestamp()
        .append_suffix(false);

    // 3. 启动日志器
    Logger::from(log_spec)
        .write_mode(WriteMode::Async)
        .duplicate_to_stdout(Duplicate::Info)
        .log_to_file(file_spec)
        .rotate(
            Criterion::Age(chrono::Days(1)),
            Naming::Timestamps,
            Cleanup::KeepLogFiles(7),
        )
        .color(flexi_logger::ColorOpt::Auto)
        .format(flexi_logger::detailed_format)
        .start()?;

    log::info!("日志模块初始化成功");
    Ok(())
}