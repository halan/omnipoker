use chrono::Local;
use clap::ValueEnum;
use colored::*;
use env_logger::{Builder, Env, Target};
use std::io::Write;

#[derive(ValueEnum, Clone)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}

impl From<LogLevel> for log::Level {
    fn from(log_level: LogLevel) -> Self {
        match log_level {
            LogLevel::Error => log::Level::Error,
            LogLevel::Warn => log::Level::Warn,
            LogLevel::Info => log::Level::Info,
            LogLevel::Debug => log::Level::Debug,
            LogLevel::Trace => log::Level::Trace,
        }
    }
}

pub fn init(log_level: LogLevel) {
    let log_level: log::Level = log_level.into();

    Builder::from_env(Env::default().default_filter_or(log_level.to_string()))
        .target(Target::Stdout)
        .filter_module("actix_server", log::LevelFilter::Warn)
        .filter_module("actix_web", log::LevelFilter::Warn)
        .format(|buf, record| {
            let level = match record.level() {
                log::Level::Error => "ERROR".red().bold(),
                log::Level::Warn => "WARN".yellow().bold(),
                log::Level::Info => "INFO".green().bold(),
                log::Level::Debug => "DEBUG".blue().bold(),
                log::Level::Trace => "TRACE".purple().bold(),
            };

            let timestamp = format!("[{}]", Local::now().format("%Y-%m-%d %H:%M:%S")).black();
            writeln!(buf, "{} {} {}", timestamp, level, record.args())
        })
        .init();
}
