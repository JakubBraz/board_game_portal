use serde_json::{json, Value};
use super::{AiDifficulty, GameLogic, GameStatus, GameType, PlayerColor};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GomokuColor {
    Black,
    White,
}

impl GomokuColor {
    fn opposite(&self) -> GomokuColor {
        match self {
            GomokuColor::Black => GomokuColor::White,
            GomokuColor::White => GomokuColor::Black,
        }
    }
    fn to_player(&self) -> PlayerColor {
        match self {
            GomokuColor::White => PlayerColor::White,
            GomokuColor::Black => PlayerColor::Black,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GomokuGame {
    pub size: usize,
    pub board: Vec<Vec<Option<GomokuColor>>>,
    pub current_turn: GomokuColor,
    pub status: GameStatus,
    pub move_count: u32,
    pub last_move: Option<(usize, usize)>,
}

impl GomokuGame {
    pub fn new(size: usize) -> Self {
        GomokuGame {
            size,
            board: vec![vec![None; size]; size],
            current_turn: GomokuColor::Black,
            status: GameStatus::Playing,
            move_count: 0,
            last_move: None,
        }
    }

    fn check_win(&self, row: usize, col: usize, color: GomokuColor) -> bool {
        let directions = [(0, 1), (1, 0), (1, 1), (1, -1i32)];

        for (dr, dc) in &directions {
            let mut count = 1;

            // Count in positive direction
            let mut r = row as i32 + dr;
            let mut c = col as i32 + dc;
            while r >= 0 && r < self.size as i32 && c >= 0 && c < self.size as i32 {
                if self.board[r as usize][c as usize] == Some(color) {
                    count += 1;
                    r += dr;
                    c += dc;
                } else {
                    break;
                }
            }

            // Count in negative direction
            let mut r = row as i32 - dr;
            let mut c = col as i32 - dc;
            while r >= 0 && r < self.size as i32 && c >= 0 && c < self.size as i32 {
                if self.board[r as usize][c as usize] == Some(color) {
                    count += 1;
                    r -= dr;
                    c -= dc;
                } else {
                    break;
                }
            }

            if count >= 5 {
                return true;
            }
        }
        false
    }

    fn is_board_full(&self) -> bool {
        for row in &self.board {
            for cell in row {
                if cell.is_none() {
                    return false;
                }
            }
        }
        true
    }

    fn board_to_json(&self) -> Value {
        let rows: Vec<Value> = self.board.iter().map(|row| {
            let cols: Vec<Value> = row.iter().map(|cell| match cell {
                None => Value::Null,
                Some(GomokuColor::Black) => json!("black"),
                Some(GomokuColor::White) => json!("white"),
            }).collect();
            Value::Array(cols)
        }).collect();
        Value::Array(rows)
    }

    fn ai_random_move(&self) -> Option<Value> {
        let mut empties = Vec::new();
        for r in 0..self.size {
            for c in 0..self.size {
                if self.board[r][c].is_none() {
                    empties.push((r, c));
                }
            }
        }
        if empties.is_empty() { return None; }
        // Pseudo-random using move count as seed
        let idx = (self.move_count as usize * 7919 + 13) % empties.len();
        let (r, c) = empties[idx];
        Some(json!({ "row": r, "col": c }))
    }

    fn neighbors_have_stone(&self, row: usize, col: usize, radius: usize) -> bool {
        let rmin = row.saturating_sub(radius);
        let rmax = (row + radius + 1).min(self.size);
        let cmin = col.saturating_sub(radius);
        let cmax = (col + radius + 1).min(self.size);
        for r in rmin..rmax {
            for c in cmin..cmax {
                if self.board[r][c].is_some() {
                    return true;
                }
            }
        }
        false
    }

    fn score_line(&self, row: usize, col: usize, dr: i32, dc: i32, color: GomokuColor) -> i32 {
        let mut my_count = 0i32;
        let mut open_ends = 0i32;
        // count in positive direction
        let mut r = row as i32 + dr;
        let mut c = col as i32 + dc;
        while r >= 0 && r < self.size as i32 && c >= 0 && c < self.size as i32 {
            match self.board[r as usize][c as usize] {
                Some(col) if col == color => { my_count += 1; r += dr; c += dc; }
                None => { open_ends += 1; break; }
                _ => break,
            }
        }
        // count in negative direction
        let mut r = row as i32 - dr;
        let mut c = col as i32 - dc;
        while r >= 0 && r < self.size as i32 && c >= 0 && c < self.size as i32 {
            match self.board[r as usize][c as usize] {
                Some(col) if col == color => { my_count += 1; r -= dr; c -= dc; }
                None => { open_ends += 1; break; }
                _ => break,
            }
        }
        // score based on length and openness
        match (my_count, open_ends) {
            (c, _) if c >= 4 => 100_000,
            (3, 2) => 5_000,
            (3, 1) => 500,
            (2, 2) => 100,
            (2, 1) => 10,
            (1, 2) => 5,
            _ => 0,
        }
    }

    fn evaluate_cell(&self, row: usize, col: usize, color: GomokuColor) -> i32 {
        let dirs = [(0i32,1i32),(1,0),(1,1),(1,-1i32)];
        let mut my_score = 0i32;
        let mut opp_score = 0i32;
        let opp = color.opposite();
        for (dr, dc) in dirs {
            my_score += self.score_line(row, col, dr, dc, color);
            opp_score += self.score_line(row, col, dr, dc, opp);
        }
        // Weight: attack slightly more than defend, but blocking wins is critical
        my_score + (opp_score as f32 * 1.1) as i32
    }

    fn ai_heuristic_move(&self, color: GomokuColor) -> Option<Value> {
        // If board empty, play center
        if self.move_count == 0 {
            let c = self.size / 2;
            return Some(json!({ "row": c, "col": c }));
        }
        let mut best_score = i32::MIN;
        let mut best = None;
        for r in 0..self.size {
            for c in 0..self.size {
                if self.board[r][c].is_none() && self.neighbors_have_stone(r, c, 2) {
                    let score = self.evaluate_cell(r, c, color);
                    if score > best_score {
                        best_score = score;
                        best = Some((r, c));
                    }
                }
            }
        }
        // Fallback if no neighbors
        if best.is_none() {
            let c = self.size / 2;
            best = Some((c, c));
        }
        best.map(|(r, c)| json!({ "row": r, "col": c }))
    }

    fn gomoku_minimax(&self, depth: u8, mut alpha: i32, mut beta: i32, maximizing: bool, ai_color: GomokuColor) -> i32 {
        if self.status != super::GameStatus::Playing || depth == 0 {
            // Evaluate board from ai_color perspective
            let mut score = 0i32;
            let dirs = [(0i32,1i32),(1,0),(1,1),(1,-1i32)];
            for r in 0..self.size {
                for c in 0..self.size {
                    if self.board[r][c].is_none() {
                        for (dr, dc) in dirs {
                            score += self.score_line(r, c, dr, dc, ai_color);
                            score -= self.score_line(r, c, dr, dc, ai_color.opposite());
                        }
                    }
                }
            }
            return score;
        }
        let current = if maximizing { ai_color } else { ai_color.opposite() };
        // Restrict candidates to neighborhood
        let mut candidates = Vec::new();
        for r in 0..self.size {
            for c in 0..self.size {
                if self.board[r][c].is_none() && self.neighbors_have_stone(r, c, 1) {
                    let score = self.evaluate_cell(r, c, current);
                    candidates.push((score, r, c));
                }
            }
        }
        // Sort and take top candidates to limit branching
        candidates.sort_by(|a, b| b.0.cmp(&a.0));
        candidates.truncate(10);
        if candidates.is_empty() {
            return 0;
        }
        if maximizing {
            let mut best = i32::MIN + 1;
            for (_, r, c) in candidates {
                let mut next = self.clone();
                next.board[r][c] = Some(current);
                next.move_count += 1;
                if next.check_win(r, c, current) {
                    next.status = if current == GomokuColor::Black { super::GameStatus::BlackWon } else { super::GameStatus::WhiteWon };
                } else {
                    next.current_turn = current.opposite();
                }
                let val = next.gomoku_minimax(depth - 1, alpha, beta, false, ai_color);
                if val > best { best = val; }
                if best > alpha { alpha = best; }
                if alpha >= beta { break; }
            }
            best
        } else {
            let mut best = i32::MAX - 1;
            for (_, r, c) in candidates {
                let mut next = self.clone();
                next.board[r][c] = Some(current);
                next.move_count += 1;
                if next.check_win(r, c, current) {
                    next.status = if current == GomokuColor::Black { super::GameStatus::BlackWon } else { super::GameStatus::WhiteWon };
                } else {
                    next.current_turn = current.opposite();
                }
                let val = next.gomoku_minimax(depth - 1, alpha, beta, true, ai_color);
                if val < best { best = val; }
                if best < beta { beta = best; }
                if alpha >= beta { break; }
            }
            best
        }
    }

    fn ai_minimax_move(&self, color: GomokuColor, depth: u8) -> Option<Value> {
        if self.move_count == 0 {
            let c = self.size / 2;
            return Some(json!({ "row": c, "col": c }));
        }
        let mut candidates = Vec::new();
        for r in 0..self.size {
            for c in 0..self.size {
                if self.board[r][c].is_none() && self.neighbors_have_stone(r, c, 1) {
                    let score = self.evaluate_cell(r, c, color);
                    candidates.push((score, r, c));
                }
            }
        }
        candidates.sort_by(|a, b| b.0.cmp(&a.0));
        candidates.truncate(15);
        if candidates.is_empty() {
            return self.ai_heuristic_move(color);
        }
        let mut best_val = i32::MIN + 1;
        let mut best = None;
        for (_, r, c) in &candidates {
            let mut next = self.clone();
            next.board[*r][*c] = Some(color);
            next.move_count += 1;
            if next.check_win(*r, *c, color) {
                // Immediate win — take it!
                return Some(json!({ "row": r, "col": c }));
            }
            next.current_turn = color.opposite();
            let val = next.gomoku_minimax(depth - 1, i32::MIN + 1, i32::MAX - 1, false, color);
            if val > best_val {
                best_val = val;
                best = Some((*r, *c));
            }
        }
        best.map(|(r, c)| json!({ "row": r, "col": c }))
    }
}

impl GameLogic for GomokuGame {
    fn make_move(&mut self, mv: &Value, player: &PlayerColor) -> Result<(), String> {
        if self.status != GameStatus::Playing {
            return Err("Game is over".into());
        }
        if self.current_turn.to_player() != *player {
            return Err("Not your turn".into());
        }

        let row = mv["row"].as_u64().ok_or("invalid row")? as usize;
        let col = mv["col"].as_u64().ok_or("invalid col")? as usize;

        if row >= self.size || col >= self.size {
            return Err("Coordinates out of bounds".into());
        }

        if self.board[row][col].is_some() {
            return Err("Intersection already occupied".into());
        }

        let color = self.current_turn;
        self.board[row][col] = Some(color);
        self.last_move = Some((row, col));
        self.move_count += 1;

        if self.check_win(row, col, color) {
            self.status = match color {
                GomokuColor::Black => GameStatus::BlackWon,
                GomokuColor::White => GameStatus::WhiteWon,
            };
        } else if self.is_board_full() {
            self.status = GameStatus::Draw;
        } else {
            self.current_turn = color.opposite();
        }

        Ok(())
    }

    fn to_json(&self) -> Value {
        json!({
            "type": "gomoku",
            "size": self.size,
            "board": self.board_to_json(),
            "current_turn": match self.current_turn {
                GomokuColor::Black => "black",
                GomokuColor::White => "white",
            },
            "status": self.status,
            "move_count": self.move_count,
            "last_move": self.last_move.map(|(r, c)| json!([r, c])),
        })
    }

    fn status(&self) -> GameStatus {
        self.status.clone()
    }

    fn current_player(&self) -> PlayerColor {
        self.current_turn.to_player()
    }

    fn game_type(&self) -> GameType {
        GameType::Gomoku
    }

    fn clone_box(&self) -> Box<dyn GameLogic> {
        Box::new(self.clone())
    }

    fn get_ai_move(&self, difficulty: AiDifficulty) -> Option<Value> {
        let color = match self.current_turn {
            GomokuColor::Black => GomokuColor::Black,
            GomokuColor::White => GomokuColor::White,
        };
        match difficulty {
            AiDifficulty::Easy => self.ai_random_move(),
            AiDifficulty::Normal => self.ai_heuristic_move(color),
            AiDifficulty::Hard => self.ai_minimax_move(color, 4),
        }
    }
}
