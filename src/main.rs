use actix_web::{
    web::{get, Data},
    App, HttpServer,
};
use env_logger::Env;
use tokio::task::spawn;

mod game;
mod session;

const BIND_ADDR: &str = "0.0.0.0:8080";

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let (game_server, game_handler) = game::GameServer::new();

    let game_server = spawn(game_server.run());

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(game_handler.clone()))
            .route("/ws", get().to(session::handler::<game::GameHandle>))
    })
    .bind(BIND_ADDR)?
    .run()
    .await?;

    if let Err(_) = game_server.await {
        log::error!("Game server task failed");
    }

    Ok(())
}
