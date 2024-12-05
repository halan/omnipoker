use crate::{
    game::GameHandle,
    limit::{release_session, try_acquire_session, Limit},
    session,
};
use actix_web::{
    get,
    web::{self, Payload},
    HttpRequest, HttpResponse,
};
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
pub async fn handler(
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
