mod arg_parser;
mod chess_detection;
mod drawing;
mod input_capture;
mod play;
mod process;
mod stockfish;
use anyhow::{Context, Result};
use arg_parser::Args;
use chess_detection::ChessDetection;
use clap::Parser;
use ort::session::{builder::GraphOptimizationLevel, Session};
use play::play;
use process::process;
use stockfish::Stockfish;

fn main() -> Result<()> {
    let args = Args::parse();
    let mut chess_detector = initialize_chess_detector(&args)?;

    match args.command {
        arg_parser::Commands::Play {
            ref screenshot_command,
            ref click_command,
            screenshot_delay,
            ref stockfish_path,
            stockfish_depth,
            recheck_after_change,
            move_delay
        } => {
            let input_capture = input_capture::input_capture_manager::create_input_capture(
                0,
                click_command.clone(),
                screenshot_command.clone(),
            )?;
            let stockfish = Stockfish::new(&stockfish_path)?;

            play(
                screenshot_delay,
                stockfish_depth,
                stockfish,
                recheck_after_change,
                move_delay,
                &args,
                &mut chess_detector,
                input_capture,
            )?;
        }

        arg_parser::Commands::Process {
            ref image_path,
            no_fen,
            print_detections,
            best_chessboard_detection_only,
            ref output_path,
            dont_exit,
        } => {
            process(
                &image_path.to_string(),
                no_fen,
                print_detections,
                best_chessboard_detection_only,
                output_path.clone(),
                dont_exit,
                &args,
                &mut chess_detector,
            )?;
        }
    }

    Ok(())
}

// Helper: converts any ort::Error<R> into an anyhow::Error via its Display impl,
// sidestepping the fact that ort::Error<R> isn't Send + Sync (so `?` can't auto-convert it).
fn ort_err<R>(e: ort::Error<R>) -> anyhow::Error {
    anyhow::anyhow!(e.to_string())
}

fn initialize_chess_detector(args: &Args) -> Result<ChessDetection> {
    let model = if cfg!(feature = "embed_model") {
        Session::builder()
            .map_err(ort_err)?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(ort_err)?
            .with_intra_threads(2)
            .map_err(ort_err)?
            .commit_from_memory(include_bytes!("../chess_detection.onnx"))
            .map_err(ort_err)?
    } else {
        Session::builder()
            .map_err(ort_err)?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(ort_err)?
            .with_intra_threads(2)
            .map_err(ort_err)?
            .commit_from_file(&args.model_path)
            .map_err(ort_err)
            .context(format!(
                "Failed to load the model `{}`. Are you sure that the path is correct?",
                args.model_path
            ))?
    };
    Ok(ChessDetection::new(model, args.conf, args.refined_padding))
}
