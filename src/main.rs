mod arg_parser;
mod chess_detection;
mod drawing;

use anyhow::{Context, Result};
use arg_parser::{Args, Pov};
use chess_detection::{get_best_chessboard_match, ChessDetection, DetectionLevel};
use clap::Parser;
use drawing::draw_detections;
use ort::session::{builder::GraphOptimizationLevel, Session};

fn main() -> Result<()> {
    let args = Args::parse();

    if args.no_fen && args.output_path.is_none() && !args.print_detections {
        return Err(anyhow::anyhow!("No output requested."));
    }

    // Load ONNX model with optimizations
    let model = Session::builder()?
        .with_optimization_level(GraphOptimizationLevel::Level3)?
        .with_intra_threads(2)?
        .commit_from_memory(include_bytes!("../chess_detection.onnx"))?;

    let chessboard_detection = ChessDetection::new(model, args.conf, args.refined_padding);

    // Load image
    let mut img = image::open(&args.image_path)
        .context(format!("Failed to load image {}", args.image_path))?;

    let detection_level = if args.refined_search {
        DetectionLevel::Refined
    } else {
        DetectionLevel::Basic
    };

    // Run detection
    let output = chessboard_detection
        .detect(&img, detection_level)
        .context("Detection failed")?
        .context("Failed to find the chessboard")?;

    let mut detection_filtter: Box<dyn Fn(&[f32]) -> bool> =
        Box::new(move |row: &[f32]| row[4] >= args.conf);
    // Process detection results
    if !args.no_fen {
        let best_detection = get_best_chessboard_match(&output)
            .context("No chessboard found")?
            .0;

        let fen = chessboard_detection.output_to_fen(
            &output,
            (best_detection[0] as u32, best_detection[1] as u32),
            (best_detection[2] as u32, best_detection[3] as u32),
            args.pov == Pov::W,
        );
        println!("FEN: {}", fen);
        if args.best_chessboard_detection_only {
            let (bx, by, bw, bh) = (
                best_detection[0] as u32,
                best_detection[1] as u32,
                best_detection[2] as u32,
                best_detection[3] as u32,
            );

            let cell_size: u32 = ((bw + bh) / 2) / 8;
            let half_cell_size = cell_size as f32 / 2.0;

            detection_filtter = Box::new(move |row: &[f32]| {
                if row[4] < args.conf {
                    return false;
                }
                let (x_center, y_center) = ((row[0]) + half_cell_size, (row[1]) + half_cell_size);
                if x_center < bx as f32
                    || x_center > (bx + bw) as f32
                    || y_center < by as f32
                    || y_center > (by + bh) as f32
                {
                    return false;
                }

                true
            });
        }
    }

    // Print detections if requested
    if args.print_detections {
        output.axis_iter(ndarray::Axis(0)).for_each(|row| {
            if detection_filtter(row.to_slice().unwrap()) {
                println!(
                    "{}: {}, {}, {}, {}, {}",
                    row[5], row[0], row[1], row[2], row[3], row[4]
                );
            }
        });
    }

    // Save output image if requested
    if let Some(output_path) = args.output_path {
        draw_detections(&mut img, &output, &detection_filtter);
        img.save(&output_path)
            .context(format!("Failed to save image to {}", output_path))?;
    }

    Ok(())
}
