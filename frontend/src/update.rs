use super::{Model, Msg};
use futures_util::{SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message};
use shared::{InboundMessage, OutboundMessage};
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

pub fn update(
    model: &mut Model,
    ctx: &Context<Model>,
    msg: <Model as BaseComponent>::Message,
) -> bool {
    match msg {
        Msg::Inbound(InboundMessage::Connect { nickname }) => {
            let link = ctx.link().clone();
            let ws = WebSocket::open("ws://127.0.0.1:8080/ws?mode=json").unwrap();

            let (write, mut read) = ws.split();
            model.ws_sink = Some(Rc::new(RefCell::new(write)));

            spawn_local(async move {
                while let Some(Ok(message)) = read.next().await {
                    if let Message::Text(text) = message {
                        link.send_message(Msg::Outbound(
                            serde_json::from_str(&text).unwrap_or(OutboundMessage::Unknown),
                        ));
                    }
                }
            });

            if let Some(ws_sink) = &model.ws_sink {
                let ws_sink = ws_sink.clone();
                spawn_local(async move {
                    let mut sink = ws_sink.borrow_mut();
                    if let Ok(text) = serde_json::to_string(&InboundMessage::Connect { nickname }) {
                        let _ = sink.send(Message::Text(text)).await;
                    };
                });
            }

            true
        }
        Msg::Inbound(inbound) => {
            if let Some(ws_sink) = &model.ws_sink {
                let ws_sink = ws_sink.clone();
                spawn_local(async move {
                    let mut sink = ws_sink.borrow_mut();
                    if let Ok(text) = serde_json::to_string(&inbound) {
                        sink.send(Message::Text(text)).await.unwrap();
                    };
                });
            }
            true
        }
        Msg::Outbound(outbound) => match outbound {
            OutboundMessage::UserList(users) => {
                model.user_lists = users;
                true
            }

            _ => {
                model.messages.push(outbound.to_string());
                true
            }
        },
    }
}
