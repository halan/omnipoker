use futures_util::{SinkExt, StreamExt};
use std::process::{Child, Command};
use tokio::time::{timeout, Duration};
use tokio_tungstenite::{tungstenite::protocol::Message, WebSocketStream};

// Helper to ensure server is stopped after the test
pub struct ServerGuard {
    process: Option<Child>,
}

impl ServerGuard {
    pub fn new() -> Self {
        Self { process: None }
    }

    pub fn start(&mut self) {
        if self.process.is_some() {
            panic!("Server is already running!");
        }

        self.process = Some(
            Command::new("cargo")
                .arg("run")
                .spawn()
                .expect("Failed to start server"),
        );
    }
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.kill(); // Ensure the server is killed
        }
    }
}

// Use a generic type for the WebSocket stream
pub async fn send_message<S>(ws_stream: &mut WebSocketStream<S>, message: &str)
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    ws_stream
        .send(Message::Text(message.to_string()))
        .await
        .expect("Failed to send message");
}

pub async fn expect_message<S>(
    ws_stream: &mut WebSocketStream<S>,
    expected: &str,
    timeout_secs: u64,
) where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let response = timeout(Duration::from_secs(timeout_secs), ws_stream.next())
        .await
        .expect("Timed out waiting for message")
        .expect("Failed to read message")
        .expect("WebSocket message was not valid text");
    assert_eq!(response, expected.into());
}

pub async fn collect_messages<S>(
    ws_stream: &mut WebSocketStream<S>,
    count: usize,
    timeout_secs: u64,
) -> Vec<String>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let mut buffer = Vec::new();
    let mut time_left = timeout_secs;

    while buffer.len() < count && time_left > 0 {
        let start = tokio::time::Instant::now();
        if let Ok(Some(Ok(Message::Text(response)))) =
            timeout(Duration::from_secs(time_left), ws_stream.next()).await
        {
            buffer.push(response);
        }
        time_left = time_left.saturating_sub(start.elapsed().as_secs());
    }

    if buffer.len() < count {
        panic!("Expected {} messages but received {}", count, buffer.len());
    }

    buffer
}
