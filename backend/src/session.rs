use crate::{
    error::{Result, *},
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

async fn handle_error(result: Result<()>, session: &mut actix_ws::Session) -> Option<CloseReason> {
    if let Err(err) = result {
        log::error!("{}", err);

        match err {
            // handle errors that should close the connection
            Error::NicknameAlreadyInUse(_) | Error::NicknameCannotBeEmpty => {
                return Some(CloseReason {
                    code: 1008.into(),
                    description: Some(err.to_string()),
                });
            }
            // handle errors that should be sent to the user
            _ => {
                session
                    .text(
                        serde_json::to_string(&OutboundMessage::Error(err.to_string()))
                            .expect("failed to serialize error message"),
                    )
                    .await
                    .expect("failed to send error message to the user");
            }
        }
    }

    None
}

fn parse_inbound_message(text: &str, mode: &Option<Mode>) -> InboundMessage {
    match mode {
        Some(Mode::Json) => serde_json::from_str(text).unwrap_or(InboundMessage::Unknown),
        _ => InboundMessage::from_string(text),
    }
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
                        let inbound = parse_inbound_message(&text, &mode);
                        if let InboundMessage::Unknown = inbound {
                            log::error!("Unknown message: {}", text);
                            continue;
                        }

                        let result = handle_text_message(
                            &inbound,
                            &mut nickname,
                            &mut conn_id,
                            &game_handler,
                            &conn_tx,
                        )
                        .await;

                        {
                            let result = handle_error(result, &mut session).await;
                            if result.is_some() {
                                break result;
                            }
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
        match game_handler.disconnect(&conn_id).await {
            Ok(_) => {}
            Err(err) => log::error!("failed to disconnect user: {:?}: {}", conn_id, err),
        }
    }

    let _ = session.close(close_reason).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_parse_inbound_message_json() {
        let mode = Some(Mode::Json);

        let text = &json!({"connect": {"nickname": "Player1"}}).to_string();
        let result = parse_inbound_message(text, &mode);
        assert_eq!(
            result,
            InboundMessage::Connect {
                nickname: "Player1".to_string()
            }
        );

        let text = &json!({"setstatus": "Active"}).to_string();
        let result = parse_inbound_message(text, &mode);
        assert_eq!(
            result,
            InboundMessage::SetStatus(shared::UserStatus::Active)
        );

        let text = &json!({"vote": {"value": "2"}}).to_string();
        let result = parse_inbound_message(text, &mode);
        assert_eq!(
            result,
            InboundMessage::Vote {
                value: shared::Vote::Option(2)
            }
        );

        let text = &json!({"unknown": "message"}).to_string();
        let result = parse_inbound_message(text, &mode);
        assert_eq!(result, InboundMessage::Unknown);
    }

    #[tokio::test]
    async fn test_parse_inbound_message_text() {
        let mode = None;

        let text = "/join Player1";
        let result = parse_inbound_message(text, &mode);
        assert_eq!(
            result,
            InboundMessage::Connect {
                nickname: "Player1".to_string()
            }
        );

        let text = "/setaway";
        let result = parse_inbound_message(text, &mode);
        assert_eq!(result, InboundMessage::SetStatus(shared::UserStatus::Away));

        let text = "/setback";
        let result = parse_inbound_message(text, &mode);
        assert_eq!(
            result,
            InboundMessage::SetStatus(shared::UserStatus::Active)
        );

        let text = "2";
        let result = parse_inbound_message(text, &mode);
        assert_eq!(
            result,
            InboundMessage::Vote {
                value: shared::Vote::Option(2)
            }
        );

        let text = "unknown message";
        let result = parse_inbound_message(text, &mode);
        assert_eq!(
            result,
            InboundMessage::Vote {
                value: shared::Vote::Null
            }
        );
    }
}
