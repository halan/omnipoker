use futures_util::{SinkExt, StreamExt};
use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    thread,
};
use tokio::{
    net::TcpStream,
    time::{timeout, Duration},
};
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};

pub type WsStream = WebSocketStream<MaybeTlsStream<TcpStream>>;

static PORT: Mutex<u16> = Mutex::new(8080);
// That timeout should considering the compiling time on CI
const TIMEOUT: Duration = Duration::from_secs(30);

pub fn get_port() -> String {
    let mut port = PORT.lock().unwrap();
    *port += 1;
    format!("{}", port)
}

pub struct ServerGuard {
    pub process: Option<Child>,
    logs: Arc<Mutex<Vec<String>>>,
    errs: Arc<Mutex<Vec<String>>>,
}

impl ServerGuard {
    pub fn new() -> Self {
        Self {
            process: None,
            logs: Arc::new(Mutex::new(Vec::new())),
            errs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn start(&mut self, port: &str) {
        if self.process.is_some() {
            panic!("Server is already running!");
        }

        let command = "cargo";
        let addr = format!("127.0.0.1:{}", port);
        let args = vec!["run", "--", addr.as_str()];
        let command_string = format!("{} {}", command, args.join(" "));

        println!("Executing command: {}", command_string);

        self.process = Some(
            Command::new(command)
                .args(&args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Failed to start server"),
        );

        self.start_capture_logs();

        let start_time = tokio::time::Instant::now();
        let max_duration = &TIMEOUT;

        loop {
            let elapsed = start_time.elapsed();
            if elapsed >= *max_duration {
                println!(
                    "Captured errors before timeout:\n{}",
                    self.errs
                        .lock()
                        .unwrap()
                        .drain(..)
                        .collect::<Vec<_>>()
                        .join("\n\t")
                );

                panic!(
                    "Timed out waiting for server to start after {} seconds",
                    &TIMEOUT.as_secs()
                );
            }

            println!("waiting...");
            tokio::time::sleep(Duration::from_millis(500)).await;

            match timeout(
                TIMEOUT,
                connect_async(format!("ws://127.0.0.1:{}/ws", port)),
            )
            .await
            {
                Ok(Ok(_)) => {
                    println!("Server started successfully!");
                    return;
                }
                Ok(Err(_)) => {
                    println!("Connection refused. trying again...");
                }
                Err(_) => {
                    println!("Timeout during connection attempt");
                }
            }
        }
    }

    fn start_capture_logs(&mut self) {
        let process = self
            .process
            .as_mut()
            .expect("Server process is not running");

        let logs = Arc::clone(&self.logs);
        let stdout = process.stdout.take().expect("Failed to capture stdout");

        // Spawn a thread to capture stdout logs
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if let Ok(line) = line {
                    let mut logs = logs.lock().unwrap();
                    logs.push(line);
                }
            }
        });

        let errs = Arc::clone(&self.errs);
        let stderr = process.stderr.take().expect("Failed to capture stderr");

        // Spawn a thread to capture stderr logs
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if let Ok(line) = line {
                    let mut errs = errs.lock().unwrap();
                    errs.push(line);
                }
            }
        });
    }

    pub fn read_logs(&self) -> Vec<String> {
        let mut logs = self.logs.lock().unwrap();
        logs.drain(..).collect()
    }
}

impl Drop for ServerGuard {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            println!("Dropping server process...");
            if let Err(e) = process.kill() {
                println!("Failed to kill process: {:?}", e);
            } else {
                println!("Process killed successfully.");
            }

            if let Err(e) = process.wait() {
                println!("Failed to wait for process: {:?}", e);
            } else {
                println!("Process waited successfully.");
            }
        }
    }
}

pub async fn send_message(ws_stream: &mut WsStream, message: &str) {
    ws_stream
        .send(Message::Text(message.to_string()))
        .await
        .expect("Failed to send message");
}

pub async fn expect_message(receive_message: impl Fn(&str), ws_stream: &mut WsStream) {
    loop {
        match timeout(TIMEOUT, ws_stream.next())
            .await
            .expect("Timed out waiting for message")
            .expect("Failed to read message")
        {
            Ok(Message::Text(text)) => {
                receive_message(&text);
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
