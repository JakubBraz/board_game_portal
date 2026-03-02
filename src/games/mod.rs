pub mod chess;
pub mod checkers;
pub mod go;
pub mod gomoku;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum AiDifficulty {
    Easy,
    Normal,
    Hard,
}

impl AiDifficulty {
    pub fn from_str(s: &str) -> Option<AiDifficulty> {
        match s {
            "easy" => Some(AiDifficulty::Easy),
            "normal" => Some(AiDifficulty::Normal),
            "hard" => Some(AiDifficulty::Hard),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PlayerColor {
    White,
    Black,
}

impl PlayerColor {
    pub fn opposite(&self) -> PlayerColor {
        match self {
            PlayerColor::White => PlayerColor::Black,
            PlayerColor::Black => PlayerColor::White,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum GameStatus {
    Playing,
    WhiteWon,
    BlackWon,
    Draw,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum GameType {
    Chess,
    Checkers,
    Go,
    Gomoku,
}

impl GameType {
    pub fn from_str(s: &str) -> Option<GameType> {
        match s {
            "chess" => Some(GameType::Chess),
            "checkers" => Some(GameType::Checkers),
            "go" => Some(GameType::Go),
            "gomoku" => Some(GameType::Gomoku),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            GameType::Chess => "Chess",
            GameType::Checkers => "Checkers",
            GameType::Go => "Go",
            GameType::Gomoku => "Gomoku",
        }
    }
}

pub trait GameLogic: Send + Sync + 'static {
    fn make_move(&mut self, mv: &Value, player: &PlayerColor) -> Result<(), String>;
    fn to_json(&self) -> Value;
    fn status(&self) -> GameStatus;
    fn current_player(&self) -> PlayerColor;
    fn game_type(&self) -> GameType;
    fn clone_box(&self) -> Box<dyn GameLogic>;
    fn get_ai_move(&self, difficulty: AiDifficulty) -> Option<Value>;
}

pub fn create_game(game_type: &GameType) -> Box<dyn GameLogic> {
    match game_type {
        GameType::Chess => Box::new(chess::ChessGame::new()),
        GameType::Checkers => Box::new(checkers::CheckersGame::new()),
        GameType::Go => Box::new(go::GoGame::new(19)),
        GameType::Gomoku => Box::new(gomoku::GomokuGame::new(15)),
    }
}
