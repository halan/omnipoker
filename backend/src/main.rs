use actix_web::{web::Data, App, HttpServer};
use clap::Parser;
use colored::*;
use limit::Limit;
use logger::LogLevel;
use std::sync::{Arc, Mutex};

mod game;
mod handlers;
mod limit;
mod logger;
mod session;

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

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
    let cli = Cli::parse();
    let addr = cli.addr.as_deref().unwrap_or("127.0.0.1:8080");
    let limit = cli.limit.unwrap_or_default();
    let log_level = cli.log.unwrap_or_default();

    logger::init(log_level);

    log::info!(
        "Starting service: \"planning-poker\", limit of sessions: {}, listening on: {}",
        limit.max.to_string().blue(),
        addr.blue()
    );

    let (mut game_server, game_handler) = game::GameServer::new();
    let server_task = tokio::spawn(async move { game_server.run().await });
    let session_count = Arc::new(Mutex::new(limit));

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(game_handler.clone()))
            .app_data(Data::new(session_count.clone()))
            .service(handlers::ws)
            .service(handlers::assets)
    })
    .bind(addr)?
    .run()
    .await?;

    if server_task.await.is_err() {
        log::error!("Game server task failed");
    }

    Ok(())
}
