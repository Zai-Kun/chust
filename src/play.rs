use crate::{
    arg_parser::Args,
    chess_detection::{get_best_chessboard_match, ChessDetection, DetectionLevel},
    input_capture::InputCaptureTrait,
    stockfish::Stockfish,
};
use anyhow::{Context, Result};
use ndarray::{ArrayBase, IxDyn, OwnedRepr};
use std::io::{self, Read};

pub fn play(
    screenshot_delay: f32,
    stockfish_depth: u32,
    mut stockfish: Stockfish,
    recheck_after_change: bool,
    move_delay: f32,
    args: &Args,
    chess_detector: &ChessDetection,
    mut input_capture: Box<dyn InputCaptureTrait>,
) -> Result<()> {
    let detection_level = if args.refined_search {
        DetectionLevel::Refined
    } else {
        DetectionLevel::Basic
    };
    let is_white_pov = args.pov == crate::arg_parser::Pov::W;

    let mut current_fen = "".to_string();
    loop {
        let (_current_fen, detection) = wait_for_changes(
            &current_fen,
            &detection_level,
            is_white_pov,
            &mut input_capture,
            chess_detector,
            screenshot_delay,
            recheck_after_change,
        )?;
        current_fen = _current_fen;

        let best_chessboard_match = get_best_chessboard_match(&detection).unwrap().0;
        let board_cords = (
            best_chessboard_match[0] as u32,
            best_chessboard_match[1] as u32,
        );
        let tile_size =
            ((best_chessboard_match[2] as u32 + best_chessboard_match[3] as u32) / 2) / 8;

        let castling = format!(
            "{}{}",
            if args.castle_w { "KQ" } else { "-" },
            if args.castle_b { "kq" } else { "-" },
        );
        let fen = format!(
            "{} {} {}",
            current_fen,
            if is_white_pov { "w" } else { "b" },
            castling
        );

        let best_move = stockfish.get_best_move(&fen, stockfish_depth)?;

        click_notation(
            board_cords,
            tile_size,
            &best_move[0..2],
            is_white_pov,
            &mut input_capture,
        )?;
        std::thread::sleep(std::time::Duration::from_secs_f32(move_delay));
        click_notation(
            board_cords,
            tile_size,
            &best_move[2..4],
            is_white_pov,
            &mut input_capture,
        )?;

        if best_move.len() == 5 {
            println!("Promotion move detected, promoting is not supported yet. Please manually promot to {} and press enter...", &best_move[4..5]);
            io::stdin().read_exact(&mut [0])?;
        }

        current_fen = stockfish.make_move_and_get_fen(&fen, &best_move)?;
    }
}

fn wait_for_changes(
    current_fen: &str,
    detection_level: &DetectionLevel,
    is_white_pov: bool,
    input_capture: &mut Box<dyn InputCaptureTrait>,
    chess_detector: &ChessDetection,
    screenshot_delay: f32,

    mut recheck_after_change: bool,
) -> Result<(String, ArrayBase<OwnedRepr<f32>, IxDyn>)> {
    loop {
        std::thread::sleep(std::time::Duration::from_secs_f32(screenshot_delay));

        let (fen, detection) = take_screenshot_and_get_fen(
            input_capture,
            chess_detector,
            is_white_pov,
            detection_level,
        )?;

        if fen == current_fen {
            continue;
        }

        if recheck_after_change {
            recheck_after_change = false;
            continue;
        }

        return Ok((fen, detection));
    }
}

fn take_screenshot_and_get_fen(
    input_capture: &mut Box<dyn InputCaptureTrait>,
    chess_detector: &ChessDetection,
    is_white_pov: bool,
    detection_level: &DetectionLevel,
) -> Result<(String, ArrayBase<OwnedRepr<f32>, IxDyn>)> {
    let screenshot = input_capture.screenshot()?;
    let detection = chess_detector
        .detect(&screenshot, &detection_level)
        .context("Detection failed")?
        .context("Board not found")?;
    let best_chessboard_match = get_best_chessboard_match(&detection)
        .context("Board not found")?
        .0;
    let fen = chess_detector.output_to_fen(
        &detection,
        (
            best_chessboard_match[0] as u32,
            best_chessboard_match[1] as u32,
        ),
        (
            best_chessboard_match[2] as u32,
            best_chessboard_match[3] as u32,
        ),
        is_white_pov,
    );
    Ok((fen, detection))
}

fn click_notation(
    board_cords: (u32, u32),
    tile_size: u32,
    notation: &str,
    is_white_pov: bool,
    input_capture: &mut Box<dyn InputCaptureTrait>,
) -> Result<()> {
    let (x, y) = notation_to_positions(board_cords, tile_size, notation, is_white_pov)
        .context("Invalid notation")?;
    input_capture.click_at(x, y)?;
    Ok(())
}

fn notation_to_positions(
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
