use actix_files::Files;
use actix_web::{
    web::{get, Data},
    App, HttpServer,
};
use clap::Parser;
use num_cpus;
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

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
    let addr = cli.addr.as_deref().unwrap_or("127.0.0.1:8080");

    log::info!(
        "Starting service: \"planning-poker\", workers: {}, listening on: {}",
        num_cpus::get(),
        addr
    );

    let (mut game_server, game_handler) = game::GameServer::new();
    let server_task = tokio::spawn(async move { game_server.run().await });
    let session_count = Arc::new(Mutex::new(0usize));

    HttpServer::new(move || {
        let app = App::new()
            .app_data(Data::new(game_handler.clone()))
            .app_data(Data::new(session_count.clone()))
            .route("/ws", get().to(session::handler::<game::GameHandle>));

        let base_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let frontend_path = base_dir.join("../frontend/dist");

        app.service(Files::new("/", frontend_path).index_file("index.html"))
    })
    .bind(addr)?
    .run()
    .await?;

    if let Err(_) = server_task.await {
        log::error!("Game server task failed");
    }

    Ok(())
}
