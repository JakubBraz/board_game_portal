# Board Games Portal — Communication Protocol

## Overview

The server exposes two interfaces:

- **REST API** — room management (create, list, inspect)
- **WebSocket** — real-time game play

All WebSocket messages are JSON text frames. The server runs on `http://127.0.0.1:8080` by default.

---

## REST API

### POST /api/rooms — Create a room

**Request body:**
```json
{
  "game_type": "chess",        // "chess" | "checkers" | "go" | "gomoku"
  "vs_computer": true,         // optional, default false
  "difficulty": "normal"       // optional: "easy" | "normal" | "hard"
}
```

**Response:**
```json
{
  "room_id": "a3f9c12e",
  "room": {
    "id": "a3f9c12e",
    "game_type": "chess",
    "status": "waiting",       // "waiting" | "playing" | "finished"
    "white_player": null,
    "black_player": null,
    "created_at": 1709500000
  }
}
```

---

### GET /api/rooms — List open rooms

Returns only human-vs-human rooms that are not finished.

**Response:**
```json
{
  "rooms": [
    {
      "id": "a3f9c12e",
      "game_type": "chess",
      "status": "waiting",
      "white_player": "player-uuid",
      "black_player": null,
      "created_at": 1709500000
    }
  ]
}
```

---

### GET /api/rooms/:id — Get a specific room

**Response (found):**
```json
{
  "room": { "id": "a3f9c12e", "game_type": "chess", "status": "playing", ... }
}
```

**Response (not found):** HTTP 404
```json
{ "error": "Room not found" }
```

---

## WebSocket Connection

```
WS /ws/:room_id?player_id=<uuid>
```

The `player_id` is generated client-side (stored in `sessionStorage`) and sent as a query parameter. It is used to re-associate a player with their color on reconnect.

The server closes the connection immediately if:
- The room does not exist
- The room already has two human players and the connecting `player_id` is unknown

---

## WebSocket Message Reference

### Messages sent by the CLIENT

#### make_move
Submits a move. The payload of `move` is game-specific (see below).

```json
{ "type": "make_move", "move": { ... } }
```

Move payloads per game:

| Game     | Payload |
|----------|---------|
| Chess    | `{ "from": [row, col], "to": [row, col], "promotion": "queen" }` — promotion is optional |
| Checkers | `{ "from": [row, col], "to": [row, col] }` |
| Go       | `{ "row": 3, "col": 4, "pass": false }` |
| Gomoku   | `{ "row": 3, "col": 4 }` |

Board coordinates: `(0, 0)` is top-left. For chess, row 0 = rank 1 (white's back rank).

---

#### resign
Immediately ends the game; the opponent wins.

```json
{ "type": "resign" }
```

---

#### chat
Sends a chat message (max 500 characters). Broadcast to both players.

```json
{ "type": "chat", "text": "Good game!" }
```

---

#### ping
Application-level keepalive. Server responds with `pong`.

```json
{ "type": "ping" }
```

---

### Messages sent by the SERVER

#### connected
First message sent to a client on connection (or reconnection). Sent only to the connecting player.

```json
{
  "type": "connected",
  "player_id": "550e8400-e29b-41d4-a716-446655440000",
  "color": "white",
  "room_status": "waiting",
  "vs_computer": false,
  "ai_difficulty": null
}
```

`color` is either `"white"` or `"black"`. `ai_difficulty` is `null` for human games, or `"easy"` / `"normal"` / `"hard"` for vs-computer rooms.

---

#### game_state
Sent to **both** players after every move (human or AI), and to the connecting player alone while the room is still in `waiting` status.

```json
{
  "type": "game_state",
  "room_id": "a3f9c12e",
  "room_status": "playing",
  "white_player": "uuid-1",
  "black_player": "uuid-2",
  "vs_computer": false,
  "ai_difficulty": null,
  "state": { ... }
}
```

The `state` object is game-specific (see Game State Payloads below).

---

#### game_started
Broadcast to **both** players when the second player joins and the game begins.

```json
{
  "type": "game_started",
  "white_player": "uuid-1",
  "black_player": "uuid-2",
  "vs_computer": false,
  "ai_difficulty": null
}
```

---

#### game_over
Broadcast to **both** players when the game ends (checkmate, resignation, draw, etc.).

```json
{
  "type": "game_over",
  "status": "white_won",
  "reason": "resignation"
}
```

`status` values: `"white_won"` | `"black_won"` | `"draw"`

`reason` is only present on resignation: `"resignation"`.

---

#### chat
Echoed to **both** players when either sends a chat message.

```json
{
  "type": "chat",
  "color": "white",
  "text": "Good game!"
}
```

---

#### pong
Response to a client `ping`. Sent only to the requesting player.

```json
{ "type": "pong" }
```

---

#### opponent_disconnected
Sent to the remaining connected player when the other player's WebSocket closes.

```json
{
  "type": "opponent_disconnected",
  "color": "white"
}
```

If **both** players disconnect, the room is deleted from the server (no message is sent).

---

#### error
Sent only to the player who caused the error. Occurs on invalid JSON, unknown message type, illegal move, or moving out of turn.

```json
{ "type": "error", "message": "Illegal move" }
```

---

## Game State Payloads

### Chess `state`

```json
{
  "type": "chess",
  "board": [[...], ...],         // 8×8 array; null or { "type": "pawn", "color": "white" }
  "current_turn": "white",
  "status": "playing",           // "playing" | "white_won" | "black_won" | "draw"
  "castling": {
    "white_king_side": true,
    "white_queen_side": true,
    "black_king_side": false,
    "black_queen_side": true
  },
  "en_passant": [4, 5],          // null or [row, col] of the capture target square
  "legal_moves": [
    { "from": [1, 4], "to": [[3, 4], [2, 4]] }
  ],
  "last_move": [[1, 4], [3, 4]], // null or [[fromRow, fromCol], [toRow, toCol]]
  "in_check": false,
  "full_move_number": 1
}
```

Piece types: `"king"` `"queen"` `"rook"` `"bishop"` `"knight"` `"pawn"`

---

### Checkers `state`

```json
{
  "type": "checkers",
  "board": [[...], ...],         // 8×8; null or { "color": "black", "is_king": false }
  "current_turn": "black",
  "status": "playing",
  "must_continue_from": null,    // [row, col] during a multi-jump sequence
  "valid_moves": [
    { "from": [5, 2], "to": [[4, 3]], "must_jump": false }
  ]
}
```

---

### Go `state`

```json
{
  "type": "go",
  "size": 19,
  "board": [[...], ...],         // 19×19; null | "black" | "white"
  "current_turn": "black",
  "status": "playing",
  "captures_black": 3,
  "captures_white": 1,
  "ko_point": null,              // null or [row, col]
  "consecutive_passes": 0,
  "move_count": 12,
  "score_black": 0.0,
  "score_white": 0.0
}
```

---

### Gomoku `state`

```json
{
  "type": "gomoku",
  "size": 15,
  "board": [[...], ...],         // 15×15; null | "black" | "white"
  "current_turn": "black",
  "status": "playing",
  "move_count": 5,
  "last_move": [7, 7]            // null or [row, col]
}
```

---

## Connection Lifecycle Diagrams

### Human vs Human — full game

```
Client A (White)                  Server                   Client B (Black)
     |                               |                            |
     |-- POST /api/rooms ----------->|                            |
     |<- { room_id, room } ----------|                            |
     |                               |                            |
     |== WS /ws/:room_id?player_id= =|                            |
     |<- connected (color=white,     |                            |
     |             status=waiting)   |                            |
     |<- game_state (to A only) -----|                            |
     |                               |== WS /ws/:room_id?player_id=
     |                               |<- connected (color=black,  |
     |                               |             status=playing) |
     |<- game_state (broadcast) -----|--- game_state (broadcast) ->|
     |<- game_started (broadcast) ---|--- game_started (broadcast)->|
     |                               |                            |
     |-- make_move ----------------->|                            |
     |<- game_state (broadcast) -----|--- game_state (broadcast) ->|
     |                               |                            |
     |                               |<------------ make_move ----|
     |<- game_state (broadcast) -----|--- game_state (broadcast) ->|
     |                               |                            |
     |-- make_move ----------------->|  (checkmate)               |
     |<- game_state (broadcast) -----|--- game_state (broadcast) ->|
     |<- game_over  (broadcast) -----|--- game_over  (broadcast) ->|
     |                               |                            |
```

---

### Human vs Computer

```
Client (Human)                    Server
     |                               |
     |-- POST /api/rooms ----------->|
     |   { vs_computer: true,        |
     |     difficulty: "normal" }    |
     |<- { room_id, room } ----------|
     |                               |
     |== WS /ws/:room_id?player_id= =|
     |<- connected (color=white,     |
     |             vs_computer=true) |
     |<- game_state (to player) -----|
     |<- game_started (to player) ---|
     |                               |
     |-- make_move ----------------->|
     |<- game_state (after human) ---|
     |           [AI computes move]  |
     |<- game_state (after AI) ------|
     |                               |
     |-- resign ----------------------|
     |<- game_over ------------------|
     |                               |
```

*For games where Black moves first (Go, Gomoku) and the human is assigned White, the server fires the AI's first move automatically right after the player connects, before any human move is made.*

---

### Reconnection

```
Client                            Server
     |                               |
     |== WS /ws/:room_id?player_id= =|   (player_id matches existing slot)
     |                               |
     |<- connected ------------------|   (same color re-assigned)
     |<- game_state (current state) -|   (full board state, not a diff)
     |                               |
     |         ... game resumes ...  |
     |                               |
```

---

### Disconnect / Room Cleanup

```
Client A                          Server                   Client B
     |                               |                            |
     |=X= (connection closed) ======>|                            |
     |                               |--- opponent_disconnected ->|
     |                               |                            |
     |  (if Client B also closes)    |                            |
     |                               |<==X (connection closed) ===|
     |                               |                            |
     |                   [room deleted from server]               |
     |                               |                            |
```

---

## Notes on Replay Support

All game outcomes are fully determined by the sequence of `make_move` messages sent by players. To support replays:

1. Store each `make_move` payload alongside its `player_id`, `color`, and a timestamp.
2. To replay: recreate the initial game state and apply the moves in order.

No server-side changes are required now — the inputs are already self-contained. The `full_move_number` (chess) and `move_count` (Go, Gomoku) fields in game state can be used to verify replay integrity.
