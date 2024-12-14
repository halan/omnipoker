use actix_web::{web::Data, App, HttpServer};
use std::sync::{Arc, Mutex};

mod cli;
mod error;
mod game;
mod handlers;
mod limit;
mod logger;
mod session;

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
    let (addr, limit, log_level) = cli::get_args();

    logger::init(&log_level);
    logger::welcome(&addr, &limit);

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
    .bind(&addr)?
    .run()
    .await?;

    if server_task.await.is_err() {
        log::error!("Game server task failed");
    }

    Ok(())
}
