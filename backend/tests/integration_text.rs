use helpers::{expect_message, expect_messages, send_message, ServerGuard};
use tokio::{net::TcpStream, time::Duration};
use tokio_tungstenite::{connect_async, tungstenite::Error, MaybeTlsStream, WebSocketStream};

mod helpers;

#[tokio::test]
async fn test_integration_planning_poker() {
    let port = "8081";
    let server_url = format!("ws://127.0.0.1:{}/ws", port);
    let waiting_time = Duration::from_secs(10);
    let mut server_guard = ServerGuard::new();

    server_guard.start(port, waiting_time).await;

    let (mut ws_stream_1, _) = connect_async(server_url.as_str())
        .await
        .expect("Failed to connect to WebSocket");

    let (mut ws_stream_2, _) = connect_async(server_url.as_str())
        .await
        .expect("Failed to connect to WebSocket");

    send_message(&mut ws_stream_1, "/join Player1").await;
    expect_message(&mut ws_stream_1, "Users: Player1", waiting_time).await;

    send_message(&mut ws_stream_2, "/join Player2").await;
    expect_message(&mut ws_stream_1, "Users: Player1, Player2", waiting_time).await;

    send_message(&mut ws_stream_1, "1").await;
    expect_messages(
        &mut ws_stream_1,
        vec![
            "You voted: 1",                              // you vote
            "Votes: Player1: voted, Player2: not voted", // votes summary
        ],
        waiting_time,
    )
    .await;

    send_message(&mut ws_stream_2, "2").await;
    expect_message(
        &mut ws_stream_1,
        "Votes: Player1: 1, Player2: 2",
        waiting_time,
    )
    .await;

    send_message(&mut ws_stream_1, "4").await;
    expect_messages(
        &mut ws_stream_1,
        vec![
            "You voted: not voted",
            "Votes: Player1: not voted, Player2: not voted",
        ],
        waiting_time,
    )
    .await;

    send_message(&mut ws_stream_1, "?").await;
    expect_messages(
        &mut ws_stream_1,
        vec!["You voted: ?", "Votes: Player1: voted, Player2: not voted"],
        waiting_time,
    )
    .await;

    ws_stream_1
        .close(None)
        .await
        .expect("Failed to close connection");

    expect_messages(
        &mut ws_stream_2,
        vec![
            "Users: Player1, Player2",                       // update user list
            "Votes: Player1: voted, Player2: not voted",     // votes summary
            "You voted: 2",                                  // you vote
            "Votes: Player1: 1, Player2: 2",                 // votes summary final
            "Votes: Player1: not voted, Player2: not voted", // votes summary
            "Votes: Player1: voted, Player2: not voted",     // votes summary
            "Users: Player2",                                // update user list
        ],
        waiting_time,
    )
    .await;

    ws_stream_2
        .close(None)
        .await
        .expect("Failed to close connection");

    let captured_logs = server_guard.read_logs();
    let expected_logs = vec![
        "Starting service",           // listening message
        "Game started",               // first message
        "User identified: Player1",   // Player1 identified
        "User identified: Player2",   // Player2 identified
        "User disconnected: Player1", // Player1 disconnected
    ];

    assert_eq!(captured_logs.len(), expected_logs.len());

    for (log, expected) in captured_logs.iter().zip(expected_logs.iter()) {
        assert!(
            log.contains(expected),
            "Captured logs:\n{}\n\nexpected:\n{}",
            captured_logs.join("\n"),
            expected_logs.join("\n")
        );
    }
}

#[tokio::test]
async fn test_server_limit() {
    let port = "8083";
    let server_url = format!("ws://127.0.0.1:{}/ws", port);
    let waiting_time = Duration::from_secs(10);
    let mut server_guard = ServerGuard::new();

    server_guard.start(port, waiting_time).await;

    const SERVER_LIMIT: usize = 15;

    let mut connections: [Option<WebSocketStream<MaybeTlsStream<TcpStream>>>; SERVER_LIMIT - 1] =
        Default::default();
    for i in 0..(SERVER_LIMIT - 1) {
        let (ws_stream, _) = connect_async(server_url.as_str())
            .await
            .expect("Failed to connect to WebSocket");
        connections[i] = Some(ws_stream);
    }

    let result = connect_async(server_url.as_str()).await;

    // connection 15
    assert!(result.is_ok(), "Unexpected result: {:?}", result);

    // connection 16 - should fail
    match connect_async(server_url).await {
        Err(Error::Http(response)) => {
            assert_eq!(
                response.status(),
                actix_web::http::StatusCode::TOO_MANY_REQUESTS
            );
        }
        result => {
            panic!("Unexpected result: {:?}", result);
        }
    }
}
