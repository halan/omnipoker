use tokio::time::Duration;
use tokio_tungstenite::connect_async;

mod helpers;

use helpers::{collect_messages, expect_message, send_message, ServerGuard};

#[tokio::test]
async fn test_integration_websocket() {
    let server_url = "ws://127.0.0.1:8080/ws";
    let waiting_time = 1; // second
    let mut server_guard = ServerGuard::new();

    server_guard.start();

    tokio::time::sleep(Duration::from_secs(waiting_time)).await; // 😩

    let (mut ws_stream_1, _) = connect_async(server_url)
        .await
        .expect("Failed to connect to WebSocket");

    let (mut ws_stream_2, _) = connect_async(server_url)
        .await
        .expect("Failed to connect to WebSocket");

    send_message(&mut ws_stream_1, "/join Player1").await;
    expect_message(&mut ws_stream_1, "Users: Player1", waiting_time).await;

    send_message(&mut ws_stream_2, "/join Player2").await;
    let responses = collect_messages(&mut ws_stream_1, 1, waiting_time).await;
    assert_eq!(responses[0], "Users: Player1, Player2");

    send_message(&mut ws_stream_1, "1").await;
    let responses = collect_messages(&mut ws_stream_1, 2, waiting_time).await;
    assert_eq!(responses[0], "You voted: 1");
    assert_eq!(responses[1], "Votes: Player1: voted, Player2: not voted");

    send_message(&mut ws_stream_2, "2").await;
    expect_message(
        &mut ws_stream_1,
        "Votes: Player1: 1, Player2: 2",
        waiting_time,
    )
    .await;

    send_message(&mut ws_stream_1, "4").await;
    expect_message(
        &mut ws_stream_1,
        "Votes: Player1: not voted, Player2: not voted",
        waiting_time,
    )
    .await;

    send_message(&mut ws_stream_1, "?").await;
    expect_message(
        &mut ws_stream_1,
        "Votes: Player1: voted, Player2: not voted",
        waiting_time,
    )
    .await;

    ws_stream_1
        .close(None)
        .await
        .expect("Failed to close connection");

    ws_stream_2
        .close(None)
        .await
        .expect("Failed to close connection");
}
