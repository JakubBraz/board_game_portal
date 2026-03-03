use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use super::{AiDifficulty, GameLogic, GameStatus, GameType, PlayerColor};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PieceType {
    King,
    Queen,
    Rook,
    Bishop,
    Knight,
    Pawn,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Piece {
    pub piece_type: PieceType,
    pub color: Color,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Color {
    White,
    Black,
}

impl Color {
    fn opposite(&self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
    fn to_player(&self) -> PlayerColor {
        match self {
            Color::White => PlayerColor::White,
            Color::Black => PlayerColor::Black,
        }
    }
}

type Board = [[Option<Piece>; 8]; 8];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CastlingRights {
    pub white_king_side: bool,
    pub white_queen_side: bool,
    pub black_king_side: bool,
    pub black_queen_side: bool,
}

#[derive(Debug, Clone)]
pub struct ChessGame {
    pub board: Board,
    pub current_turn: Color,
    pub castling: CastlingRights,
    pub en_passant: Option<(usize, usize)>,
    pub half_move_clock: u32,
    pub full_move_number: u32,
    pub status: GameStatus,
    pub last_move: Option<(usize, usize, usize, usize)>,
    pub position_history: HashMap<String, u8>,
}

impl ChessGame {
    pub fn new() -> Self {
        let mut board: Board = [[None; 8]; 8];

        // White pieces on rows 0-1
        let back_row = [
            PieceType::Rook,
            PieceType::Knight,
            PieceType::Bishop,
            PieceType::Queen,
            PieceType::King,
            PieceType::Bishop,
            PieceType::Knight,
            PieceType::Rook,
        ];
        for (col, &pt) in back_row.iter().enumerate() {
            board[0][col] = Some(Piece { piece_type: pt, color: Color::White });
            board[7][col] = Some(Piece { piece_type: pt, color: Color::Black });
        }
        for col in 0..8 {
            board[1][col] = Some(Piece { piece_type: PieceType::Pawn, color: Color::White });
            board[6][col] = Some(Piece { piece_type: PieceType::Pawn, color: Color::Black });
        }

        let mut game = ChessGame {
            board,
            current_turn: Color::White,
            castling: CastlingRights {
                white_king_side: true,
                white_queen_side: true,
                black_king_side: true,
                black_queen_side: true,
            },
            en_passant: None,
            half_move_clock: 0,
            full_move_number: 1,
            status: GameStatus::Playing,
            last_move: None,
            position_history: HashMap::new(),
        };
        let key = game.position_key();
        game.position_history.insert(key, 1);
        game
    }

    fn piece_at(&self, row: usize, col: usize) -> Option<Piece> {
        self.board[row][col]
    }

    fn find_king(&self, color: Color) -> Option<(usize, usize)> {
        for row in 0..8 {
            for col in 0..8 {
                if let Some(p) = self.board[row][col] {
                    if p.piece_type == PieceType::King && p.color == color {
                        return Some((row, col));
                    }
                }
            }
        }
        None
    }

    fn is_square_attacked(&self, row: usize, col: usize, by_color: Color) -> bool {
        for r in 0..8usize {
            for c in 0..8usize {
                if let Some(p) = self.board[r][c] {
                    if p.color == by_color {
                        let moves = self.pseudo_legal_moves(r, c, false);
                        if moves.contains(&(row, col)) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    fn is_in_check(&self, color: Color) -> bool {
        if let Some((kr, kc)) = self.find_king(color) {
            self.is_square_attacked(kr, kc, color.opposite())
        } else {
            false
        }
    }

    // pseudo_legal_moves ignores check. include_castling controls whether to include castling moves.
    fn pseudo_legal_moves(&self, row: usize, col: usize, include_castling: bool) -> Vec<(usize, usize)> {
        let mut moves = Vec::new();
        let piece = match self.board[row][col] {
            Some(p) => p,
            None => return moves,
        };

        let color = piece.color;

        let add_if_valid = |moves: &mut Vec<(usize, usize)>, r: i32, c: i32| {
            if r >= 0 && r < 8 && c >= 0 && c < 8 {
                let (r, c) = (r as usize, c as usize);
                match self.board[r][c] {
                    None => { moves.push((r, c)); true }
                    Some(p) if p.color != color => { moves.push((r, c)); false }
                    _ => false,
                }
            } else {
                false
            }
        };

        let slide = |moves: &mut Vec<(usize, usize)>, dr: i32, dc: i32| {
            let mut r = row as i32 + dr;
            let mut c = col as i32 + dc;
            while r >= 0 && r < 8 && c >= 0 && c < 8 {
                let (ri, ci) = (r as usize, c as usize);
                match self.board[ri][ci] {
                    None => { moves.push((ri, ci)); }
                    Some(p) if p.color != color => { moves.push((ri, ci)); break; }
                    _ => break,
                }
                r += dr;
                c += dc;
            }
        };

        match piece.piece_type {
            PieceType::Pawn => {
                let dir: i32 = if color == Color::White { 1 } else { -1 };
                let start_row = if color == Color::White { 1 } else { 6 };
                let r = row as i32 + dir;
                // Forward move
                if r >= 0 && r < 8 && self.board[r as usize][col].is_none() {
                    moves.push((r as usize, col));
                    // Double forward from start
                    if row == start_row {
                        let r2 = row as i32 + 2 * dir;
                        if self.board[r2 as usize][col].is_none() {
                            moves.push((r2 as usize, col));
                        }
                    }
                }
                // Diagonal captures
                for dc in [-1i32, 1] {
                    let c = col as i32 + dc;
                    if r >= 0 && r < 8 && c >= 0 && c < 8 {
                        let (ri, ci) = (r as usize, c as usize);
                        if let Some(p) = self.board[ri][ci] {
                            if p.color != color {
                                moves.push((ri, ci));
                            }
                        }
                        // En passant
                        if let Some((ep_r, ep_c)) = self.en_passant {
                            if ri == ep_r && ci == ep_c {
                                moves.push((ri, ci));
                            }
                        }
                    }
                }
            }
            PieceType::Knight => {
                for (dr, dc) in [(-2,-1),(-2,1),(-1,-2),(-1,2),(1,-2),(1,2),(2,-1),(2,1)] {
                    add_if_valid(&mut moves, row as i32 + dr, col as i32 + dc);
                }
            }
            PieceType::Bishop => {
                for (dr, dc) in [(-1,-1),(-1,1),(1,-1),(1,1)] {
                    slide(&mut moves, dr, dc);
                }
            }
            PieceType::Rook => {
                for (dr, dc) in [(-1,0),(1,0),(0,-1),(0,1)] {
                    slide(&mut moves, dr, dc);
                }
            }
            PieceType::Queen => {
                for (dr, dc) in [(-1,-1),(-1,0),(-1,1),(0,-1),(0,1),(1,-1),(1,0),(1,1)] {
                    slide(&mut moves, dr, dc);
                }
            }
            PieceType::King => {
                for (dr, dc) in [(-1,-1),(-1,0),(-1,1),(0,-1),(0,1),(1,-1),(1,0),(1,1)] {
                    add_if_valid(&mut moves, row as i32 + dr, col as i32 + dc);
                }
                // Castling
                if include_castling && !self.is_in_check(color) {
                    let back_row = if color == Color::White { 0 } else { 7 };
                    if row == back_row && col == 4 {
                        // King-side
                        let ks = if color == Color::White { self.castling.white_king_side } else { self.castling.black_king_side };
                        if ks && self.board[back_row][5].is_none() && self.board[back_row][6].is_none() {
                            if !self.is_square_attacked(back_row, 5, color.opposite())
                                && !self.is_square_attacked(back_row, 6, color.opposite()) {
                                moves.push((back_row, 6));
                            }
                        }
                        // Queen-side
                        let qs = if color == Color::White { self.castling.white_queen_side } else { self.castling.black_queen_side };
                        if qs && self.board[back_row][3].is_none() && self.board[back_row][2].is_none() && self.board[back_row][1].is_none() {
                            if !self.is_square_attacked(back_row, 3, color.opposite())
                                && !self.is_square_attacked(back_row, 2, color.opposite()) {
                                moves.push((back_row, 2));
                            }
                        }
                    }
                }
            }
        }
        moves
    }

    fn legal_moves(&self, row: usize, col: usize) -> Vec<(usize, usize)> {
        let piece = match self.board[row][col] {
            Some(p) => p,
            None => return vec![],
        };
        let color = piece.color;
        let pseudo = self.pseudo_legal_moves(row, col, true);
        pseudo.into_iter().filter(|&(tr, tc)| {
            let mut test = self.clone();
            test.apply_move_raw(row, col, tr, tc, None);
            !test.is_in_check(color)
        }).collect()
    }

    fn apply_move_raw(&mut self, from_r: usize, from_c: usize, to_r: usize, to_c: usize, promotion: Option<PieceType>) {
        let piece = match self.board[from_r][from_c] {
            Some(p) => p,
            None => return,
        };

        // En passant capture
        if piece.piece_type == PieceType::Pawn {
            if let Some((ep_r, ep_c)) = self.en_passant {
                if to_r == ep_r && to_c == ep_c {
                    let capture_row = if piece.color == Color::White { to_r - 1 } else { to_r + 1 };
                    self.board[capture_row][to_c] = None;
                }
            }
        }

        // Castling rook move
        if piece.piece_type == PieceType::King {
            let back_row = if piece.color == Color::White { 0 } else { 7 };
            if from_r == back_row && from_c == 4 {
                if to_c == 6 {
                    // King-side
                    self.board[back_row][5] = self.board[back_row][7];
                    self.board[back_row][7] = None;
                } else if to_c == 2 {
                    // Queen-side
                    self.board[back_row][3] = self.board[back_row][0];
                    self.board[back_row][0] = None;
                }
            }
            if piece.color == Color::White {
                self.castling.white_king_side = false;
                self.castling.white_queen_side = false;
            } else {
                self.castling.black_king_side = false;
                self.castling.black_queen_side = false;
            }
        }

        // Update castling rights when rook moves
        if piece.piece_type == PieceType::Rook {
            match (piece.color, from_r, from_c) {
                (Color::White, 0, 0) => self.castling.white_queen_side = false,
                (Color::White, 0, 7) => self.castling.white_king_side = false,
                (Color::Black, 7, 0) => self.castling.black_queen_side = false,
                (Color::Black, 7, 7) => self.castling.black_king_side = false,
                _ => {}
            }
        }

        // Update en passant target
        self.en_passant = None;
        if piece.piece_type == PieceType::Pawn {
            let diff = (to_r as i32 - from_r as i32).abs();
            if diff == 2 {
                let ep_row = (from_r + to_r) / 2;
                self.en_passant = Some((ep_row, from_c));
            }
        }

        // Move piece
        let moved_piece = if piece.piece_type == PieceType::Pawn {
            let promo_row = if piece.color == Color::White { 7 } else { 0 };
            if to_r == promo_row {
                Piece {
                    piece_type: promotion.unwrap_or(PieceType::Queen),
                    color: piece.color,
                }
            } else {
                piece
            }
        } else {
            piece
        };

        self.board[to_r][to_c] = Some(moved_piece);
        self.board[from_r][from_c] = None;
    }

    fn has_any_legal_move(&self, color: Color) -> bool {
        for row in 0..8 {
            for col in 0..8 {
                if let Some(p) = self.board[row][col] {
                    if p.color == color && !self.legal_moves(row, col).is_empty() {
                        return true;
                    }
                }
            }
        }
        false
    }

    fn position_key(&self) -> String {
        let mut key = String::with_capacity(72);
        for row in 0..8 {
            for col in 0..8 {
                key.push(match self.board[row][col] {
                    None => '.',
                    Some(p) => match (p.piece_type, p.color) {
                        (PieceType::King,   Color::White) => 'K',
                        (PieceType::Queen,  Color::White) => 'Q',
                        (PieceType::Rook,   Color::White) => 'R',
                        (PieceType::Bishop, Color::White) => 'B',
                        (PieceType::Knight, Color::White) => 'N',
                        (PieceType::Pawn,   Color::White) => 'P',
                        (PieceType::King,   Color::Black) => 'k',
                        (PieceType::Queen,  Color::Black) => 'q',
                        (PieceType::Rook,   Color::Black) => 'r',
                        (PieceType::Bishop, Color::Black) => 'b',
                        (PieceType::Knight, Color::Black) => 'n',
                        (PieceType::Pawn,   Color::Black) => 'p',
                    },
                });
            }
        }
        key.push(if self.current_turn == Color::White { 'w' } else { 'b' });
        key.push(if self.castling.white_king_side  { 'K' } else { '-' });
        key.push(if self.castling.white_queen_side { 'Q' } else { '-' });
        key.push(if self.castling.black_king_side  { 'k' } else { '-' });
        key.push(if self.castling.black_queen_side { 'q' } else { '-' });
        match self.en_passant {
            None => key.push_str("--"),
            Some((r, c)) => {
                key.push((b'a' + c as u8) as char);
                key.push((b'1' + r as u8) as char);
            }
        }
        key
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

    fn all_legal_moves_for(&self, color: Color) -> Vec<(usize, usize, usize, usize)> {
        let mut moves = Vec::new();
        for row in 0..8 {
            for col in 0..8 {
                if let Some(p) = self.board[row][col] {
                    if p.color == color {
                        for (tr, tc) in self.legal_moves(row, col) {
                            moves.push((row, col, tr, tc));
                        }
                    }
                }
            }
        }
        moves
    }

    fn ai_random(&self, color: Color) -> Option<Value> {
        let moves = self.all_legal_moves_for(color);
        if moves.is_empty() { return None; }
        let color_val: usize = match color { Color::White => 0, Color::Black => 1 };
        let idx = (self.full_move_number as usize * 6991 + color_val * 31) % moves.len();
        let (fr, fc, tr, tc) = moves[idx];
        Some(json!({ "from": [fr, fc], "to": [tr, tc] }))
    }

    fn material_value(pt: PieceType) -> i32 {
        match pt {
            PieceType::Pawn => 100,
            PieceType::Knight => 320,
            PieceType::Bishop => 330,
            PieceType::Rook => 500,
            PieceType::Queen => 900,
            PieceType::King => 20000,
        }
    }

    // Piece-square tables indexed [row][col] from white's perspective (row 0 = rank 1)
    fn pst_bonus(pt: PieceType, color: Color, row: usize, col: usize) -> i32 {
        let r = if color == Color::White { row } else { 7 - row };
        match pt {
            PieceType::Pawn => {
                const T: [[i32;8];8] = [
                    [0,0,0,0,0,0,0,0],
                    [5,10,10,-20,-20,10,10,5],
                    [5,-5,-10,0,0,-10,-5,5],
                    [0,0,0,20,20,0,0,0],
                    [5,5,10,25,25,10,5,5],
                    [10,10,20,30,30,20,10,10],
                    [50,50,50,50,50,50,50,50],
                    [0,0,0,0,0,0,0,0],
                ];
                T[r][col]
            }
            PieceType::Knight => {
                const T: [[i32;8];8] = [
                    [-50,-40,-30,-30,-30,-30,-40,-50],
                    [-40,-20,0,0,0,0,-20,-40],
                    [-30,0,10,15,15,10,0,-30],
                    [-30,5,15,20,20,15,5,-30],
                    [-30,0,15,20,20,15,0,-30],
                    [-30,5,10,15,15,10,5,-30],
                    [-40,-20,0,5,5,0,-20,-40],
                    [-50,-40,-30,-30,-30,-30,-40,-50],
                ];
                T[r][col]
            }
            PieceType::Bishop => {
                const T: [[i32;8];8] = [
                    [-20,-10,-10,-10,-10,-10,-10,-20],
                    [-10,0,0,0,0,0,0,-10],
                    [-10,0,5,10,10,5,0,-10],
                    [-10,5,5,10,10,5,5,-10],
                    [-10,0,10,10,10,10,0,-10],
                    [-10,10,10,10,10,10,10,-10],
                    [-10,5,0,0,0,0,5,-10],
                    [-20,-10,-10,-10,-10,-10,-10,-20],
                ];
                T[r][col]
            }
            PieceType::Rook => {
                const T: [[i32;8];8] = [
                    [0,0,0,0,0,0,0,0],
                    [5,10,10,10,10,10,10,5],
                    [-5,0,0,0,0,0,0,-5],
                    [-5,0,0,0,0,0,0,-5],
                    [-5,0,0,0,0,0,0,-5],
                    [-5,0,0,0,0,0,0,-5],
                    [-5,0,0,0,0,0,0,-5],
                    [0,0,0,5,5,0,0,0],
                ];
                T[r][col]
            }
            PieceType::Queen => {
                const T: [[i32;8];8] = [
                    [-20,-10,-10,-5,-5,-10,-10,-20],
                    [-10,0,0,0,0,0,0,-10],
                    [-10,0,5,5,5,5,0,-10],
                    [-5,0,5,5,5,5,0,-5],
                    [0,0,5,5,5,5,0,-5],
                    [-10,5,5,5,5,5,0,-10],
                    [-10,0,5,0,0,0,0,-10],
                    [-20,-10,-10,-5,-5,-10,-10,-20],
                ];
                T[r][col]
            }
            PieceType::King => {
                // Middlegame king safety
                const T: [[i32;8];8] = [
                    [20,30,10,0,0,10,30,20],
                    [20,20,0,0,0,0,20,20],
                    [-10,-20,-20,-20,-20,-20,-20,-10],
                    [-20,-30,-30,-40,-40,-30,-30,-20],
                    [-30,-40,-40,-50,-50,-40,-40,-30],
                    [-30,-40,-40,-50,-50,-40,-40,-30],
                    [-30,-40,-40,-50,-50,-40,-40,-30],
                    [-30,-40,-40,-50,-50,-40,-40,-30],
                ];
                T[r][col]
            }
        }
    }

    fn evaluate(&self, for_color: Color) -> i32 {
        if self.status == GameStatus::WhiteWon {
            return if for_color == Color::White { 90000 } else { -90000 };
        }
        if self.status == GameStatus::BlackWon {
            return if for_color == Color::Black { 90000 } else { -90000 };
        }
        if self.status == GameStatus::Draw {
            return 0;
        }
        let mut score = 0i32;
        for row in 0..8 {
            for col in 0..8 {
                if let Some(p) = self.board[row][col] {
                    let v = Self::material_value(p.piece_type) + Self::pst_bonus(p.piece_type, p.color, row, col);
                    if p.color == for_color { score += v; } else { score -= v; }
                }
            }
        }
        score
    }

    fn minimax(&self, depth: u8, mut alpha: i32, mut beta: i32, maximizing: bool, for_color: Color) -> i32 {
        if self.status != GameStatus::Playing {
            return self.evaluate(for_color);
        }
        if depth == 0 {
            return self.evaluate(for_color);
        }
        let current = if maximizing { for_color } else { for_color.opposite() };
        let mut moves = self.all_legal_moves_for(current);
        if moves.is_empty() {
            return self.evaluate(for_color);
        }
        // Move ordering: captures first (sorted by victim value)
        moves.sort_by_key(|&(_, _, tr, tc)| {
            if let Some(victim) = self.board[tr][tc] {
                -(Self::material_value(victim.piece_type))
            } else {
                0
            }
        });
        if maximizing {
            let mut best = i32::MIN + 1;
            for (fr, fc, tr, tc) in moves {
                let mut next = self.clone();
                next.apply_move_raw(fr, fc, tr, tc, Some(PieceType::Queen));
                next.current_turn = next.current_turn.opposite();
                // Update status for terminal detection
                let nc = next.current_turn;
                if !next.has_any_legal_move(nc) {
                    next.status = if next.is_in_check(nc) {
                        if nc == Color::White { GameStatus::BlackWon } else { GameStatus::WhiteWon }
                    } else { GameStatus::Draw };
                }
                let val = next.minimax(depth - 1, alpha, beta, false, for_color);
                if val > best { best = val; }
                if best > alpha { alpha = best; }
                if alpha >= beta { break; }
            }
            best
        } else {
            let mut best = i32::MAX - 1;
            for (fr, fc, tr, tc) in moves {
                let mut next = self.clone();
                next.apply_move_raw(fr, fc, tr, tc, Some(PieceType::Queen));
                next.current_turn = next.current_turn.opposite();
                let nc = next.current_turn;
                if !next.has_any_legal_move(nc) {
                    next.status = if next.is_in_check(nc) {
                        if nc == Color::White { GameStatus::BlackWon } else { GameStatus::WhiteWon }
                    } else { GameStatus::Draw };
                }
                let val = next.minimax(depth - 1, alpha, beta, true, for_color);
                if val < best { best = val; }
                if best < beta { beta = best; }
                if alpha >= beta { break; }
            }
            best
        }
    }

    fn ai_minimax_root(&self, color: Color, depth: u8) -> Option<Value> {
        let mut moves = self.all_legal_moves_for(color);
        if moves.is_empty() { return None; }
        moves.sort_by_key(|&(_, _, tr, tc)| {
            if let Some(victim) = self.board[tr][tc] { -(Self::material_value(victim.piece_type)) } else { 0 }
        });
        let mut best_val = i32::MIN + 1;
        let mut best_move = moves[0];
        for (fr, fc, tr, tc) in &moves {
            let mut next = self.clone();
            next.apply_move_raw(*fr, *fc, *tr, *tc, Some(PieceType::Queen));
            next.current_turn = next.current_turn.opposite();
            let nc = next.current_turn;
            if !next.has_any_legal_move(nc) {
                next.status = if next.is_in_check(nc) {
                    if nc == Color::White { GameStatus::BlackWon } else { GameStatus::WhiteWon }
                } else { GameStatus::Draw };
            }
            let val = next.minimax(depth - 1, i32::MIN + 1, i32::MAX - 1, false, color);
            if val > best_val {
                best_val = val;
                best_move = (*fr, *fc, *tr, *tc);
            }
        }
        let (fr, fc, tr, tc) = best_move;
        // Check if promotion needed
        let promo = if let Some(p) = self.board[fr][fc] {
            if p.piece_type == PieceType::Pawn {
                let promo_row = if color == Color::White { 7 } else { 0 };
                if tr == promo_row { Some("queen") } else { None }
            } else { None }
        } else { None };
        if let Some(p) = promo {
            Some(json!({ "from": [fr, fc], "to": [tr, tc], "promotion": p }))
        } else {
            Some(json!({ "from": [fr, fc], "to": [tr, tc] }))
        }
    }
}

impl GameLogic for ChessGame {
    fn make_move(&mut self, mv: &Value, player: &PlayerColor) -> Result<(), String> {
        if self.status != GameStatus::Playing {
            return Err("Game is over".into());
        }
        let expected = self.current_turn.to_player();
        if expected != *player {
            return Err("Not your turn".into());
        }

        let from_r = mv["from"][0].as_u64().ok_or("invalid from row")? as usize;
        let from_c = mv["from"][1].as_u64().ok_or("invalid from col")? as usize;
        let to_r = mv["to"][0].as_u64().ok_or("invalid to row")? as usize;
        let to_c = mv["to"][1].as_u64().ok_or("invalid to col")? as usize;

        if from_r >= 8 || from_c >= 8 || to_r >= 8 || to_c >= 8 {
            return Err("Coordinates out of bounds".into());
        }

        let piece = self.board[from_r][from_c].ok_or("No piece at source")?;
        if piece.color.to_player() != *player {
            return Err("That piece doesn't belong to you".into());
        }

        let legal = self.legal_moves(from_r, from_c);
        if !legal.contains(&(to_r, to_c)) {
            return Err("Illegal move".into());
        }

        let promotion = mv["promotion"].as_str().and_then(|s| match s {
            "queen" => Some(PieceType::Queen),
            "rook" => Some(PieceType::Rook),
            "bishop" => Some(PieceType::Bishop),
            "knight" => Some(PieceType::Knight),
            _ => None,
        });

        self.apply_move_raw(from_r, from_c, to_r, to_c, promotion);
        self.last_move = Some((from_r, from_c, to_r, to_c));
        self.current_turn = self.current_turn.opposite();

        // Check game end conditions
        let next_color = self.current_turn;
        let in_check = self.is_in_check(next_color);
        let has_moves = self.has_any_legal_move(next_color);

        if !has_moves {
            if in_check {
                // Checkmate
                self.status = match next_color {
                    Color::White => GameStatus::BlackWon,
                    Color::Black => GameStatus::WhiteWon,
                };
            } else {
                // Stalemate
                self.status = GameStatus::Draw;
            }
        }

        // Threefold repetition
        if self.status == GameStatus::Playing {
            let key = self.position_key();
            let count = self.position_history.entry(key).or_insert(0);
            *count += 1;
            if *count >= 3 {
                self.status = GameStatus::Draw;
            }
        }

        if self.current_turn == Color::White {
            self.full_move_number += 1;
        }

        Ok(())
    }

    fn to_json(&self) -> Value {
        let legal_moves_map: Vec<Value> = {
            let mut all = Vec::new();
            if self.status == GameStatus::Playing {
                for row in 0..8 {
                    for col in 0..8 {
                        if let Some(p) = self.board[row][col] {
                            if p.color.to_player() == self.current_turn.to_player() {
                                let lm = self.legal_moves(row, col);
                                if !lm.is_empty() {
                                    all.push(json!({
                                        "from": [row, col],
                                        "to": lm,
                                    }));
                                }
                            }
                        }
                    }
                }
            }
            all
        };

        json!({
            "type": "chess",
            "board": self.board_to_json(),
            "current_turn": format!("{:?}", self.current_turn).to_lowercase(),
            "status": self.status,
            "castling": {
                "white_king_side": self.castling.white_king_side,
                "white_queen_side": self.castling.white_queen_side,
                "black_king_side": self.castling.black_king_side,
                "black_queen_side": self.castling.black_queen_side,
            },
            "en_passant": self.en_passant.map(|(r, c)| json!([r, c])),
            "legal_moves": legal_moves_map,
            "last_move": self.last_move.map(|(fr, fc, tr, tc)| json!([[fr, fc], [tr, tc]])),
            "in_check": self.is_in_check(self.current_turn),
            "full_move_number": self.full_move_number,
        })
    }

    fn status(&self) -> GameStatus {
        self.status.clone()
    }

    fn current_player(&self) -> PlayerColor {
        self.current_turn.to_player()
    }

    fn game_type(&self) -> GameType {
        GameType::Chess
    }

    fn clone_box(&self) -> Box<dyn GameLogic> {
        Box::new(self.clone())
    }

    fn get_ai_move(&self, difficulty: AiDifficulty) -> Option<Value> {
        let color = match self.current_player() {
            PlayerColor::White => Color::White,
            PlayerColor::Black => Color::Black,
        };
        match difficulty {
            AiDifficulty::Easy => self.ai_random(color),
            AiDifficulty::Normal => self.ai_minimax_root(color, 3),
            AiDifficulty::Hard => self.ai_minimax_root(color, 5),
        }
    }
}
