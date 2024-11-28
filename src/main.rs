use actix_web::{
    web::{get, Data},
    App, HttpServer,
};
use clap::Parser;
use num_cpus;
use tokio::task::spawn;

mod game;
mod logger;
mod session;

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    addr: Option<String>,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
    logger::init("info");

    let cli = Cli::parse();
    let (game_server, game_handler) = game::GameServer::new();
    let game_server = spawn(game_server.run());
    let addr = cli.addr.as_deref().unwrap_or("127.0.0.1:8080");

    log::info!(
        "Starting service: \"planning-poker\", workers: {}, listening on: {}",
        num_cpus::get(),
        addr
    );

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(game_handler.clone()))
            .route("/ws", get().to(session::handler::<game::GameHandle>))
    })
    .bind(addr)?
    .run()
    .await?;

    if let Err(_) = game_server.await {
        log::error!("Game server task failed");
    }

    Ok(())
}
