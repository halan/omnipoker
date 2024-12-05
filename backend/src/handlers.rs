use crate::{
    game::GameHandle,
    limit::{release_session, try_acquire_session, Limit},
    session,
};
use actix_web::{get, web, HttpResponse, Responder};
use actix_web::{web::Payload, HttpRequest};
use rust_embed::RustEmbed;
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use tokio::task::spawn_local;

#[derive(Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Json,
    Text,
}

#[derive(Deserialize)]
pub struct QueryParams {
    mode: Option<Mode>,
}

#[get("/ws")]
pub async fn ws(
    req: HttpRequest,
    stream: Payload,
    game_handler: web::Data<GameHandle>,
    session_count: web::Data<Arc<Mutex<Limit>>>,
    query: web::Query<QueryParams>,
) -> Result<HttpResponse, actix_web::Error> {
    let (res, session, msg_stream) = actix_ws::handle(&req, stream)?;

    try_acquire_session(&session_count)?;

    let session_count = session_count.clone();
    spawn_local(async move {
        session::init(
            (*game_handler.get_ref()).clone(),
            session,
            msg_stream,
            query.mode.clone(),
        )
        .await;

        release_session(&session_count);
    });

    Ok(res)
}

#[derive(RustEmbed)]
#[folder = "$CARGO_MANIFEST_DIR/../frontend/dist"]
struct Assets;

#[get("/{filename:.*}")]
pub async fn assets(filename: web::Path<String>) -> impl Responder {
    let filename = if filename == web::Path::from("".to_owned()) {
        "index.html"
    } else {
        &*filename
    };
    if let Some(content) = Assets::get(&filename) {
        let body = content.data;
        let mime_type = mime_guess::from_path(&*filename).first_or_text_plain();
        HttpResponse::Ok()
            .content_type(mime_type.as_ref())
            .body(body)
    } else {
        HttpResponse::NotFound().body("404 - Not Found")
    }
}
