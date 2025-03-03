use clap::{Parser, ValueEnum};

/// A tool for detecting chessboards and chess pieces in images.
///
/// This tool accepts either a file path to an image or "-" to read image data from standard input. When reading from
/// standard input, the first byte specifies the point of view (POV), the next 4 bytes indicate the image size, and
/// the remaining bytes are the image data.
///
/// It supports:
/// - Extracting FEN notation from the best chessboard match.
/// - Detecting and printing positions of chess pieces and board.
/// - Saving an output image with all detections marked.
/// - A continuous mode (when using piped input) that processes images until terminated.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to the image file to process, or "-" to read from the input pipe.
    pub image_path: String,

    /// Don't attempt to extract FEN notation from the best chessboard detection (default: false).
    #[arg(long="no-fen", default_value_t = false)]
    pub no_fen: bool,

    /// Specify the point of view for detection. Accepts "w" for white or "b" for black (default: w).
    /// This option is ignored if --no-fen is set to true AND if the input mode is standard input.
    #[arg(long, value_enum, default_value_t = Pov::W)]
    pub pov: Pov,

    /// Enable refined search mode. This mode first detects the chessboard, crops the board with padding,
    /// and performs a second detection for improved accuracy on smaller boards (default: false).
    #[arg(long, default_value_t = false)]
    pub refined_search: bool,

    /// Print the coordinates (x, y, width, height) of the detections.
    /// If --best-chessboard-detection-only is enabled, only the detections used for FEN extraction are printed.
    #[arg(long, default_value_t = false)]
    pub print_detections: bool,

    /// Print only the best chessboard detection and its associated pieces for FEN extraction.
    /// This flag is ignored if --no-fen is true (default: false).
    #[arg(long, default_value_t = false)]
    pub best_chessboard_detection_only: bool,

    /// Confidence threshold for detections (default: 0.7).
    #[arg(long, default_value_t = 0.7)]
    pub conf: f32,

    /// Padding to add around the cropped chessboard detection before performing a refined detection (default: 0.1).
    #[arg(long, default_value_t = 0.1)]
    pub refined_padding: f32,

    /// Do not exit after processing an image; continuously wait for additional images from the input pipe.
    /// This option is ignored if a file path is not "-" (default: false).
    #[arg(long, default_value_t = false)]
    pub dont_exit: bool,

    /// If specified, the tool will annotate the original image with all detections and save it at the given path. You can also give it "-" to write to the output pipe.
    #[arg(long)]
    pub output_path: Option<String>,

    /// Path to the onnx model file for chessboard detection (default: "chess_detection.onnx").
    #[arg(long, default_value_t = String::from("chess_detection.onnx"))]
    pub model_path: String,
}

/// Represents the point of view (POV) for chessboard detection.
#[derive(ValueEnum, Debug, Clone, PartialEq)]
pub enum Pov {
    /// White's point of view.
    W,
    /// Black's point of view.
    B,
}
