use clap::{Parser, Subcommand, ValueEnum};

/// A tool for processing an image and extracting chessboards, and pieces locations.
/// It can also play the game for you.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,

    /// Specify the point of view for detection. Accepts "w" for white or "b" for black (default: w).
    /// This option is ignored if --no-fen is set to true AND if the input mode is standard input.
    #[arg(global=true,long, value_enum, default_value_t = Pov::W)]
    pub pov: Pov,

    /// Enable refined search mode. This mode first detects the chessboard, crops the board with padding,
    /// and performs a second detection for improved accuracy on smaller boards (default: false).
    #[arg(global = true, long, default_value_t = false)]
    pub refined_search: bool,

    /// Confidence threshold for detections (default: 0.7).
    #[arg(global = true, long, default_value_t = 0.7)]
    pub conf: f32,

    /// Padding to add around the cropped chessboard detection before performing a refined detection (default: 0.1).
    #[arg(global = true, long, default_value_t = 0.1)]
    pub refined_padding: f32,

    /// Path to the onnx model file for chessboard detection (default: "chess_detection.onnx").
    #[arg(global = true, long, default_value = "chess_detection.onnx")]
    pub model_path: String,

    /// Enables castling for white (default: false).
    #[arg(global = true, long, default_value_t = false)]
    pub castle_w: bool,

    /// Enables castling for black (default: false).
    #[arg(global = true, long, default_value_t = false)]
    pub castle_b: bool,
}

/// Represents the point of view (POV) for chessboard detection.
#[derive(ValueEnum, Debug, Clone, PartialEq)]
pub enum Pov {
    /// White's point of view.
    W,
    /// Black's point of view.
    B,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Process an image file and print the detections and fen.
    Process {
        /// Path to the image file, or "-" to read from stdin.
        /// When reading from standard input, the first byte specifies the point of view (POV), the next 4 bytes indicate the image size, and the remaining bytes are the image data.
        image_path: String,

        /// Don't attempt to extract FEN notation from the best chessboard detection (default: false).
        #[arg(long, default_value_t = false)]
        no_fen: bool,

        /// Print the coordinates (x, y, width, height) of the detections.
        /// If --best-chessboard-detection-only is enabled, only the detections used for FEN extraction are printed.
        #[arg(long, default_value_t = false)]
        print_detections: bool,

        /// Print only the best chessboard detection and its associated pieces for FEN extraction.
        /// This flag is ignored if --no-fen is true (default: false).
        #[arg(long, default_value_t = false)]
        best_chessboard_detection_only: bool,

        /// If specified, the tool will annotate the original image with all detections and save it at the given path. You can also give it "-" to write to the output pipe.
        #[arg(long)]
        output_path: Option<String>,

        /// Do not exit after processing an image; continuously wait for additional images from the input pipe.
        /// This option is ignored if a file path is not "-" (default: false).
        #[arg(long, default_value_t = false)]
        dont_exit: bool,
    },

    /// Play a game of chess for you as a bot.
    Play {
        /// Defines the command to capture a screenshot and pipe image data to stdout for Chust to process.
        /// The image must be in a format supported by the "image" crate.
        /// Default: `xcap` for Windows, Linux (X11), macOS, and `wlr-screencopy-unstable-v1` for Wayland (via wayland-client).
        #[arg(long)]
        screenshot_command: Option<String>,

        /// Specifies a command to simulate a mouse click at given coordinates.
        /// Use `{x}` and `{y}` as placeholders, which Chust replaces with actual coordinates before execution.
        /// Default: `enigo` for Windows, Linux (X11), macOS, and `wlr-virtual-pointer-unstable-v1` for Wayland.
        ///
        /// Example: "some_tool click {x}, {y}"
        #[arg(long)]
        click_command: Option<String>,

        /// Sets the delay (in seconds) before capturing another screenshot. (default: 0.5 seconds).
        #[arg(long, default_value_t = 0.5)]
        screenshot_delay: f32,

        /// Path to the Stockfish binary.
        /// Default: "stockfish" (Linux/macOS) or "stockfish.exe" (Windows).
        #[arg(long, default_value = default_stockfish_path())]
        stockfish_path: String,

        /// Depth for Stockfish analysis. Higher values improve move quality but increase computation time.
        /// Default: 10.
        #[arg(long, default_value_t = 10)]
        stockfish_depth: u32,

        /// Ensures board stability before confirming a move by rechecking after detecting a change.
        /// When a new position is detected, we wait briefly and verify that the board state remains consistent
        /// before proceeding. This helps prevent false positives caused by animations, lag, or partial updates.
        ///
        /// If disabled, Chust may react more quickly but at the risk of misinterpreting temporary visual changes.
        ///
        /// Default: false.
        #[arg(long, default_value_t = false)]
        recheck_after_change: bool,

        /// Specifies the delay (in seconds) between selecting a piece and clicking its destination.
        /// This simulates a more human-like interaction with the board.
        /// Default: 0.1 seconds.
        #[arg(long, default_value_t = 0.1)]
        move_delay: f32,
    },
}

fn default_stockfish_path() -> &'static str {
    #[cfg(target_os = "windows")]
    {
        "stockfish.exe"
    }

    #[cfg(not(target_os = "windows"))]
    {
        "stockfish"
    }
}
