use crate::game::{CommandHandler, ConnId, Nickname, OutboundMessage};
use actix_web::{web, web::Payload, HttpRequest, HttpResponse};
use actix_ws::{AggregatedMessage, CloseReason};
use futures_util::{
    future::{select, Either},
    StreamExt as _,
};
use serde::Deserialize;
use shared::InboundMessage;
use std::{
    pin::pin,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::{sync::mpsc, task::spawn_local, time::interval};

async fn handle_text_message<'a, T: CommandHandler>(
    inbound: &InboundMessage,
    nickname: &mut Option<Nickname>,
    conn_id: &mut Option<ConnId>,
    game_handler: &T,
    conn_tx: &mpsc::UnboundedSender<OutboundMessage>,
) -> Result<(), String> {
    if nickname.is_none() {
        if let InboundMessage::Connect {
            nickname: new_nickname,
        } = inbound
        {
            *nickname = Some(new_nickname.clone());
            let result = game_handler
                .connect(conn_tx.clone(), new_nickname.as_str())
                .await;

            match result {
                Ok(Ok(new_conn_id)) => {
                    *conn_id = Some(new_conn_id);
                    return Ok(());
                }
                Err(err) => {
                    log::error!("Failed to connect: {}", err);
                    return Err(err.to_string());
                }
                Ok(Err(err)) => {
                    log::error!("Failed to connect: {}", err);
                    return Err(err);
                }
            }
        }

        return Ok(());
    }

    if nickname.is_none() {
        return Err("Nickname is not set".to_string());
    }

    if let InboundMessage::Vote { value: vote } = inbound {
        if let Some(conn_id) = conn_id {
            game_handler.vote(conn_id.clone(), vote).await;
        }
    }

    Ok(())
}

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn stream_handler<T: CommandHandler>(
    game_handler: T,
    mut session: actix_ws::Session,
    msg_stream: actix_ws::MessageStream,
    mode: Option<Mode>,
) {
    let mut nickname = None;
    let mut conn_id = None;
    let mut last_heartbeat = Instant::now();
    let mut interval = interval(HEARTBEAT_INTERVAL);

    let (conn_tx, mut conn_rx) = mpsc::unbounded_channel();

    let msg_stream = msg_stream
        .max_frame_size(128 * 1024)
        .aggregate_continuations()
        .max_continuation_size(2 * 1024 * 1024);

    let mut msg_stream = pin!(msg_stream); // outbound

    let close_reason = loop {
        let tick = pin!(interval.tick()); // ticks
        let msg_rx = pin!(conn_rx.recv()); // inbound
        let messages = pin!(select(msg_stream.next(), msg_rx)); // inbound & outbound

        match select(messages, tick).await {
            // commands & messages received from client
            Either::Left((Either::Left((Some(Ok(msg)), _)), _)) => {
                log::debug!("msg: {msg:?}");

                match msg {
                    AggregatedMessage::Ping(bytes) => {
                        last_heartbeat = Instant::now();
                        session.pong(&bytes).await.expect("failed to send pong");
                    }

                    AggregatedMessage::Pong(_) => {
                        last_heartbeat = Instant::now();
                    }

                    // text message from client
                    AggregatedMessage::Text(text) => {
                        let inbound = match mode {
                            Some(Mode::Json) => match serde_json::from_str(&text) {
                                Ok(inbound) => inbound,
                                Err(err) => {
                                    log::error!("failed to parse JSON message: {}", err);
                                    continue;
                                }
                            },
                            _ => text.into(),
                        };

                        let result = handle_text_message(
                            &inbound,
                            &mut nickname,
                            &mut conn_id,
                            &game_handler,
                            &conn_tx,
                        )
                        .await;

                        if let Err(err) = result {
                            log::error!("{}", err);
                            break Some(CloseReason {
                                code: 1008.into(),
                                description: Some(err.to_owned()),
                            });
                        }
                    }

                    AggregatedMessage::Binary(_bin) => {
                        log::warn!("unexpected binary message");
                    }

                    AggregatedMessage::Close(reason) => break reason,
                }
            }

            // client WebSocket stream error
            Either::Left((Either::Left((Some(Err(err)), _)), _)) => {
                log::error!("{}", err);
                break None;
            }

            Either::Left((Either::Left((None, _)), _)) => break None,

            // messages to send to client
            Either::Left((Either::Right((Some(answer), _)), _)) => {
                let outbound = match mode {
                    Some(Mode::Json) => {
                        serde_json::to_string(&answer).expect("failed to serialize JSON message")
                    }
                    _ => answer.to_string(),
                };

                session
                    .text(outbound)
                    .await
                    .expect("failed to send chat message");
            }

            Either::Left((Either::Right((None, _)), _)) => unreachable!(
                "all connection message senders were dropped; chat server may have panicked"
            ),

            // heartbeat internal tick
            Either::Right((_inst, _)) => {
                // if no heartbeat ping/pong received recently, close the connection
                if Instant::now().duration_since(last_heartbeat) > CLIENT_TIMEOUT {
                    log::info!(
                        "client has not sent heartbeat in over {CLIENT_TIMEOUT:?}; disconnecting"
                    );
                    break None;
                }

                // send heartbeat ping
                let _ = session.ping(b"").await;
            }
        };
    };

    if let Some(conn_id) = conn_id {
        game_handler.disconnect(conn_id);
    }

    let _ = session.close(close_reason).await;
}

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

pub async fn handler<T: CommandHandler + Clone + 'static>(
    req: HttpRequest,
    stream: Payload,
    game_handler: web::Data<T>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::MockCommandHandler;
    use actix_web::web::Data;
    use actix_web::{dev::Service, test, web, App};
    use mockall::predicate::*;
    use shared::Vote;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_handle_text_message() {
        let mut game_handler = MockCommandHandler::new();

        let conn_tx = mpsc::unbounded_channel::<OutboundMessage>().0;

        let mut nickname = None;
        let mut conn_id = None;
        let new_nickname = "Player1";
        let conn_id_result = ConnId::new();

        game_handler
            .expect_connect()
            .withf({
                let conn_tx = conn_tx.clone();
                move |conn, nick| conn.same_channel(&conn_tx) && nick == new_nickname
            })
            .returning(move |_, _| Ok(Ok(conn_id_result)));

        let _ = handle_text_message(
            &InboundMessage::Connect {
                nickname: new_nickname.to_string(),
            },
            &mut nickname,
            &mut conn_id,
            &game_handler,
            &conn_tx,
        )
        .await;

        assert_eq!(nickname, Some(new_nickname.to_string()));
        assert_eq!(conn_id, Some(conn_id_result));

        let mut nickname = Some(new_nickname.to_string());
        let mut conn_id = Some(conn_id_result);
        let vote_text = Vote::new(1);

        game_handler
            .expect_vote()
            .with(eq(conn_id_result.clone()), eq(vote_text.clone()))
            .returning(|_, _| ());

        let _ = handle_text_message(
            &InboundMessage::Vote {
                value: vote_text.clone(),
            },
            &mut nickname,
            &mut conn_id,
            &game_handler,
            &conn_tx,
        )
        .await;
    }

    #[actix_rt::test]
    async fn test_websocket() {
        let _ = env_logger::builder().is_test(true).try_init();
        let game_handler = MockCommandHandler::new();
        let session_count = Arc::new(Mutex::new(0usize));

        let app = test::init_service(
            App::new()
                .app_data(Data::new(game_handler))
                .app_data(Data::new(session_count))
                .route("/ws", web::get().to(handler::<MockCommandHandler>)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/ws")
            .insert_header(("Upgrade", "websocket"))
            .insert_header(("Connection", "Upgrade"))
            .insert_header(("Sec-WebSocket-Key", "test_key"))
            .insert_header(("Sec-WebSocket-Version", "13"))
            .to_request();
        let resp = app.call(req).await.unwrap();

        assert_eq!(
            resp.status(),
            actix_web::http::StatusCode::SWITCHING_PROTOCOLS
        );
    }

    #[actix_rt::test]
    async fn test_websocket_limit() {
        let _ = env_logger::builder().is_test(true).try_init();
        let game_handler = MockCommandHandler::new();
        let session_count = Arc::new(Mutex::new(15usize));

        let app = test::init_service(
            App::new()
                .app_data(Data::new(game_handler))
                .app_data(Data::new(session_count))
                .route("/ws", web::get().to(handler::<MockCommandHandler>)),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/ws")
            .insert_header(("Upgrade", "websocket"))
            .insert_header(("Connection", "Upgrade"))
            .insert_header(("Sec-WebSocket-Key", "test_key"))
            .insert_header(("Sec-WebSocket-Version", "13"))
            .to_request();
        let resp = app.call(req).await.unwrap();

        assert_eq!(
            resp.status(),
            actix_web::http::StatusCode::TOO_MANY_REQUESTS
        );
    }
}
