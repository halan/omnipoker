use actix::Actor;
use actix_web::{
    web::{get, Data},
    App, HttpServer,
};
use env_logger::Env;

mod game;
mod messages;
mod session;

const BIND_ADDR: &str = "127.0.0.1:8080";

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    let game_addr = game::Game::new().start();

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(game_addr.clone()))
            .route("/ws", get().to(session::handler))
    })
    .bind(BIND_ADDR)?
    .run()
    .await
}
