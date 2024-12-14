use crate::{
    error::Result,
    game::{ConnId, GameHandle, Nickname, OutboundMessage},
    handlers::Mode,
};
use actix_ws::{AggregatedMessage, CloseReason};
use futures_util::{
    future::{select, Either},
    StreamExt as _,
};
use shared::InboundMessage;
use std::{
    pin::pin,
    time::{Duration, Instant},
};
use tokio::{sync::mpsc, time::interval};

async fn handle_text_message(
    inbound: &InboundMessage,
    nickname: &mut Option<Nickname>,
    conn_id: &mut Option<ConnId>,
    game_handler: &GameHandle,
    conn_tx: &mpsc::UnboundedSender<OutboundMessage>,
) -> Result<()> {
    if nickname.is_none() {
        if let InboundMessage::Connect {
            nickname: new_nickname,
        } = inbound
        {
            *conn_id = Some(game_handler.connect(conn_tx.clone(), new_nickname).await?);
            *nickname = Some(new_nickname.to_string());
        }

        return Ok(());
    }

    // Commands after identifying

    if let Some(conn_id) = conn_id {
        match inbound {
            InboundMessage::SetStatus(value) => game_handler.set_status(conn_id, value).await?,
            InboundMessage::Vote { value } => game_handler.vote(conn_id, value).await?,
            _ => {}
        }
    }

    Ok(())
}

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub async fn init(
    game_handler: GameHandle,
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
                                description: Some(err.to_string()),
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
        match game_handler.disconnect(&conn_id) {
            Ok(_) => {}
            Err(err) => log::error!("failed to disconnect user: {:?}: {}", conn_id, err),
        }
    }

    let _ = session.close(close_reason).await;
}
