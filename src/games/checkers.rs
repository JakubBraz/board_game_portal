use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use super::{AiDifficulty, GameLogic, GameStatus, GameType, PlayerColor};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckersPieceType {
    Man,
    King,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckersColor {
    White,
    Black,
}

impl CheckersColor {
    fn opposite(&self) -> CheckersColor {
        match self {
            CheckersColor::White => CheckersColor::Black,
            CheckersColor::Black => CheckersColor::White,
        }
    }
    fn to_player(&self) -> PlayerColor {
        match self {
            CheckersColor::White => PlayerColor::White,
            CheckersColor::Black => PlayerColor::Black,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CheckersPiece {
    pub piece_type: CheckersPieceType,
    pub color: CheckersColor,
}

type Board = [[Option<CheckersPiece>; 8]; 8];

#[derive(Debug, Clone)]
pub struct CheckersGame {
    pub board: Board,
    pub current_turn: CheckersColor,
    pub status: GameStatus,
    // If a multi-jump is in progress, only this piece can move
    pub must_continue_from: Option<(usize, usize)>,
}

impl CheckersGame {
    pub fn new() -> Self {
        let mut board: Board = [[None; 8]; 8];

        // Black pieces on rows 5-7 (top of board)
        for row in 5..8usize {
            for col in 0..8usize {
                if (row + col) % 2 == 1 {
                    board[row][col] = Some(CheckersPiece {
                        piece_type: CheckersPieceType::Man,
                        color: CheckersColor::Black,
                    });
                }
            }
        }
        // White pieces on rows 0-2 (bottom)
        for row in 0..3usize {
            for col in 0..8usize {
                if (row + col) % 2 == 1 {
                    board[row][col] = Some(CheckersPiece {
                        piece_type: CheckersPieceType::Man,
                        color: CheckersColor::White,
                    });
                }
            }
        }

        CheckersGame {
            board,
            current_turn: CheckersColor::White,
            status: GameStatus::Playing,
            must_continue_from: None,
        }
    }

    // Returns list of (to_row, to_col, captured_row, captured_col) for jumps
    fn jump_moves(&self, row: usize, col: usize) -> Vec<(usize, usize, usize, usize)> {
        let piece = match self.board[row][col] {
            Some(p) => p,
            None => return vec![],
        };

        let directions: Vec<(i32, i32)> = match piece.piece_type {
            CheckersPieceType::Man => {
                if piece.color == CheckersColor::White {
                    vec![(1, -1), (1, 1)]
                } else {
                    vec![(-1, -1), (-1, 1)]
                }
            }
            CheckersPieceType::King => vec![(1, -1), (1, 1), (-1, -1), (-1, 1)],
        };

        let mut jumps = Vec::new();
        for (dr, dc) in directions {
            let mid_r = row as i32 + dr;
            let mid_c = col as i32 + dc;
            let land_r = row as i32 + 2 * dr;
            let land_c = col as i32 + 2 * dc;

            if land_r < 0 || land_r >= 8 || land_c < 0 || land_c >= 8 {
                continue;
            }
            let (mr, mc, lr, lc) = (mid_r as usize, mid_c as usize, land_r as usize, land_c as usize);
            if let Some(mid_p) = self.board[mr][mc] {
                if mid_p.color != piece.color && self.board[lr][lc].is_none() {
                    jumps.push((lr, lc, mr, mc));
                }
            }
        }
        jumps
    }

    // Returns simple moves (no capture)
    fn simple_moves(&self, row: usize, col: usize) -> Vec<(usize, usize)> {
        let piece = match self.board[row][col] {
            Some(p) => p,
            None => return vec![],
        };

        let directions: Vec<(i32, i32)> = match piece.piece_type {
            CheckersPieceType::Man => {
                if piece.color == CheckersColor::White {
                    vec![(1, -1), (1, 1)]
                } else {
                    vec![(-1, -1), (-1, 1)]
                }
            }
            CheckersPieceType::King => vec![(1, -1), (1, 1), (-1, -1), (-1, 1)],
        };

        let mut moves = Vec::new();
        for (dr, dc) in directions {
            let r = row as i32 + dr;
            let c = col as i32 + dc;
            if r >= 0 && r < 8 && c >= 0 && c < 8 {
                if self.board[r as usize][c as usize].is_none() {
                    moves.push((r as usize, c as usize));
                }
            }
        }
        moves
    }

    fn any_jumps_available(&self, color: CheckersColor) -> bool {
        for row in 0..8 {
            for col in 0..8 {
                if let Some(p) = self.board[row][col] {
                    if p.color == color && !self.jump_moves(row, col).is_empty() {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn has_any_moves(&self, color: CheckersColor) -> bool {
        for row in 0..8 {
            for col in 0..8 {
                if let Some(p) = self.board[row][col] {
                    if p.color == color {
                        if !self.jump_moves(row, col).is_empty() {
                            return true;
                        }
                        if !self.simple_moves(row, col).is_empty() {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn board_to_json(&self) -> Value {
        let mut rows = Vec::new();
        for row in 0..8 {
            let mut cols = Vec::new();
            for col in 0..8 {
                match self.board[row][col] {
                    None => cols.push(Value::Null),
                    Some(p) => cols.push(json!({
                        "type": format!("{:?}", p.piece_type).to_lowercase(),
                        "color": format!("{:?}", p.color).to_lowercase(),
                    })),
                }
            }
            rows.push(Value::Array(cols));
        }
        Value::Array(rows)
    }

    fn get_valid_moves_for_player(&self, color: CheckersColor) -> Value {
        let has_jumps = self.any_jumps_available(color);
        let mut result = Vec::new();

        for row in 0..8 {
            for col in 0..8 {
                if let Some(p) = self.board[row][col] {
                    if p.color != color { continue; }
                    if let Some(must) = self.must_continue_from {
                        if must != (row, col) { continue; }
                    }

                    if has_jumps {
                        let jumps = self.jump_moves(row, col);
                        if !jumps.is_empty() {
                            let to_list: Vec<Value> = jumps.iter()
                                .map(|&(tr, tc, _, _)| json!([tr, tc]))
                                .collect();
                            result.push(json!({ "from": [row, col], "to": to_list, "must_jump": true }));
                        }
                    } else {
                        let moves = self.simple_moves(row, col);
                        if !moves.is_empty() {
                            let to_list: Vec<Value> = moves.iter().map(|&(r, c)| json!([r, c])).collect();
                            result.push(json!({ "from": [row, col], "to": to_list, "must_jump": false }));
                        }
                    }
                }
            }
        }
        Value::Array(result)
    }

    fn all_moves(&self, color: CheckersColor) -> Vec<serde_json::Value> {
        let has_jumps = self.any_jumps_available(color);
        let mut result = Vec::new();
        for row in 0..8 {
            for col in 0..8 {
                if let Some(p) = self.board[row][col] {
                    if p.color != color { continue; }
                    if has_jumps {
                        for (tr, tc, _, _) in self.jump_moves(row, col) {
                            result.push(json!({ "from": [row, col], "to": [tr, tc] }));
                        }
                    } else {
                        for (tr, tc) in self.simple_moves(row, col) {
                            result.push(json!({ "from": [row, col], "to": [tr, tc] }));
                        }
                    }
                }
            }
        }
        result
    }

    fn evaluate_board(&self, for_color: CheckersColor) -> i32 {
        let mut score = 0i32;
        for row in 0..8 {
            for col in 0..8 {
                if let Some(p) = self.board[row][col] {
                    let piece_val = match p.piece_type {
                        CheckersPieceType::Man => 100,
                        CheckersPieceType::King => 300,
                    };
                    // Positional bonus: advance men toward promotion
                    let pos_bonus = match p.color {
                        CheckersColor::White => row as i32 * 5,
                        CheckersColor::Black => (7 - row as i32) * 5,
                    };
                    let val = piece_val + pos_bonus;
                    if p.color == for_color { score += val; } else { score -= val; }
                }
            }
        }
        score
    }

    fn checkers_minimax(&self, depth: u8, mut alpha: i32, mut beta: i32, maximizing: bool, ai_color: CheckersColor) -> i32 {
        if self.status != super::GameStatus::Playing || depth == 0 {
            return self.evaluate_board(ai_color);
        }
        let current = self.current_turn;
        let is_max = current == ai_color;
        let moves = self.all_moves(current);
        if moves.is_empty() {
            return if is_max { -50000 } else { 50000 };
        }
        if is_max {
            let mut best = i32::MIN + 1;
            for mv in moves {
                let mut next = self.clone();
                let _ = next.make_move(&mv, &if current == CheckersColor::White { super::PlayerColor::White } else { super::PlayerColor::Black });
                let val = next.checkers_minimax(depth - 1, alpha, beta, next.current_turn != current || next.status != super::GameStatus::Playing, ai_color);
                if val > best { best = val; }
                if best > alpha { alpha = best; }
                if alpha >= beta { break; }
            }
            best
        } else {
            let mut best = i32::MAX - 1;
            for mv in moves {
                let mut next = self.clone();
                let _ = next.make_move(&mv, &if current == CheckersColor::White { super::PlayerColor::White } else { super::PlayerColor::Black });
                let val = next.checkers_minimax(depth - 1, alpha, beta, next.current_turn != current || next.status != super::GameStatus::Playing, ai_color);
                if val < best { best = val; }
                if best < beta { beta = best; }
                if alpha >= beta { break; }
            }
            best
        }
    }

    fn ai_checkers_random(&self, color: CheckersColor) -> Option<Value> {
        let moves = self.all_moves(color);
        if moves.is_empty() { return None; }
        let idx = moves.len() / 2; // deterministic pseudo-random
        Some(moves[idx].clone())
    }

    fn ai_checkers_minimax_root(&self, color: CheckersColor, depth: u8) -> Option<Value> {
        let moves = self.all_moves(color);
        if moves.is_empty() { return None; }
        let mut best_val = i32::MIN + 1;
        let mut best_move = moves[0].clone();
        for mv in &moves {
            let mut next = self.clone();
            let _ = next.make_move(mv, &if color == CheckersColor::White { super::PlayerColor::White } else { super::PlayerColor::Black });
            let val = next.checkers_minimax(depth - 1, i32::MIN + 1, i32::MAX - 1, next.current_turn != color, color);
            if val > best_val {
                best_val = val;
                best_move = mv.clone();
            }
        }
        Some(best_move)
    }
}

impl GameLogic for CheckersGame {
    fn make_move(&mut self, mv: &Value, player: &PlayerColor) -> Result<(), String> {
        if self.status != GameStatus::Playing {
            return Err("Game is over".into());
        }
        if self.current_turn.to_player() != *player {
            return Err("Not your turn".into());
        }

        let from_r = mv["from"][0].as_u64().ok_or("invalid from row")? as usize;
        let from_c = mv["from"][1].as_u64().ok_or("invalid from col")? as usize;
        let to_r = mv["to"][0].as_u64().ok_or("invalid to row")? as usize;
        let to_c = mv["to"][1].as_u64().ok_or("invalid to col")? as usize;

        if from_r >= 8 || from_c >= 8 || to_r >= 8 || to_c >= 8 {
            return Err("Coordinates out of bounds".into());
        }

        // Check if continuing a multi-jump
        if let Some(must) = self.must_continue_from {
            if must != (from_r, from_c) {
                return Err("You must continue jumping with the same piece".into());
            }
        }

        let piece = self.board[from_r][from_c].ok_or("No piece at source")?;
        if piece.color.to_player() != *player {
            return Err("That piece doesn't belong to you".into());
        }

        let has_jumps = self.any_jumps_available(self.current_turn);
        let row_diff = to_r as i32 - from_r as i32;
        let col_diff = (to_c as i32 - from_c as i32).abs();
        let is_jump = row_diff.abs() == 2 && col_diff == 2;

        if has_jumps && !is_jump {
            return Err("You must make a jump move".into());
        }

        if is_jump {
            let jumps = self.jump_moves(from_r, from_c);
            let jump = jumps.iter().find(|&&(tr, tc, _, _)| tr == to_r && tc == to_c);
            let &(_, _, cap_r, cap_c) = jump.ok_or("Illegal jump")?;

            self.board[cap_r][cap_c] = None;
            self.board[to_r][to_c] = Some(piece);
            self.board[from_r][from_c] = None;

            // Kinging
            if piece.piece_type == CheckersPieceType::Man {
                let king_row = if piece.color == CheckersColor::White { 7 } else { 0 };
                if to_r == king_row {
                    self.board[to_r][to_c] = Some(CheckersPiece {
                        piece_type: CheckersPieceType::King,
                        color: piece.color,
                    });
                    // Can't continue jumping after kinging
                    self.must_continue_from = None;
                    self.current_turn = self.current_turn.opposite();
                } else {
                    // Check for more jumps
                    let more_jumps = self.jump_moves(to_r, to_c);
                    if more_jumps.is_empty() {
                        self.must_continue_from = None;
                        self.current_turn = self.current_turn.opposite();
                    } else {
                        self.must_continue_from = Some((to_r, to_c));
                    }
                }
            } else {
                let more_jumps = self.jump_moves(to_r, to_c);
                if more_jumps.is_empty() {
                    self.must_continue_from = None;
                    self.current_turn = self.current_turn.opposite();
                } else {
                    self.must_continue_from = Some((to_r, to_c));
                }
            }
        } else {
            // Simple move
            let moves = self.simple_moves(from_r, from_c);
            if !moves.contains(&(to_r, to_c)) {
                return Err("Illegal move".into());
            }

            self.board[to_r][to_c] = Some(piece);
            self.board[from_r][from_c] = None;

            // Kinging
            let king_row = if piece.color == CheckersColor::White { 7 } else { 0 };
            if piece.piece_type == CheckersPieceType::Man && to_r == king_row {
                self.board[to_r][to_c] = Some(CheckersPiece {
                    piece_type: CheckersPieceType::King,
                    color: piece.color,
                });
            }

            self.must_continue_from = None;
            self.current_turn = self.current_turn.opposite();
        }

        // Check win condition
        let next = self.current_turn;
        if !self.has_any_moves(next) {
            self.status = match next {
                CheckersColor::White => GameStatus::BlackWon,
                CheckersColor::Black => GameStatus::WhiteWon,
            };
        }

        Ok(())
    }

    fn to_json(&self) -> Value {
        json!({
            "type": "checkers",
            "board": self.board_to_json(),
            "current_turn": format!("{:?}", self.current_turn).to_lowercase(),
            "status": self.status,
            "must_continue_from": self.must_continue_from.map(|(r, c)| json!([r, c])),
            "valid_moves": if self.status == GameStatus::Playing {
                self.get_valid_moves_for_player(self.current_turn)
            } else {
                Value::Array(vec![])
            },
        })
    }

    fn status(&self) -> GameStatus {
        self.status.clone()
    }

    fn current_player(&self) -> PlayerColor {
        self.current_turn.to_player()
    }

    fn game_type(&self) -> GameType {
        GameType::Checkers
    }

    fn clone_box(&self) -> Box<dyn super::GameLogic> {
        Box::new(self.clone())
    }

    fn get_ai_move(&self, difficulty: AiDifficulty) -> Option<Value> {
        let color = self.current_turn;
        match difficulty {
            AiDifficulty::Easy => self.ai_checkers_random(color),
            AiDifficulty::Normal => self.ai_checkers_minimax_root(color, 4),
            AiDifficulty::Hard => self.ai_checkers_minimax_root(color, 8),
        }
    }
}
