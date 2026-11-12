mod arg_parser;
mod input_capture;
mod play;
mod process;
mod stockfish;

use anyhow::Result;
use arg_parser::{Args, Commands};
use chust_lib::ChessDetection;
use clap::Parser;
use play::play;
use process::process;

fn main() -> Result<()> {
    let args = Args::parse();
    let mut chess_detector = initialize_chess_detector(&args)?;

    match &args.command {
        Commands::Play(cmd) => play(cmd, &args, &mut chess_detector)?,
        Commands::Process(cmd) => process(cmd, &args, &mut chess_detector)?,
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
