use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message, WebSocketError};
use shared::{InboundMessage, OutboundMessage};
use std::{cell::RefCell, rc::Rc};
pub type WebSocketSink = Rc<RefCell<SplitSink<WebSocket, Message>>>;

pub fn connect_websocket(
    on_message: impl Fn(OutboundMessage) + 'static,
    on_error: impl Fn(WebSocketError) + 'static,
) -> Option<WebSocketSink> {
    let ws = WebSocket::open("/ws?mode=json").ok()?;
    let (write, mut read) = ws.split();

    wasm_bindgen_futures::spawn_local(async move {
        while let Some(msg) = read.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    let outbound = serde_json::from_str(&text).unwrap_or(OutboundMessage::Unknown);
                    on_message(outbound);
                }
                Err(err) => {
                    log::error!("Error on the WebSocket: {}", err);
                    on_error(err);
                    break;
                }
                _ => {}
            }
        }
    });

    Some(Rc::new(RefCell::new(write)))
}

pub async fn send_message(sink: &WebSocketSink, inbound: &InboundMessage) {
    if let Ok(message) = serde_json::to_string(inbound) {
        log::info!("Sending message: {}", message);
        let mut sink = sink.borrow_mut();
        let send_result = sink.send(Message::Text(message)).await;

        if let Err(err) = send_result {
            log::error!("Failed to send message: {}", err);
        }
    } else {
        log::error!("Failed to serialize the message");
    }
}
