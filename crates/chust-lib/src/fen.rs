use ndarray::{ArrayBase, Axis, IxDyn, OwnedRepr};

pub static PIECE_MAP: [char; 12] = ['p', 'r', 'n', 'b', 'q', 'k', 'P', 'R', 'N', 'B', 'Q', 'K'];

/// Converts model detection outputs into an 8x8 character array.
pub fn output_to_board_array(
    output: &ArrayBase<OwnedRepr<f32>, IxDyn>,
    board_cords: (u32, u32),
    board_size: (u32, u32),
    confidence_threshold: f32,
    white_pov: bool,
) -> [[char; 8]; 8] {
    let filtered_output = output
        .axis_iter(Axis(0))
        .filter(|row| row[4] >= confidence_threshold && row[5] != 12.0);
    let cell_size: u32 = ((board_size.0 + board_size.1) / 2) / 8;
    let half_cell_size = cell_size as f32 / 2.0;

    let mut board = [[' '; 8]; 8];

    for detection in filtered_output {
        let (x, y) = (
            (detection[0]) + half_cell_size,
            (detection[1]) + half_cell_size,
        );

        let x_location = ((x - board_cords.0 as f32) / cell_size as f32).ceil() as usize;
        let y_location = ((y - board_cords.1 as f32) / cell_size as f32).ceil() as usize;

        if x_location > 8 || x_location < 1 || y_location > 8 || y_location < 1 {
            continue;
        }

        if let Some(&piece) = PIECE_MAP.get(detection[5] as usize) {
            if white_pov {
                board[y_location - 1][x_location - 1] = piece;
            } else {
                board[8 - y_location][8 - x_location] = piece;
            }
        }
    }

    board
}

/// Converts an 8x8 character array into a FEN piece placement string (e.g. "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR").
pub fn board_to_fen(board: &[[char; 8]; 8]) -> String {
    let mut fen = String::with_capacity(64 + 7);
    for row in board {
        let mut empty_count: u8 = 0;
        for &cell in row.iter() {
            if cell == ' ' {
                empty_count += 1;
            } else {
                if empty_count > 0 {
                    fen.push((b'0' + empty_count) as char);
                    empty_count = 0;
                }
                fen.push(cell);
            }
        }
        if empty_count > 0 {
            fen.push((b'0' + empty_count) as char);
        }
        fen.push('/');
    }
    fen.pop(); // Remove trailing '/'
    fen
}

/// Helper that converts detection output directly to a FEN string.
pub fn output_to_fen(
    output: &ArrayBase<OwnedRepr<f32>, IxDyn>,
    board_cords: (u32, u32),
    board_size: (u32, u32),
    confidence_threshold: f32,
    white_pov: bool,
) -> String {
    let board = output_to_board_array(
        output,
        board_cords,
        board_size,
        confidence_threshold,
        white_pov,
    );
    board_to_fen(&board)
}
