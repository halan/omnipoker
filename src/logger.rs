use chrono::Local;
use env_logger::{Builder, Env, Target};
use std::io::Write;

pub fn init(log_level: &str) {
    Builder::from_env(Env::default().default_filter_or(log_level))
        .target(Target::Stdout)
        .filter_module("actix_server", log::LevelFilter::Warn)
        .filter_module("actix_web", log::LevelFilter::Warn)
        .format(|buf, record| {
            writeln!(
                buf,
                "[{} {}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        .init();
}
