use actix_web::{web::Data, App, HttpServer};
use clap::Parser;
use limit::Limit;
use num_cpus;
use std::sync::{Arc, Mutex};

mod frontend;
mod game;
mod limit;
mod logger;
mod session;
mod ws;

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(help = "Specify the address for the server (e.g., '127.0.0.1:8080').")]
    addr: Option<String>,
    #[arg(short, long, help = "Specify the maximum limit of users.")]
    limit: Option<usize>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
    logger::init("info");

    let cli = Cli::parse();
    let addr = cli.addr.as_deref().unwrap_or("127.0.0.1:8080");
    let limit = Limit::new(cli.limit.unwrap_or(15));

    log::info!(
        "Starting service: \"planning-poker\", workers: {}, listening on: {}",
        num_cpus::get(),
        addr
    );

    let (mut game_server, game_handler) = game::GameServer::new();
    let server_task = tokio::spawn(async move { game_server.run().await });
    let session_count = Arc::new(Mutex::new(limit));

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(game_handler.clone()))
            .app_data(Data::new(session_count.clone()))
            .service(ws::handler)
            .service(frontend::assets)
    })
    .bind(addr)?
    .run()
    .await?;

    if server_task.await.is_err() {
        log::error!("Game server task failed");
    }

    Ok(())
}
