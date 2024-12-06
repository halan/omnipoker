# omnipoker

A simple planning poker server

## Warning

⚠️ **Experimental Project**

This is just my very first Rust experience. I'm writing it as a way to stablish the knowledge that I just acquired learning "The Rust Programming Language". Anyway, feel free to copy, learn, compile and use whatever you want. I also would appreciate a PR.

## Quick Instructions

### Text mode

To interact with the server, you can install wscat.

Connect using: `wscat http://127.0.0.1/ws`

Next, identify yourself with: `/join nickname`

From this point on, any input you provide will be considered a vote. The valid votes are: ?, 1, 2, 3, 5, 8 and 13.

Once everyone has voted, all votes will be revealed. The server will not dictate what to do with the votes.

### Browser mode

By the fault, the server will serve an UI by the :8080.

https://github.com/user-attachments/assets/beb7abc2-05ac-4eec-90f0-dc41a808c525

