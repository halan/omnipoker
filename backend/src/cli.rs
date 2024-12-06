use crate::{limit::Limit, logger::LogLevel};
use clap::Parser;

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(help = "Specify the address for the server (e.g., '127.0.0.1:8080').")]
    addr: Option<String>,
    #[arg(short, long, help = "Specify the maximum limit of users.")]
    limit: Option<Limit>,
    #[arg(long, help = "Log level")]
    log: Option<LogLevel>,
}

pub fn get_args() -> (String, Limit, LogLevel) {
    let cli = Cli::parse();

    let addr = cli.addr.as_deref().unwrap_or("127.0.0.1:8080");
    let limit = cli.limit.unwrap_or_default();
    let log_level = cli.log.unwrap_or_default();

    (addr.to_owned(), limit, log_level)
}
