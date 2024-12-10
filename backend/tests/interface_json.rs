use helpers::{expect_message, get_port, send_message, ServerGuard};
use serde_json::json;
use tokio_tungstenite::connect_async;

mod helpers;

fn get_server_url() -> (String, String) {
    let port = &get_port();
    (
        port.to_owned(),
        format!("ws://127.0.0.1:{}/ws?mode=json", port),
    )
}

#[tokio::test]
async fn planning_poker_json() {
    let (port, server_url) = get_server_url();
    let mut server_guard = ServerGuard::new();

    server_guard.start(&port).await;

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
    )
    .await;

    expect_message(
        |text| assert_eq!(&text, &json!({"your_vote": "2"}).to_string()),
        &mut ws_stream_2,
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
    )
    .await;

    expect_message(
        |text| assert_eq!(&text, &json!({"user_list": ["Player2"]}).to_string()),
        &mut ws_stream_2,
    )
    .await;

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
