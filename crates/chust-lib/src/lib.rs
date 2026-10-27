pub mod board;
pub mod drawing;
pub mod error;
pub mod fen;
pub mod geometry;
pub mod model;
pub mod move_detector;

// Re-exports for convenient top-level usage
pub use board::{notation_to_positions, try_notation_to_positions, BoardLayout};
pub use drawing::{annotate_detections, draw_bounding_box, draw_label, get_font};
pub use error::{ChustError, Result};
pub use fen::{board_to_fen, output_to_board_array, output_to_fen, PIECE_MAP};
pub use geometry::{crop_with_padding, letterbox_resize, process_image, scale_bbox};
pub use model::{get_best_chessboard_match, ChessDetection, DetectionLevel};
pub use move_detector::{deduce_move, MoveDetector, SquareDiff};
