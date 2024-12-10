use helpers::{expect_message, get_port, send_message, ServerGuard};
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, tungstenite::Error, MaybeTlsStream, WebSocketStream};

mod helpers;

fn get_server_url() -> (String, String) {
    let port = &get_port();
    (port.to_owned(), format!("ws://127.0.0.1:{}/ws", port))
}

#[tokio::test]
async fn planning_poker() {
    let (port, server_url) = get_server_url();
    let mut server_guard = ServerGuard::new();

    server_guard.start(&port).await;

    let (mut ws_stream_1, _) = connect_async(server_url.as_str())
        .await
        .expect("Failed to connect to WebSocket");

    let (mut ws_stream_2, _) = connect_async(server_url.as_str())
        .await
        .expect("Failed to connect to WebSocket");

    send_message(&mut ws_stream_1, "/join Player1").await;
    expect_message(|text| assert_eq!(text, "Users: Player1"), &mut ws_stream_1).await;

    send_message(&mut ws_stream_2, "/join Player2").await;
    expect_message(
        |text| assert_eq!(text, "Users: Player1, Player2"),
        &mut ws_stream_1,
    )
    .await;

    send_message(&mut ws_stream_1, "1").await;
    expect_message(|text| assert_eq!(text, "You voted: 1"), &mut ws_stream_1).await;
    expect_message(
        |text| assert_eq!(text, "Votes: Player1: voted, Player2: not voted"),
        &mut ws_stream_1,
    )
    .await;

    send_message(&mut ws_stream_2, "2").await;
    expect_message(
        |text| assert_eq!(text, "Votes: Player1: 1, Player2: 2"),
        &mut ws_stream_1,
    )
    .await;

    send_message(&mut ws_stream_1, "4").await;
    expect_message(
        |text| assert_eq!(text, "You voted: not voted"),
        &mut ws_stream_1,
    )
    .await;
    expect_message(
        |text| assert_eq!(text, "Votes: Player1: not voted, Player2: not voted"),
        &mut ws_stream_1,
    )
    .await;

    send_message(&mut ws_stream_1, "?").await;
    expect_message(|text| assert_eq!(text, "You voted: ?"), &mut ws_stream_1).await;
    expect_message(
        |text| assert_eq!(text, "Votes: Player1: voted, Player2: not voted"),
        &mut ws_stream_1,
    )
    .await;

    ws_stream_1
        .close(None)
        .await
        .expect("Failed to close connection");

    expect_message(
        |text| assert_eq!(text, "Users: Player1, Player2"),
        &mut ws_stream_2,
    )
    .await;
    expect_message(
        |text| assert_eq!(text, "Votes: Player1: voted, Player2: not voted"),
        &mut ws_stream_2,
    )
    .await;
    expect_message(|text| assert_eq!(text, "You voted: 2"), &mut ws_stream_2).await;
    expect_message(
        |text| assert_eq!(text, "Votes: Player1: 1, Player2: 2"),
        &mut ws_stream_2,
    )
    .await;
    expect_message(
        |text| assert_eq!(text, "Votes: Player1: not voted, Player2: not voted"),
        &mut ws_stream_2,
    )
    .await;
    expect_message(
        |text| assert_eq!(text, "Votes: Player1: voted, Player2: not voted"),
        &mut ws_stream_2,
    )
    .await;
    expect_message(|text| assert_eq!(text, "Users: Player2"), &mut ws_stream_2).await;

    ws_stream_2
        .close(None)
        .await
        .expect("Failed to close connection");

    let captured_logs = server_guard.read_logs();
    let expected_logs = vec![
        "Starting service", // welcome message
        "",
        "limit of sessions",
        "listening on",
        "directly on a browser",
        "to connect a websocket",
        "",
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
async fn test_away() {
    let port = &get_port();
    let server_url = format!("ws://127.0.0.1:{}/ws", port);
    let mut server_guard = ServerGuard::new();

    server_guard.start(port).await;

    let (mut ws_stream_1, _) = connect_async(server_url.as_str())
        .await
        .expect("Failed to connect to WebSocket");

    let (mut ws_stream_2, _) = connect_async(server_url.as_str())
        .await
        .expect("Failed to connect to WebSocket");

    send_message(&mut ws_stream_1, "/join Player1").await;
    expect_message(|text| assert_eq!(text, "Users: Player1"), &mut ws_stream_1).await;

    send_message(&mut ws_stream_2, "/join Player2").await;
    expect_message(
        |text| assert_eq!(text, "Users: Player1, Player2"),
        &mut ws_stream_1,
    )
    .await;

    send_message(&mut ws_stream_1, "/setaway").await;
    expect_message(|text| assert_eq!(text, "You are away"), &mut ws_stream_1).await;
    expect_message(|text| assert_eq!(text, "Users: Player2"), &mut ws_stream_1).await;

    send_message(&mut ws_stream_2, "2").await;
    expect_message(
        |text| assert_eq!(text, "Votes: Player2: 2"),
        &mut ws_stream_1,
    )
    .await;

    send_message(&mut ws_stream_1, "/setback").await;
    expect_message(|text| assert_eq!(text, "You are active"), &mut ws_stream_1).await;
    expect_message(
        |text| assert_eq!(text, "Users: Player1, Player2"),
        &mut ws_stream_1,
    )
    .await;

    send_message(&mut ws_stream_1, "1").await;
    expect_message(|text| assert_eq!(text, "You voted: 1"), &mut ws_stream_1).await;
    expect_message(
        |text| assert_eq!(text, "Votes: Player1: voted, Player2: not voted"),
        &mut ws_stream_1,
    )
    .await;

    send_message(&mut ws_stream_2, "1").await;
    expect_message(
        |text| assert_eq!(text, "Votes: Player1: 1, Player2: 1"),
        &mut ws_stream_1,
    )
    .await;
}

#[tokio::test]
async fn test_server_limit() {
    let (port, server_url) = get_server_url();
    let mut server_guard = ServerGuard::new();

    server_guard.start(&port).await;

    const SERVER_LIMIT: usize = 15;

    let mut connections: [Option<WebSocketStream<MaybeTlsStream<TcpStream>>>; SERVER_LIMIT - 1] =
        Default::default();
    for i in 0..SERVER_LIMIT - 1 {
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
