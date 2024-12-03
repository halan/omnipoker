use helpers::{expect_message, send_message, ServerGuard};
use serde_json::json;
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

    send_message(
        &mut ws_stream_1,
        &json!({"connect": {"nickname": "Player1"}}).to_string(),
    )
    .await;

    expect_message(
        |text| assert_eq!(&text, &json!({"user_list": ["Player1"]}).to_string(),),
        &mut ws_stream_1,
        waiting_time,
    )
    .await;

    send_message(
        &mut ws_stream_2,
        &json!({"connect": {"nickname": "Player2"}}).to_string(),
    )
    .await;

    expect_message(
        |text| {
            assert_eq!(
                &text,
                &json!({"user_list":["Player1","Player2"]}).to_string()
            )
        },
        &mut ws_stream_1,
        waiting_time,
    )
    .await;

    send_message(
        &mut ws_stream_1,
        &json!({"vote": {"value": "1"}}).to_string(),
    )
    .await;

    expect_message(
        |text| assert_eq!(&text, &json!({"your_vote": "1"}).to_string()),
        &mut ws_stream_1,
        waiting_time,
    )
    .await;

    expect_message(
        |text| {
            assert_eq!(
                &text,
                &json!({"votes_status": [
                    ["Player1", "voted"],
                    ["Player2", "not voted"],
                ]})
                .to_string(),
            )
        },
        &mut ws_stream_1,
        waiting_time,
    )
    .await;

    send_message(
        &mut ws_stream_2,
        &json!({"vote": {"value": "2"}}).to_string(),
    )
    .await;

    expect_message(
        |text| {
            assert_eq!(
                &text,
                &json!({"votes_result": [["Player1", "1"], ["Player2", "2"]]}).to_string()
            )
        },
        &mut ws_stream_1,
        waiting_time,
    )
    .await;

    send_message(
        &mut ws_stream_1,
        &json!({"vote": {"value": "4"}}).to_string(),
    )
    .await;

    expect_message(
        |text| assert_eq!(&text, &json!({"your_vote": "not voted"}).to_string()),
        &mut ws_stream_1,
        waiting_time,
    )
    .await;

    expect_message(
        |text| {
            assert_eq!(
                &text,
                &json!({"votes_status": [
                    ["Player1", "not voted"],
                    ["Player2", "not voted"],
                ]})
                .to_string()
            )
        },
        &mut ws_stream_1,
        waiting_time,
    )
    .await;

    send_message(
        &mut ws_stream_1,
        &json!({"vote": {"value": "?"}}).to_string(),
    )
    .await;

    expect_message(
        |text| assert_eq!(&text, &json!({"your_vote": "?"}).to_string()),
        &mut ws_stream_1,
        waiting_time,
    )
    .await;

    expect_message(
        |text| {
            assert_eq!(
                &text,
                &json!({"votes_status": [
                    ["Player1", "voted"],
                    ["Player2", "not voted"],
                ]})
                .to_string()
            )
        },
        &mut ws_stream_1,
        waiting_time,
    )
    .await;

    ws_stream_1
        .close(None)
        .await
        .expect("Failed to close connection");

    expect_message(
        |text| {
            assert_eq!(
                &text,
                &json!({"user_list":["Player1", "Player2"]}).to_string()
            )
        },
        &mut ws_stream_2,
        waiting_time,
    )
    .await;

    expect_message(
        |text| {
            assert_eq!(
                &text,
                &json!({"votes_status": [["Player1", "voted"], ["Player2", "not voted"]]})
                    .to_string()
            )
        },
        &mut ws_stream_2,
        waiting_time,
    )
    .await;

    expect_message(
        |text| assert_eq!(&text, &json!({"your_vote": "2"}).to_string()),
        &mut ws_stream_2,
        waiting_time,
    )
    .await;

    expect_message(
        |text| {
            assert_eq!(
                &text,
                &json!({"votes_result": [["Player1", "1"], ["Player2", "2"]]}).to_string()
            )
        },
        &mut ws_stream_2,
        waiting_time,
    )
    .await;

    expect_message(
        |text| {
            assert_eq!(
                &text,
                &json!({"votes_status": [["Player1", "not voted"], ["Player2", "not voted"]]})
                    .to_string()
            )
        },
        &mut ws_stream_2,
        waiting_time,
    )
    .await;

    expect_message(
        |text| {
            assert_eq!(
                &text,
                &json!({"votes_status": [["Player1", "voted"], ["Player2", "not voted"]]})
                    .to_string()
            )
        },
        &mut ws_stream_2,
        waiting_time,
    )
    .await;

    expect_message(
        |text| assert_eq!(&text, &json!({"user_list": ["Player2"]}).to_string()),
        &mut ws_stream_2,
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

    assert_eq!(
        captured_logs.len(),
        expected_logs.len(),
        "Captured logs:\n{}",
        captured_logs.join("\n")
    );

    for (log, expected) in captured_logs.iter().zip(expected_logs.iter()) {
        assert!(
            log.contains(expected),
            "Captured logs:\n{}\n\nexpected:\n{}",
            captured_logs.join("\n"),
            expected_logs.join("\n")
        );
    }
}
