use crate::{game::GameHandle, session::stream_handler};
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

const MAX_SESSIONS: usize = 15;

#[get("/ws")]
pub async fn handler(
    req: HttpRequest,
    stream: Payload,
    game_handler: web::Data<GameHandle>,
    session_count: web::Data<Arc<Mutex<usize>>>,
    query: web::Query<QueryParams>,
) -> Result<HttpResponse, actix_web::Error> {
    let (res, session, msg_stream) = actix_ws::handle(&req, stream)?;

    {
        let mut session_count_guard = session_count.lock().map_err(|_| {
            log::error!("Failed to acquire session count lock");
            actix_web::error::ErrorInternalServerError("Failed to acquire session count lock")
        })?;

        if *session_count_guard >= MAX_SESSIONS {
            log::warn!("Too many concurrent sessions; rejecting new session");
            return Ok(HttpResponse::TooManyRequests().finish());
        }

        *session_count_guard += 1;
        log::debug!("Session started. Active sessions: {}", *session_count_guard);
    }

    let session_count = session_count.clone();
    spawn_local(async move {
        stream_handler(
            (*game_handler.get_ref()).clone(),
            session,
            msg_stream,
            query.mode.clone(),
        )
        .await;

        if let Ok(mut session_count_guard) = session_count.lock() {
            *session_count_guard -= 1;
            log::debug!("Session ended. Active sessions: {}", *session_count_guard);
        } else {
            log::error!("Failed to acquire session count lock for decrement");
        }
    });

    Ok(res)
}
