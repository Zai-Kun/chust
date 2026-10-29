mod arg_parser;
mod input_capture;
mod play;
mod process;
mod stockfish;

use anyhow::Result;
use arg_parser::Args;
use chust_lib::ChessDetection;
use clap::Parser;
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
            move_delay,
        } => {
            let input_capture = input_capture::input_capture_manager::create_input_capture(
                0,
                click_command.clone(),
                screenshot_command.clone(),
            )?;
            let stockfish = Stockfish::new(stockfish_path)?;

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
                image_path,
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

fn initialize_chess_detector(args: &Args) -> Result<ChessDetection> {
    #[cfg(feature = "embed_model")]
    {
        Ok(ChessDetection::from_embedded(
            args.conf,
            args.refined_padding,
        )?)
    }

    #[cfg(not(feature = "embed_model"))]
    {
        Ok(ChessDetection::from_file(
            &args.model_path,
            args.conf,
            args.refined_padding,
        )?)
    }
}
