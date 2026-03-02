use serde_json::{json, Value};
use super::{AiDifficulty, GameLogic, GameStatus, GameType, PlayerColor};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Stone {
    Black,
    White,
}

impl Stone {
    fn opposite(&self) -> Stone {
        match self {
            Stone::Black => Stone::White,
            Stone::White => Stone::Black,
        }
    }
    fn to_player(&self) -> PlayerColor {
        match self {
            Stone::White => PlayerColor::White,
            Stone::Black => PlayerColor::Black,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GoGame {
    pub size: usize,
    pub board: Vec<Vec<Option<Stone>>>,
    pub current_turn: Stone,
    pub status: GameStatus,
    pub captures_black: u32, // stones captured BY black (white stones removed)
    pub captures_white: u32, // stones captured BY white (black stones removed)
    pub ko_point: Option<(usize, usize)>,
    pub consecutive_passes: u32,
    pub move_count: u32,
    pub score_black: f32,
    pub score_white: f32,
    // Previous board state for ko detection
    pub previous_board: Option<Vec<Vec<Option<Stone>>>>,
}

impl GoGame {
    pub fn new(size: usize) -> Self {
        GoGame {
            size,
            board: vec![vec![None; size]; size],
            current_turn: Stone::Black,
            status: GameStatus::Playing,
            captures_black: 0,
            captures_white: 0,
            ko_point: None,
            consecutive_passes: 0,
            move_count: 0,
            score_black: 0.0,
            score_white: 0.0,
            previous_board: None,
        }
    }

    fn get_group(&self, row: usize, col: usize) -> Vec<(usize, usize)> {
        let color = match self.board[row][col] {
            Some(c) => c,
            None => return vec![],
        };
        let mut group = vec![(row, col)];
        let mut visited = vec![vec![false; self.size]; self.size];
        visited[row][col] = true;
        let mut stack = vec![(row, col)];

        while let Some((r, c)) = stack.pop() {
            for (nr, nc) in self.neighbors(r, c) {
                if !visited[nr][nc] {
                    if let Some(nc_color) = self.board[nr][nc] {
                        if nc_color == color {
                            visited[nr][nc] = true;
                            group.push((nr, nc));
                            stack.push((nr, nc));
                        }
                    }
                }
            }
        }
        group
    }

    fn get_liberties(&self, group: &[(usize, usize)]) -> usize {
        let mut liberties = std::collections::HashSet::new();
        for &(r, c) in group {
            for (nr, nc) in self.neighbors(r, c) {
                if self.board[nr][nc].is_none() {
                    liberties.insert((nr, nc));
                }
            }
        }
        liberties.len()
    }

    fn neighbors(&self, row: usize, col: usize) -> Vec<(usize, usize)> {
        let mut result = Vec::new();
        if row > 0 { result.push((row - 1, col)); }
        if row + 1 < self.size { result.push((row + 1, col)); }
        if col > 0 { result.push((row, col - 1)); }
        if col + 1 < self.size { result.push((row, col + 1)); }
        result
    }

    fn remove_group(&mut self, group: &[(usize, usize)]) -> u32 {
        let count = group.len() as u32;
        for &(r, c) in group {
            self.board[r][c] = None;
        }
        count
    }

    fn boards_equal(&self, a: &Vec<Vec<Option<Stone>>>, b: &Vec<Vec<Option<Stone>>>) -> bool {
        for r in 0..self.size {
            for c in 0..self.size {
                if a[r][c] != b[r][c] {
                    return false;
                }
            }
        }
        true
    }

    fn calculate_score(&mut self) {
        // Chinese rules: area scoring (stones + territory)
        let mut black_area = 0i32;
        let mut white_area = 0i32;

        // Count stones
        for r in 0..self.size {
            for c in 0..self.size {
                match self.board[r][c] {
                    Some(Stone::Black) => black_area += 1,
                    Some(Stone::White) => white_area += 1,
                    None => {}
                }
            }
        }

        // Count territory using flood fill
        let mut visited = vec![vec![false; self.size]; self.size];
        for r in 0..self.size {
            for c in 0..self.size {
                if self.board[r][c].is_none() && !visited[r][c] {
                    // BFS to find connected empty region
                    let mut region = Vec::new();
                    let mut borders_black = false;
                    let mut borders_white = false;
                    let mut stack = vec![(r, c)];
                    visited[r][c] = true;

                    while let Some((sr, sc)) = stack.pop() {
                        region.push((sr, sc));
                        for (nr, nc) in self.neighbors(sr, sc) {
                            match self.board[nr][nc] {
                                Some(Stone::Black) => borders_black = true,
                                Some(Stone::White) => borders_white = true,
                                None => {
                                    if !visited[nr][nc] {
                                        visited[nr][nc] = true;
                                        stack.push((nr, nc));
                                    }
                                }
                            }
                        }
                    }

                    let size = region.len() as i32;
                    if borders_black && !borders_white {
                        black_area += size;
                    } else if borders_white && !borders_black {
                        white_area += size;
                    }
                }
            }
        }

        let komi = 6.5f32; // Komi for white
        self.score_black = black_area as f32;
        self.score_white = white_area as f32 + komi;

        if self.score_black > self.score_white {
            self.status = GameStatus::BlackWon;
        } else {
            self.status = GameStatus::WhiteWon;
        }
    }

    fn board_to_json(&self) -> Value {
        let mut rows = Vec::new();
        for row in &self.board {
            let cols: Vec<Value> = row.iter().map(|cell| match cell {
                None => Value::Null,
                Some(Stone::Black) => json!("black"),
                Some(Stone::White) => json!("white"),
            }).collect();
            rows.push(Value::Array(cols));
        }
        Value::Array(rows)
    }

    fn legal_positions(&self) -> Vec<(usize, usize)> {
        let mut result = Vec::new();
        for r in 0..self.size {
            for c in 0..self.size {
                if self.board[r][c].is_none() {
                    if let Some((kr, kc)) = self.ko_point {
                        if r == kr && c == kc { continue; }
                    }
                    result.push((r, c));
                }
            }
        }
        result
    }

    fn score_move(&self, row: usize, col: usize, color: Stone) -> i32 {
        let mut test = self.clone();
        test.board[row][col] = Some(color);
        // Count immediate captures
        let mut captures = 0i32;
        for (nr, nc) in test.neighbors(row, col) {
            if let Some(nc_color) = test.board[nr][nc] {
                if nc_color == color.opposite() {
                    let group = test.get_group(nr, nc);
                    if test.get_liberties(&group) == 0 {
                        captures += group.len() as i32;
                    }
                }
            }
        }
        // Check if move fills own liberties (bad)
        let placed_group = test.get_group(row, col);
        let own_libs = test.get_liberties(&placed_group) as i32;
        // Check atari (threatening to capture opponent)
        let mut atari_threats = 0i32;
        for (nr, nc) in test.neighbors(row, col) {
            if let Some(nc_color) = test.board[nr][nc] {
                if nc_color == color.opposite() {
                    let group = test.get_group(nr, nc);
                    if test.get_liberties(&group) == 1 {
                        atari_threats += group.len() as i32;
                    }
                }
            }
        }
        // Avoid edges/corners on small board, prefer center on large board
        let center = self.size / 2;
        let dist_center = ((row as i32 - center as i32).abs() + (col as i32 - center as i32).abs()) as i32;
        let position_bonus = (self.size as i32 - dist_center) / 2;
        captures * 50 + atari_threats * 20 + own_libs * 10 + position_bonus - if own_libs == 0 { 1000 } else { 0 }
    }

    fn ai_go_random(&self) -> Option<Value> {
        let positions = self.legal_positions();
        if positions.is_empty() {
            return Some(json!({ "row": 0, "col": 0, "pass": true }));
        }
        let idx = (self.move_count as usize * 7919 + 17) % positions.len();
        let (r, c) = positions[idx];
        Some(json!({ "row": r, "col": c, "pass": false }))
    }

    fn ai_go_heuristic(&self, color: Stone) -> Option<Value> {
        let positions = self.legal_positions();
        if positions.is_empty() {
            return Some(json!({ "row": 0, "col": 0, "pass": true }));
        }
        // Filter out suicide moves
        let mut best_score = i32::MIN;
        let mut best = None;
        for (r, c) in &positions {
            let mut test = self.clone();
            test.board[*r][*c] = Some(color);
            let group = test.get_group(*r, *c);
            // Skip suicide
            if test.get_liberties(&group) == 0 {
                // Check if it captures opponent first
                let captures_something = test.neighbors(*r, *c).iter().any(|(nr, nc)| {
                    if let Some(nc_color) = test.board[*nr][*nc] {
                        if nc_color == color.opposite() {
                            let g = test.get_group(*nr, *nc);
                            return test.get_liberties(&g) == 0;
                        }
                    }
                    false
                });
                if !captures_something { continue; }
            }
            let score = self.score_move(*r, *c, color);
            if score > best_score {
                best_score = score;
                best = Some((*r, *c));
            }
        }
        if let Some((r, c)) = best {
            Some(json!({ "row": r, "col": c, "pass": false }))
        } else {
            Some(json!({ "row": 0, "col": 0, "pass": true }))
        }
    }

    fn ai_go_hard(&self, color: Stone) -> Option<Value> {
        let positions = self.legal_positions();
        if positions.is_empty() {
            return Some(json!({ "row": 0, "col": 0, "pass": true }));
        }
        // Score each position and also do 1-ply look-ahead for top candidates
        let mut scored: Vec<(i32, usize, usize)> = positions.iter().filter_map(|&(r, c)| {
            let mut test = self.clone();
            test.board[r][c] = Some(color);
            let group = test.get_group(r, c);
            if test.get_liberties(&group) == 0 {
                // Check if captures
                let cap = test.neighbors(r, c).iter().any(|(nr, nc)| {
                    if let Some(nc_c) = test.board[*nr][*nc] {
                        if nc_c == color.opposite() {
                            let g = test.get_group(*nr, *nc);
                            return test.get_liberties(&g) == 0;
                        }
                    }
                    false
                });
                if !cap { return None; }
            }
            Some((self.score_move(r, c, color), r, c))
        }).collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.truncate(20);
        let mut best_score = i32::MIN;
        let mut best = None;
        for (base_score, r, c) in &scored {
            let mut test = self.clone();
            test.board[*r][*c] = Some(color);
            // Look at opponent's best response
            let opp_positions = test.legal_positions();
            let opp_score = opp_positions.iter()
                .map(|&(or, oc)| test.score_move(or, oc, color.opposite()))
                .max()
                .unwrap_or(0);
            let net = base_score - opp_score / 2;
            if net > best_score {
                best_score = net;
                best = Some((*r, *c));
            }
        }
        if let Some((r, c)) = best {
            Some(json!({ "row": r, "col": c, "pass": false }))
        } else {
            Some(json!({ "row": 0, "col": 0, "pass": true }))
        }
    }
}

impl GameLogic for GoGame {
    fn make_move(&mut self, mv: &Value, player: &PlayerColor) -> Result<(), String> {
        if self.status != GameStatus::Playing {
            return Err("Game is over".into());
        }
        if self.current_turn.to_player() != *player {
            return Err("Not your turn".into());
        }

        // Pass move
        if mv["pass"].as_bool() == Some(true) {
            self.consecutive_passes += 1;
            self.ko_point = None;
            self.previous_board = None;
            if self.consecutive_passes >= 2 {
                self.calculate_score();
            } else {
                self.current_turn = self.current_turn.opposite();
            }
            self.move_count += 1;
            return Ok(());
        }

        let row = mv["row"].as_u64().ok_or("invalid row")? as usize;
        let col = mv["col"].as_u64().ok_or("invalid col")? as usize;

        if row >= self.size || col >= self.size {
            return Err("Coordinates out of bounds".into());
        }

        if self.board[row][col].is_some() {
            return Err("Intersection already occupied".into());
        }

        // Ko check
        if let Some((ko_r, ko_c)) = self.ko_point {
            if row == ko_r && col == ko_c {
                return Err("Illegal move (Ko)".into());
            }
        }

        let color = self.current_turn;
        let prev_board = self.board.clone();

        // Place stone
        self.board[row][col] = Some(color);

        // Remove captured opponent groups
        let mut total_captured = 0u32;
        let mut single_capture: Option<(usize, usize)> = None;

        let neighbors = self.neighbors(row, col);
        for (nr, nc) in &neighbors {
            if let Some(nc_color) = self.board[*nr][*nc] {
                if nc_color == color.opposite() {
                    let group = self.get_group(*nr, *nc);
                    if self.get_liberties(&group) == 0 {
                        if group.len() == 1 {
                            single_capture = Some(group[0]);
                        }
                        let captured = self.remove_group(&group);
                        total_captured += captured;
                    }
                }
            }
        }

        // Update captures
        match color {
            Stone::Black => self.captures_black += total_captured,
            Stone::White => self.captures_white += total_captured,
        }

        // Check if placed stone has liberties (suicide check)
        let placed_group = self.get_group(row, col);
        if self.get_liberties(&placed_group) == 0 {
            // Suicide - revert
            self.board = prev_board;
            match color {
                Stone::Black => self.captures_black -= total_captured,
                Stone::White => self.captures_white -= total_captured,
            }
            return Err("Suicide move is illegal".into());
        }

        // Ko detection: if exactly one stone was captured and the board reverts to the previous state
        if let Some(prev) = &self.previous_board.clone() {
            if total_captured == 1 && self.boards_equal(&self.board, prev) {
                // Ko! Revert
                self.board = prev_board;
                match color {
                    Stone::Black => self.captures_black -= total_captured,
                    Stone::White => self.captures_white -= total_captured,
                }
                return Err("Illegal move (Ko)".into());
            }
        }

        // Set ko point
        if total_captured == 1 {
            self.ko_point = single_capture;
        } else {
            self.ko_point = None;
        }

        self.previous_board = Some(prev_board);
        self.consecutive_passes = 0;
        self.current_turn = color.opposite();
        self.move_count += 1;

        Ok(())
    }

    fn to_json(&self) -> Value {
        json!({
            "type": "go",
            "size": self.size,
            "board": self.board_to_json(),
            "current_turn": match self.current_turn {
                Stone::Black => "black",
                Stone::White => "white",
            },
            "status": self.status,
            "captures_black": self.captures_black,
            "captures_white": self.captures_white,
            "ko_point": self.ko_point.map(|(r, c)| json!([r, c])),
            "consecutive_passes": self.consecutive_passes,
            "move_count": self.move_count,
            "score_black": self.score_black,
            "score_white": self.score_white,
        })
    }

    fn status(&self) -> GameStatus {
        self.status.clone()
    }

    fn current_player(&self) -> PlayerColor {
        self.current_turn.to_player()
    }

    fn game_type(&self) -> GameType {
        GameType::Go
    }

    fn clone_box(&self) -> Box<dyn super::GameLogic> {
        Box::new(self.clone())
    }

    fn get_ai_move(&self, difficulty: AiDifficulty) -> Option<Value> {
        let color = self.current_turn;
        match difficulty {
            AiDifficulty::Easy => self.ai_go_random(),
            AiDifficulty::Normal => self.ai_go_heuristic(color),
            AiDifficulty::Hard => self.ai_go_hard(color),
        }
    }
}
