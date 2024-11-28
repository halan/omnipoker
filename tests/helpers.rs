use futures_util::{SinkExt, StreamExt};
use std::process::{Child, Command};
use tokio::time::{timeout, Duration};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message, WebSocketStream};

// Helper to ensure server is stopped after the test
pub struct ServerGuard {
    process: Option<Child>,
}

impl ServerGuard {
    pub fn new() -> Self {
        Self { process: None }
    }

    pub async fn start(&mut self, server_url: &str, timeout_duration: Duration) {
        if self.process.is_some() {
            panic!("Server is already running!");
        }

        self.process = Some(
            Command::new("cargo")
                .arg("run")
                .spawn()
                .expect("Failed to start server"),
        );

        let start_time = tokio::time::Instant::now();
        let max_duration = timeout_duration;

        log::info!("Waiting for server to start...");
        loop {
            let elapsed = start_time.elapsed();
            if elapsed >= max_duration {
                panic!(
                    "Timed out waiting for server to start after {} seconds",
                    timeout_duration.as_secs()
                );
            }

            match timeout(Duration::from_secs(1), connect_async(server_url)).await {
                Ok(Ok(_)) => {
                    log::info!("Server started successfully!");
                    return;
                }
                Ok(Err(err)) => {
                    log::warn!("Connection failed: {:?}", err);
                }
                Err(_) => {
                    log::warn!("Timeout during connection attempt");
                }
            }

            tokio::time::sleep(Duration::from_millis(500)).await; // Pequena pausa
        }
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
    timeout_duration: Duration,
) where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    loop {
        let response = timeout(timeout_duration, ws_stream.next())
            .await
            .expect("Timed out waiting for message")
            .expect("Failed to read message");

        match response {
            Ok(Message::Text(text)) => {
                assert_eq!(text, expected);
                return;
            }
            Ok(Message::Ping(_)) => {
                log::debug!("Ignoring Ping message");
                continue;
            }
            Ok(other) => {
                panic!("Unexpected WebSocket message: {:?}", other);
            }
            _ => panic!("Unexpected WebSocket message"),
        }
    }
}

pub async fn collect_messages<S>(
    ws_stream: &mut WebSocketStream<S>,
    count: usize,
    timeout_duration: Duration,
) -> Vec<String>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let mut buffer = Vec::new();
    let start_time = tokio::time::Instant::now();

    while buffer.len() < count {
        let elapsed = start_time.elapsed();
        if elapsed >= timeout_duration {
            break;
        }

        let remaining = timeout_duration - elapsed;

        if let Ok(Some(Ok(Message::Text(response)))) = timeout(remaining, ws_stream.next()).await {
            buffer.push(response);
        }
    }

    if buffer.len() < count {
        panic!("Expected {} messages but received {}", count, buffer.len());
    }

    buffer
}
