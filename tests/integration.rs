use helpers::{expect_message, expect_messages, send_message, ServerGuard};
use tokio::time::Duration;
use tokio_tungstenite::connect_async;

mod helpers;

#[tokio::test]
async fn test_integration_planning_poker() {
    let server_url = "ws://127.0.0.1:8080/ws";
    let waiting_time = Duration::from_secs(3);
    let mut server_guard = ServerGuard::new();

    server_guard
        .start("ws://127.0.0.1:8080/ws", waiting_time)
        .await;

    let (mut ws_stream_1, _) = connect_async(server_url)
        .await
        .expect("Failed to connect to WebSocket");

    let (mut ws_stream_2, _) = connect_async(server_url)
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
        "Game started",     // first message
        "Starting service", // listening message
        "connected",        // healthcheck
        "connected",        // Player1
        "connected",        // Player2
        "Someone joined",   // Player1 identified
        "Someone joined",   // Player2 identified
    ];

    assert_eq!(captured_logs.len(), expected_logs.len());

    for (log, expected) in captured_logs.iter().zip(expected_logs.iter()) {
        assert!(
            log.contains(expected),
            "Expected log: '{}', found: '{}'",
            expected,
            log
        );
    }
}
