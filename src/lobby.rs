use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use uuid::Uuid;
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::games::{create_game, AiDifficulty, GameLogic, GameType, PlayerColor};

pub type Sender = mpsc::UnboundedSender<String>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomInfo {
    pub id: String,
    pub game_type: String,
    pub status: RoomStatus,
    pub white_player: Option<String>,
    pub black_player: Option<String>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RoomStatus {
    Waiting,   // Waiting for second player
    Playing,   // Game in progress
    Finished,  // Game over
}

pub struct Room {
    pub id: String,
    pub game_type: GameType,
    pub game: Box<dyn GameLogic>,
    pub white_player_id: Option<String>,
    pub black_player_id: Option<String>,
    pub white_sender: Option<Sender>,
    pub black_sender: Option<Sender>,
    pub status: RoomStatus,
    pub created_at: u64,
    pub vs_computer: bool,
    pub ai_difficulty: Option<AiDifficulty>,
}

impl Room {
    pub fn new(id: String, game_type: GameType) -> Self {
        let game = create_game(&game_type);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Room {
            id,
            game_type,
            game,
            white_player_id: None,
            black_player_id: None,
            white_sender: None,
            black_sender: None,
            status: RoomStatus::Waiting,
            created_at: now,
            vs_computer: false,
            ai_difficulty: None,
        }
    }

    pub fn new_vs_computer(id: String, game_type: GameType, difficulty: AiDifficulty) -> Self {
        let mut room = Room::new(id, game_type);
        room.vs_computer = true;
        room.ai_difficulty = Some(difficulty);
        room
    }

    pub fn to_info(&self) -> RoomInfo {
        RoomInfo {
            id: self.id.clone(),
            game_type: format!("{:?}", self.game_type).to_lowercase(),
            status: self.status.clone(),
            white_player: self.white_player_id.clone(),
            black_player: self.black_player_id.clone(),
            created_at: self.created_at,
        }
    }

    pub fn broadcast(&self, msg: &str) {
        if let Some(tx) = &self.white_sender {
            let _ = tx.send(msg.to_string());
        }
        if let Some(tx) = &self.black_sender {
            let _ = tx.send(msg.to_string());
        }
    }

    pub fn send_to_player(&self, color: &PlayerColor, msg: &str) {
        let tx = match color {
            PlayerColor::White => &self.white_sender,
            PlayerColor::Black => &self.black_sender,
        };
        if let Some(tx) = tx {
            let _ = tx.send(msg.to_string());
        }
    }

    pub fn player_color(&self, player_id: &str) -> Option<PlayerColor> {
        if self.white_player_id.as_deref() == Some(player_id) {
            Some(PlayerColor::White)
        } else if self.black_player_id.as_deref() == Some(player_id) {
            Some(PlayerColor::Black)
        } else {
            None
        }
    }

    pub fn add_player(&mut self, player_id: String) -> Option<PlayerColor> {
        let white_free = self.white_player_id.is_none();
        let black_free = self.black_player_id.is_none();

        if white_free && black_free {
            // Use the room ID (derived from a random UUID) as entropy.
            // UUID hex digits are uniformly distributed over 0-9/a-f,
            // exactly 8 of 16 characters have an even byte value → true 50/50.
            // This avoids Windows clock-resolution issues (100 ns granularity
            // makes time-based % 2 always even).
            let is_white = self.id.as_bytes()
                .first()
                .map(|&b| b % 2 == 0)
                .unwrap_or(true);
            if is_white {
                self.white_player_id = Some(player_id);
                if self.vs_computer { self.status = RoomStatus::Playing; }
                Some(PlayerColor::White)
            } else {
                self.black_player_id = Some(player_id);
                if self.vs_computer { self.status = RoomStatus::Playing; }
                Some(PlayerColor::Black)
            }
        } else if white_free && !self.vs_computer {
            self.white_player_id = Some(player_id);
            self.status = RoomStatus::Playing;
            Some(PlayerColor::White)
        } else if black_free && !self.vs_computer {
            self.black_player_id = Some(player_id);
            self.status = RoomStatus::Playing;
            Some(PlayerColor::Black)
        } else {
            None
        }
    }

    fn difficulty_str(&self) -> Option<&'static str> {
        self.ai_difficulty.map(|d| match d {
            AiDifficulty::Easy => "easy",
            AiDifficulty::Normal => "normal",
            AiDifficulty::Hard => "hard",
        })
    }

    pub fn game_state_message(&self) -> String {
        let state = self.game.to_json();
        json!({
            "type": "game_state",
            "room_id": self.id,
            "room_status": self.status,
            "white_player": self.white_player_id,
            "black_player": self.black_player_id,
            "vs_computer": self.vs_computer,
            "ai_difficulty": self.difficulty_str(),
            "state": state,
        }).to_string()
    }
}

pub struct AppState {
    pub rooms: HashMap<String, Room>,
}

impl AppState {
    pub fn new() -> Self {
        AppState {
            rooms: HashMap::new(),
        }
    }

    pub fn create_room(&mut self, game_type: GameType) -> String {
        let id = Uuid::new_v4().to_string()[..8].to_string();
        let room = Room::new(id.clone(), game_type);
        self.rooms.insert(id.clone(), room);
        id
    }

    pub fn create_room_vs_computer(&mut self, game_type: GameType, difficulty: AiDifficulty) -> String {
        let id = Uuid::new_v4().to_string()[..8].to_string();
        let room = Room::new_vs_computer(id.clone(), game_type, difficulty);
        self.rooms.insert(id.clone(), room);
        id
    }

    pub fn list_rooms(&self) -> Vec<RoomInfo> {
        self.rooms.values()
            .filter(|r| r.status != RoomStatus::Finished && !r.vs_computer)
            .map(|r| r.to_info())
            .collect()
    }

    pub fn get_room_info(&self, id: &str) -> Option<RoomInfo> {
        self.rooms.get(id).map(|r| r.to_info())
    }
}

pub type SharedState = Arc<Mutex<AppState>>;

pub fn new_shared_state() -> SharedState {
    Arc::new(Mutex::new(AppState::new()))
}
