# Omnipoker

A lightweight planning poker server.

## ⚠️ Warning

This is an **experimental project** and my very first experience with Rust. I created it to solidify my understanding of the concepts learned from *The Rust Programming Language*. Feel free to copy, explore, compile, and use this project as you wish. Contributions are welcome via pull requests!

## Features

- Text-based interaction via WebSocket.
- A simple web-based interface for voting.
- Supports planning poker votes with values: `?`, `1`, `2`, `3`, `5`, `8`, and `13`.

## Getting Started

### Text Mode

To interact with the server using a WebSocket client, you can install `wscat`:

1. Connect to the server:
   ```bash
   wscat -c ws://127.0.0.1/ws
   ```
2. Identify yourself:
   ```bash
   /join <nickname>
   ```
3. Start voting! Enter one of the valid vote values (`?`, `1`, `2`, `3`, `5`, `8`, `13`).

Once everyone has voted, all votes will be revealed. The server does not enforce any further actions based on the results.

### Browser Mode

By default, the server hosts a web-based user interface on port `8080`.

Access it at:
```
http://127.0.0.1:8080
```

https://github.com/user-attachments/assets/beb7abc2-05ac-4eec-90f0-dc41a808c525

## Contributing

Contributions are highly appreciated! If you’d like to improve this project, feel free to open a pull request or report any issues you encounter.
