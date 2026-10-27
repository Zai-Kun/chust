/// Represents a difference on a single square between two board states.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SquareDiff {
    pub square: String, // e.g. "e4"
    pub before: char,
    pub after: char,
}

/// Deduces the UCI move (e.g. "e2e4", "e1g1") between two 8x8 board states.
/// `board` array is indexed [rank_idx][file_idx] where rank 0 is 8th rank, rank 7 is 1st rank.
pub fn deduce_move(
    before: &[[char; 8]; 8],
    after: &[[char; 8]; 8],
) -> Option<String> {
    let mut diffs: Vec<SquareDiff> = Vec::new();

    for r in 0..8 {
        for c in 0..8 {
            if before[r][c] != after[r][c] {
                let file_char = (b'a' + c as u8) as char;
                let rank_num = 8 - r;
                diffs.push(SquareDiff {
                    square: format!("{}{}", file_char, rank_num),
                    before: before[r][c],
                    after: after[r][c],
                });
            }
        }
    }

    if diffs.is_empty() {
        return None;
    }

    // Standard move or capture: exactly 2 squares changed
    if diffs.len() == 2 {
        let from = diffs.iter().find(|d| d.after == ' ');
        let to = diffs.iter().find(|d| d.after != ' ');

        if let (Some(f), Some(t)) = (from, to) {
            // Check for pawn promotion (pawn reaches 8th or 1st rank)
            let is_white_promo = f.before == 'P' && t.square.ends_with('8');
            let is_black_promo = f.before == 'p' && t.square.ends_with('1');

            if is_white_promo || is_black_promo {
                let promo_char = t.after.to_ascii_lowercase();
                return Some(format!("{}{}{}", f.square, t.square, promo_char));
            }
            return Some(format!("{}{}", f.square, t.square));
        }
    }

    // Castling: 4 squares changed (King and Rook moved)
    if diffs.len() == 4 {
        // White King-side: e1 -> g1, h1 -> f1
        if diffs.iter().any(|d| d.square == "e1" && d.after == ' ')
            && diffs.iter().any(|d| d.square == "g1" && d.after == 'K')
        {
            return Some("e1g1".to_string());
        }
        // White Queen-side: e1 -> c1, a1 -> d1
        if diffs.iter().any(|d| d.square == "e1" && d.after == ' ')
            && diffs.iter().any(|d| d.square == "c1" && d.after == 'K')
        {
            return Some("e1c1".to_string());
        }
        // Black King-side: e8 -> g8, h8 -> f8
        if diffs.iter().any(|d| d.square == "e8" && d.after == ' ')
            && diffs.iter().any(|d| d.square == "g8" && d.after == 'k')
        {
            return Some("e8g8".to_string());
        }
        // Black Queen-side: e8 -> c8, a8 -> d8
        if diffs.iter().any(|d| d.square == "e8" && d.after == ' ')
            && diffs.iter().any(|d| d.square == "c8" && d.after == 'k')
        {
            return Some("e8c8".to_string());
        }
    }

    // En Passant: 3 squares changed (pawn moved diagonally, adjacent enemy pawn disappeared)
    if diffs.len() == 3 {
        let from_pawn = diffs.iter().find(|d| (d.before == 'P' || d.before == 'p') && d.after == ' ');
        let to_pawn = diffs.iter().find(|d| (d.after == 'P' || d.after == 'p') && d.before == ' ');
        if let (Some(f), Some(t)) = (from_pawn, to_pawn) {
            return Some(format!("{}{}", f.square, t.square));
        }
    }

    None
}

/// A debouncing state tracker for moves across video / screenshot frames.
/// Prevents false move detection during in-flight drag animations.
pub struct MoveDetector {
    confirmed_board: [[char; 8]; 8],
    pending_board: Option<[[char; 8]; 8]>,
    stable_count: usize,
    required_stability: usize,
}

impl MoveDetector {
    pub fn new(initial_board: [[char; 8]; 8], required_stability: usize) -> Self {
        Self {
            confirmed_board: initial_board,
            pending_board: None,
            stable_count: 0,
            required_stability: required_stability.max(1),
        }
    }

    /// Feed a new board detection frame.
    /// Returns `Some(uci_move)` ONLY when the new position has stabilized for `required_stability` frames.
    pub fn update(&mut self, new_board: [[char; 8]; 8]) -> Option<String> {
        if new_board == self.confirmed_board {
            self.pending_board = None;
            self.stable_count = 0;
            return None;
        }

        if let Some(ref pending) = self.pending_board {
            if *pending == new_board {
                self.stable_count += 1;
            } else {
                self.pending_board = Some(new_board);
                self.stable_count = 1;
                return None;
            }
        } else {
            self.pending_board = Some(new_board);
            self.stable_count = 1;
            return None;
        }

        if self.stable_count >= self.required_stability {
            let mv = deduce_move(&self.confirmed_board, &new_board);
            self.confirmed_board = new_board;
            self.pending_board = None;
            self.stable_count = 0;
            return mv;
        }

        None
    }

    pub fn confirmed_board(&self) -> &[[char; 8]; 8] {
        &self.confirmed_board
    }
}
