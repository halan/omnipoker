# omnipoker
A simple planning poker server

## Quick Instructions

To interact with the server, you can install wscat.

Connect using: `wscat http://127.0.0.1/ws`

Next, identify yourself with: `/join nickname`

From this point on, any input you provide will be considered a vote. The valid votes are: ?, 1, 2, 3, 5, 8 and 13.

Once everyone has voted, all votes will be revealed. The server will not dictate what to do with the votes.
 