use helpers::{expect_message, expect_messages, send_message, ServerGuard};
use tokio::time::Duration;
use tokio_tungstenite::connect_async;

mod helpers;

#[tokio::test]
async fn test_integration_planning_poker_json() {
    let port = "8082";
    let server_url = format!("ws://127.0.0.1:{}/ws?mode=json", port);
    let waiting_time = Duration::from_secs(10);
    let mut server_guard = ServerGuard::new();

    server_guard.start(port, waiting_time).await;

    let (mut ws_stream_1, _) = connect_async(server_url.as_str())
        .await
        .expect("Failed to connect to WebSocket");

    let (mut ws_stream_2, _) = connect_async(server_url.as_str())
        .await
        .expect("Failed to connect to WebSocket");

    send_message(&mut ws_stream_1, r#"{"connect":{"nickname":"Player1"}}"#).await;
    expect_message(
        &mut ws_stream_1,
        r#"{"user_list":["Player1"]}"#,
        waiting_time,
    )
    .await;

    send_message(&mut ws_stream_2, r#"{"connect":{"nickname":"Player2"}}"#).await;
    expect_message(
        &mut ws_stream_1,
        r#"{"user_list":["Player1","Player2"]}"#,
        waiting_time,
    )
    .await;

    send_message(&mut ws_stream_1, r#"{"vote": {"value": "1"}}"#).await;
    expect_messages(
        &mut ws_stream_1,
        vec![
            r#"{"your_vote":"1"}"#,                                            // you vote
            r#"{"votes_list":[["Player1","voted"],["Player2","not voted"]]}"#, // votes summary
        ],
        waiting_time,
    )
    .await;

    send_message(&mut ws_stream_2, r#"{"vote": {"value": "2"}}"#).await;
    expect_message(
        &mut ws_stream_1,
        r#"{"votes_list":[["Player1","1"],["Player2","2"]]}"#,
        waiting_time,
    )
    .await;

    send_message(&mut ws_stream_1, r#"{"vote": {"value": "4"}}"#).await;
    expect_message(
        &mut ws_stream_1,
        r#"{"votes_list":[["Player1","not voted"],["Player2","not voted"]]}"#,
        waiting_time,
    )
    .await;

    send_message(&mut ws_stream_1, r#"{"vote": {"value": "?"}}"#).await;
    expect_message(
        &mut ws_stream_1,
        r#"{"votes_list":[["Player1","voted"],["Player2","not voted"]]}"#,
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
            r#"{"user_list":["Player1","Player2"]}"#, // update user list
            r#"{"votes_list":[["Player1","voted"],["Player2","not voted"]]}"#, // votes summary
            r#"{"your_vote":"2"}"#,                   // you vote
            r#"{"votes_list":[["Player1","1"],["Player2","2"]]}"#, // votes summary final
            r#"{"votes_list":[["Player1","not voted"],["Player2","not voted"]]}"#, // votes summary
            r#"{"votes_list":[["Player1","voted"],["Player2","not voted"]]}"#, // votes summary
            r#"{"user_list":["Player2"]}"#,           // update user list
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
        "Game started",               // first message
        "Starting service",           // listening message
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
