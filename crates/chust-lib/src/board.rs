use crate::error::{ChustError, Result};

/// Represents a detected chessboard's layout on the screen or image.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoardLayout {
    pub origin_x: u32,
    pub origin_y: u32,
    pub width: u32,
    pub height: u32,
    pub tile_size: u32,
    pub white_pov: bool,
}

impl BoardLayout {
    pub fn new(origin_x: u32, origin_y: u32, width: u32, height: u32, white_pov: bool) -> Self {
        let tile_size = ((width + height) / 2) / 8;
        Self {
            origin_x,
            origin_y,
            width,
            height,
            tile_size,
            white_pov,
        }
    }

    /// Converts algebraic notation (e.g., "e4") to pixel coordinates (x, y) at the tile center.
    pub fn notation_to_positions(&self, notation: &str) -> Option<(u32, u32)> {
        notation_to_positions(
            (self.origin_x, self.origin_y),
            self.tile_size,
            notation,
            self.white_pov,
        )
    }

    /// Converts algebraic notation to pixel coordinates, returning an explicit `ChustError::InvalidNotation` on error.
    pub fn try_notation_to_positions(&self, notation: &str) -> Result<(u32, u32)> {
        try_notation_to_positions(
            (self.origin_x, self.origin_y),
            self.tile_size,
            notation,
            self.white_pov,
        )
    }

    /// Converts pixel coordinates (x, y) on the screen to algebraic square notation (e.g. "e4").
    pub fn pixel_to_notation(&self, px: u32, py: u32) -> Option<String> {
        if px < self.origin_x || py < self.origin_y {
            return None;
        }
        let col = (px - self.origin_x) / self.tile_size;
        let row = (py - self.origin_y) / self.tile_size;
        if col >= 8 || row >= 8 {
            return None;
        }

        let (file_idx, rank_num) = if self.white_pov {
            (col, 8 - row)
        } else {
            (7 - col, row + 1)
        };

        if file_idx > 7 || rank_num < 1 || rank_num > 8 {
            return None;
        }

        let file_char = (b'a' + file_idx as u8) as char;
        Some(format!("{}{}", file_char, rank_num))
    }
}

/// Converts chess algebraic notation (e.g. "e2") to screen coordinates (x, y) centered on the square.
pub fn notation_to_positions(
    board_cords: (u32, u32),
    tile_size: u32,
    notation: &str,
    is_white_pov: bool,
) -> Option<(u32, u32)> {
    if notation.len() != 2 {
        return None;
    }

    let file = notation.chars().nth(0)?;
    let rank = notation.chars().nth(1)?.to_digit(10)?;

    if !('a'..='h').contains(&file) || !(1..=8).contains(&rank) {
        return None;
    }

    let (board_start_x, board_start_y) = board_cords;
    let file_index = (file as u8 - b'a') as u32;

    let (file_pos, rank_pos) = if is_white_pov {
        (file_index, 8 - rank)
    } else {
        ((7 - file_index), rank - 1)
    };

    let x = board_start_x + (tile_size / 2) + (tile_size * file_pos);
    let y = board_start_y + (tile_size / 2) + (tile_size * rank_pos);

    Some((x, y))
}

/// Converts chess algebraic notation to screen coordinates, returning a typed `ChustError` on invalid input.
pub fn try_notation_to_positions(
    board_cords: (u32, u32),
    tile_size: u32,
    notation: &str,
    is_white_pov: bool,
) -> Result<(u32, u32)> {
    notation_to_positions(board_cords, tile_size, notation, is_white_pov)
        .ok_or_else(|| ChustError::InvalidNotation(notation.to_string()))
}
