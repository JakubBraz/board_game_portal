use actix_web::{web, HttpRequest, HttpResponse, Responder};
use actix_ws::Message;
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::sync::mpsc;

use crate::games::{AiDifficulty, GameStatus, GameType, PlayerColor};
use crate::lobby::{RoomStatus, SharedState};

#[derive(Deserialize)]
pub struct CreateRoomRequest {
    pub game_type: String,
    pub vs_computer: Option<bool>,
    pub difficulty: Option<String>,
}

#[derive(Deserialize)]
pub struct JoinRoomRequest {
    pub player_id: String,
}

#[derive(Deserialize)]
pub struct WsQuery {
    pub player_id: String,
}

pub async fn create_room(
    state: web::Data<SharedState>,
    body: web::Json<CreateRoomRequest>,
) -> impl Responder {
    let game_type = match GameType::from_str(&body.game_type) {
        Some(gt) => gt,
        None => return HttpResponse::BadRequest().json(json!({"error": "Invalid game type"})),
    };

    let mut st = state.lock().await;
    let room_id = if body.vs_computer.unwrap_or(false) {
        let difficulty = body.difficulty.as_deref()
            .and_then(AiDifficulty::from_str)
            .unwrap_or(AiDifficulty::Normal);
        st.create_room_vs_computer(game_type, difficulty)
    } else {
        st.create_room(game_type)
    };
    let info = st.get_room_info(&room_id).unwrap();
    HttpResponse::Ok().json(json!({
        "room_id": room_id,
        "room": info,
    }))
}

pub async fn list_rooms(state: web::Data<SharedState>) -> impl Responder {
    let st = state.lock().await;
    let rooms = st.list_rooms();
    HttpResponse::Ok().json(json!({ "rooms": rooms }))
}

pub async fn get_room(
    state: web::Data<SharedState>,
    path: web::Path<String>,
) -> impl Responder {
    let room_id = path.into_inner();
    let st = state.lock().await;
    match st.get_room_info(&room_id) {
        Some(info) => HttpResponse::Ok().json(json!({ "room": info })),
        None => HttpResponse::NotFound().json(json!({"error": "Room not found"})),
    }
}

pub async fn ws_handler(
    req: HttpRequest,
    stream: web::Payload,
    state: web::Data<SharedState>,
    path: web::Path<String>,
    query: web::Query<WsQuery>,
) -> actix_web::Result<HttpResponse> {
    let room_id = path.into_inner();
    let player_id = query.player_id.clone();

    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;

    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Register player in the room
    let color = {
        let mut st = state.lock().await;
        let room = match st.rooms.get_mut(&room_id) {
            Some(r) => r,
            None => {
                let _ = session.close(None).await;
                return Ok(response);
            }
        };

        let existing_color = room.player_color(&player_id);
        if let Some(color) = existing_color {
            // Reconnect
            match color {
                PlayerColor::White => room.white_sender = Some(tx.clone()),
                PlayerColor::Black => room.black_sender = Some(tx.clone()),
            }
            color
        } else {
            match room.add_player(player_id.clone()) {
                Some(color) => {
                    match color {
                        PlayerColor::White => room.white_sender = Some(tx.clone()),
                        PlayerColor::Black => room.black_sender = Some(tx.clone()),
                    }
                    color
                }
                None => {
                    let _ = session.close(None).await;
                    return Ok(response);
                }
            }
        }
    };

    // Send initial state
    {
        let st = state.lock().await;
        if let Some(room) = st.rooms.get(&room_id) {
            let diff_str = room.ai_difficulty.map(|d| match d {
                AiDifficulty::Easy => "easy",
                AiDifficulty::Normal => "normal",
                AiDifficulty::Hard => "hard",
            });
            let connected_msg = json!({
                "type": "connected",
                "player_id": player_id,
                "color": match color {
                    PlayerColor::White => "white",
                    PlayerColor::Black => "black",
                },
                "room_status": room.status,
                "vs_computer": room.vs_computer,
                "ai_difficulty": diff_str,
            }).to_string();
            let _ = tx.send(connected_msg);

            let state_msg = room.game_state_message();
            if room.status == RoomStatus::Playing {
                room.broadcast(&state_msg);
                let start_msg = json!({
                    "type": "game_started",
                    "white_player": room.white_player_id,
                    "black_player": room.black_player_id,
                    "vs_computer": room.vs_computer,
                    "ai_difficulty": diff_str,
                }).to_string();
                room.broadcast(&start_msg);
            } else {
                let _ = tx.send(state_msg);
            }
        }
    }

    let state_clone = state.clone();
    let room_id_clone = room_id.clone();
    let player_id_clone = player_id.clone();

    // Spawn outgoing message task
    let mut session_clone = session.clone();
    actix_web::rt::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if session_clone.text(msg).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages
    actix_web::rt::spawn(async move {
        // For vs_computer rooms: if the AI color moves first (e.g. Go/Gomoku start as Black),
        // trigger the initial AI move so the human isn't stuck waiting.
        let needs_initial_ai = {
            let st = state_clone.lock().await;
            st.rooms.get(&room_id_clone).map(|room| {
                room.vs_computer
                    && room.status == RoomStatus::Playing
                    && room.game.current_player() != color
            }).unwrap_or(false)
        };
        if needs_initial_ai {
            apply_ai_move(&state_clone, &room_id_clone).await;
        }

        while let Some(Ok(msg)) = msg_stream.next().await {
            match msg {
                Message::Text(text) => {
                    handle_ws_message(
                        &state_clone,
                        &room_id_clone,
                        &player_id_clone,
                        &color,
                        &text,
                    ).await;
                }
                Message::Ping(bytes) => {
                    let _ = session.pong(&bytes).await;
                }
                Message::Close(_) => {
                    break;
                }
                _ => {}
            }
        }

        // Clean up sender on disconnect
        let mut st = state_clone.lock().await;
        if let Some(room) = st.rooms.get_mut(&room_id_clone) {
            match color {
                PlayerColor::White => room.white_sender = None,
                PlayerColor::Black => room.black_sender = None,
            }
            let msg = json!({
                "type": "opponent_disconnected",
                "color": match color {
                    PlayerColor::White => "white",
                    PlayerColor::Black => "black",
                },
            }).to_string();
            room.broadcast(&msg);
        }
    });

    Ok(response)
}

/// Compute and apply the AI move for a vs_computer room.
/// Releases the lock before the (potentially expensive) AI computation.
async fn apply_ai_move(state: &SharedState, room_id: &str) {
    let info = {
        let st = state.lock().await;
        st.rooms.get(room_id).and_then(|room| {
            if room.vs_computer && room.status == RoomStatus::Playing {
                let game_clone = room.game.clone_box();
                let ai_color = room.game.current_player();
                let difficulty = room.ai_difficulty.unwrap_or(AiDifficulty::Normal);
                Some((game_clone, ai_color, difficulty))
            } else {
                None
            }
        })
    };

    if let Some((game_clone, ai_color, difficulty)) = info {
        let ai_move = tokio::task::spawn_blocking(move || {
            game_clone.get_ai_move(difficulty)
        }).await.ok().flatten();

        if let Some(ai_move) = ai_move {
            let mut st = state.lock().await;
            if let Some(room) = st.rooms.get_mut(room_id) {
                if room.status == RoomStatus::Playing {
                    if let Ok(()) = room.game.make_move(&ai_move, &ai_color) {
                        let state_msg = room.game_state_message();
                        room.broadcast(&state_msg);
                        let game_status = room.game.status();
                        if game_status != GameStatus::Playing {
                            room.status = RoomStatus::Finished;
                            let over_msg = json!({
                                "type": "game_over",
                                "status": game_status,
                            }).to_string();
                            room.broadcast(&over_msg);
                        }
                    }
                }
            }
        }
    }
}

async fn handle_ws_message(
    state: &SharedState,
    room_id: &str,
    _player_id: &str,
    color: &PlayerColor,
    text: &str,
) {
    let msg: Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(_) => {
            send_error(state, room_id, color, "Invalid JSON").await;
            return;
        }
    };

    let msg_type = match msg["type"].as_str() {
        Some(t) => t,
        None => {
            send_error(state, room_id, color, "Missing message type").await;
            return;
        }
    };

    match msg_type {
        "make_move" => {
            let mv = msg["move"].clone();

            // Phase 1: apply human move (lock held briefly)
            let should_apply_ai = {
                let mut st = state.lock().await;
                let room = match st.rooms.get_mut(room_id) {
                    Some(r) => r,
                    None => return,
                };

                if room.status != RoomStatus::Playing {
                    let err = json!({"type": "error", "message": "Game is not in progress"}).to_string();
                    room.send_to_player(color, &err);
                    return;
                }

                match room.game.make_move(&mv, color) {
                    Ok(()) => {
                        let state_msg = room.game_state_message();
                        room.broadcast(&state_msg);

                        let game_status = room.game.status();
                        if game_status != GameStatus::Playing {
                            room.status = RoomStatus::Finished;
                            let over_msg = json!({
                                "type": "game_over",
                                "status": game_status,
                            }).to_string();
                            room.broadcast(&over_msg);
                            false
                        } else {
                            room.vs_computer
                        }
                    }
                    Err(e) => {
                        let err = json!({"type": "error", "message": e}).to_string();
                        room.send_to_player(color, &err);
                        false
                    }
                }
            }; // lock released

            // Phase 2: compute and apply AI move outside the lock
            if should_apply_ai {
                apply_ai_move(state, room_id).await;
            }
        }
        "resign" => {
            let mut st = state.lock().await;
            let room = match st.rooms.get_mut(room_id) {
                Some(r) => r,
                None => return,
            };

            room.status = RoomStatus::Finished;
            let winner = match color {
                PlayerColor::White => "black",
                PlayerColor::Black => "white",
            };
            let msg = json!({
                "type": "game_over",
                "status": format!("{}_won", winner),
                "reason": "resignation",
            }).to_string();
            room.broadcast(&msg);
        }
        "chat" => {
            if let Some(text) = msg["text"].as_str() {
                let chat_color = match color {
                    PlayerColor::White => "white",
                    PlayerColor::Black => "black",
                };
                let chat_msg = json!({
                    "type": "chat",
                    "color": chat_color,
                    "text": &text[..text.len().min(500)],
                }).to_string();
                let st = state.lock().await;
                if let Some(room) = st.rooms.get(room_id) {
                    room.broadcast(&chat_msg);
                }
            }
        }
        "ping" => {
            let st = state.lock().await;
            if let Some(room) = st.rooms.get(room_id) {
                let pong = json!({"type": "pong"}).to_string();
                room.send_to_player(color, &pong);
            }
        }
        _ => {
            send_error(state, room_id, color, "Unknown message type").await;
        }
    }
}

async fn send_error(state: &SharedState, room_id: &str, color: &PlayerColor, msg: &str) {
    let st = state.lock().await;
    if let Some(room) = st.rooms.get(room_id) {
        let err = json!({"type": "error", "message": msg}).to_string();
        room.send_to_player(color, &err);
    }
}
