use crate::game::{CommandHandler, ConnId, Msg, Nickname};
use actix_web::{web, web::Payload, HttpRequest, HttpResponse};
use actix_ws::AggregatedMessage;
use futures_util::{
    future::{select, Either},
    StreamExt as _,
};
use std::{
    pin::pin,
    time::{Duration, Instant},
};
use tokio::{sync::mpsc, task::spawn_local, time::interval};

async fn handle_text_message<'a, T: CommandHandler>(
    text: &str,
    nickname: &mut Option<Nickname>,
    conn_id: &mut Option<ConnId>,
    game_handler: &T,
    conn_tx: &mpsc::UnboundedSender<Msg>,
) {
    if nickname.is_none() {
        if let ["/join", new_nickname] = text.split_whitespace().collect::<Vec<_>>().as_slice() {
            *nickname = Some(new_nickname.to_string());
            *conn_id = game_handler
                .connect(conn_tx.clone(), new_nickname)
                .await
                .ok();
        }

        return;
    }

    game_handler
        .vote(conn_id.expect("conn_id is None"), text)
        .await;
}

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn stream_handler<T: CommandHandler>(
    game_handler: T,
    mut session: actix_ws::Session,
    msg_stream: actix_ws::MessageStream,
) {
    log::info!("connected");

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

                    AggregatedMessage::Text(text) => {
                        handle_text_message(
                            &text,
                            &mut nickname,
                            &mut conn_id,
                            &game_handler,
                            &conn_tx,
                        )
                        .await;
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

            // client WebSocket stream ended
            Either::Left((Either::Left((None, _)), _)) => break None,

            // chat messages received from other room participants
            Either::Left((Either::Right((Some(chat_msg), _)), _)) => {
                session
                    .text(chat_msg)
                    .await
                    .expect("failed to send chat message");
            }

            // all connection's message senders were dropped
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

pub async fn handler<T: CommandHandler + Clone + 'static>(
    req: HttpRequest,
    stream: Payload,
    game_handler: web::Data<T>,
) -> Result<HttpResponse, actix_web::Error> {
    let (res, session, msg_stream) = actix_ws::handle(&req, stream)?;

    spawn_local(stream_handler(
        (*game_handler.get_ref()).clone(),
        session,
        msg_stream,
    ));

    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::MockCommandHandler;
    use actix_web::web::Data;
    use actix_web::{dev::Service, test, web, App};
    use mockall::predicate::*;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_handle_text_message() {
        let mut game_handler = MockCommandHandler::new();

        let conn_tx = mpsc::unbounded_channel::<Msg>().0;

        let mut nickname = None;
        let mut conn_id = None;
        let new_nickname = "Player1";
        let conn_id_result = 42;

        game_handler
            .expect_connect()
            .withf({
                let conn_tx = conn_tx.clone();
                move |conn, nick| conn.same_channel(&conn_tx) && nick == new_nickname
            })
            .returning(move |_, _| Ok(conn_id_result));

        handle_text_message(
            "/join Player1",
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
        let vote_text = "1".to_string();

        game_handler
            .expect_vote()
            .with(eq(conn_id_result.clone()), eq(vote_text.clone()))
            .returning(|_, _| ());

        handle_text_message(
            &vote_text,
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

        let app = test::init_service(
            App::new()
                .app_data(Data::new(game_handler))
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
}
