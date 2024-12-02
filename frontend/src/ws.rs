use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message};
pub use shared::{InboundMessage, OutboundMessage};
use std::{cell::RefCell, rc::Rc};
pub type WebSocketSink = Rc<RefCell<SplitSink<WebSocket, Message>>>;

pub fn connect_websocket(
    url: &str,
    on_message: impl Fn(OutboundMessage) + 'static,
) -> Option<WebSocketSink> {
    let ws = WebSocket::open(url).ok()?;
    let (write, mut read) = ws.split();

    wasm_bindgen_futures::spawn_local(async move {
        while let Some(Ok(Message::Text(text))) = read.next().await {
            let outbound = serde_json::from_str(&text).unwrap_or(OutboundMessage::Unknown);
            log::info!("Received message: {:?}", outbound);
            log::info!("Message: {:?}", text);
            on_message(outbound);
        }
    });

    Some(Rc::new(RefCell::new(write)))
}

pub async fn send_message(sink: &WebSocketSink, message: &InboundMessage) {
    let mut sink = sink.borrow_mut();
    if let Ok(message) = serde_json::to_string(message) {
        log::info!("Sending message: {}", message);
        let _ = sink.send(Message::Text(message)).await;
    } else {
        todo!();
    }
}
