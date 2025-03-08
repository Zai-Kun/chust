use crate::arg_parser::{Args, Pov};
use crate::chess_detection::{get_best_chessboard_match, ChessDetection, DetectionLevel};
use crate::drawing::annotate_detections;
use anyhow::{Context, Result};
use imageproc::image::{self, DynamicImage};
use ndarray::{ArrayBase, IxDyn, OwnedRepr};
use std::io::{self, Cursor, Read, Write};

pub fn process(
    image_path: &str,
    no_fen: bool,
    print_detections: bool,
    best_chessboard_detection_only: bool,
    output_path: Option<String>,
    dont_exit: bool,

    args: &Args,
    chess_detector: &ChessDetection,
) -> Result<()> {
    if no_fen && output_path.is_none() && !print_detections {
        return Err(anyhow::anyhow!("No output requested."));
    }

    let stdin = std::io::stdin();
    let mut handle = stdin.lock();

    let detection_level = if args.refined_search {
        DetectionLevel::Refined
    } else {
        DetectionLevel::Basic
    };

    loop {
        let (is_white_pov, mut image) = if image_path != "-" {
            (
                args.pov == Pov::W,
                image::open(image_path).context(format!("Failed to load image {}", image_path))?,
            )
        } else {
            read_image_from_stdin(&mut handle)?
        };

        let detections = chess_detector
            .detect(&image, &detection_level)
            .context("Detection failed")?
            .context("Failed to find the chessboard")?;

        let detection_filter = process_detections_and_generate_filter(
            &chess_detector,
            &args,
            &detections,
            is_white_pov,
            no_fen,
            print_detections,
            best_chessboard_detection_only,
        )?;

        if let Some(output_path) = output_path.as_ref() {
            annotate_detections(&mut image, &detections, &detection_filter);
            save_image(&image, output_path)?;
        }

        if image_path != "-" || !dont_exit {
            break;
        }
    }

    Ok(())
}

fn process_detections_and_generate_filter(
    chess_detector: &ChessDetection,
    args: &Args,
    detections: &ArrayBase<OwnedRepr<f32>, IxDyn>,
    is_white_pov: bool,
    no_fen: bool,
    print_detections: bool,
    best_chessboard_detection_only: bool,
) -> Result<Box<dyn Fn(&[f32]) -> bool>> {
    let confidence_threshold = args.conf;
    let mut detection_filter: Box<dyn Fn(&[f32]) -> bool> =
        Box::new(move |row: &[f32]| row[4] >= confidence_threshold);

    if !no_fen {
        let best_match = get_best_chessboard_match(detections)
            .context("No chessboard found")?
            .0;

        let fen = chess_detector.output_to_fen(
            detections,
            (best_match[0] as u32, best_match[1] as u32),
            (best_match[2] as u32, best_match[3] as u32),
            is_white_pov,
        );
        println!("FEN: {}\n", fen);

        if best_chessboard_detection_only {
            let (x, y, width, height) = (
                best_match[0] as u32,
                best_match[1] as u32,
                best_match[2] as u32,
                best_match[3] as u32,
            );

            let cell_size: u32 = ((width + height) / 2) / 8;
            let half_cell_size = cell_size as f32 / 2.0;

            detection_filter = Box::new(move |row: &[f32]| {
                if row[4] < confidence_threshold {
                    return false;
                }
                let (x_center, y_center) = ((row[0]) + half_cell_size, (row[1]) + half_cell_size);
                if x_center < x as f32
                    || x_center > (x + width) as f32
                    || y_center < y as f32
                    || y_center > (y + height) as f32
                {
                    return false;
                }
                true
            });
        }
    }

    if print_detections {
        detections.axis_iter(ndarray::Axis(0)).for_each(|row| {
            if detection_filter(row.to_slice().unwrap()) {
                println!(
                    "{}: {}, {}, {}, {}, {}",
                    row[5], row[0], row[1], row[2], row[3], row[4]
                );
            }
        });
        println!("");
    }

    Ok(detection_filter)
}

fn save_image(img: &image::DynamicImage, output_path: &str) -> anyhow::Result<()> {
    if output_path == "-" {
        let mut buffer = Cursor::new(Vec::new());
        img.write_to(&mut buffer, image::ImageFormat::Png)
            .context("Failed to write image to stdout")?;

        io::stdout()
            .lock()
            .write_all(&buffer.into_inner())
            .context("Failed to write image to stdout")?;
    } else {
        img.save(output_path)
            .context(format!("Failed to save image to {}", output_path))?;
    }
    Ok(())
}

fn read_image_from_stdin(handle: &mut impl Read) -> Result<(bool, DynamicImage)> {
    let mut pov_buffer = [0u8; 1];
    handle
        .read_exact(&mut pov_buffer)
        .context("Reading POV failed")?;
    let is_white_pov = pov_buffer[0] == 1;

    let mut image_size_buffer = [0u8; 4];
    handle
        .read_exact(&mut image_size_buffer)
        .context("Reading image size failed")?;
    let image_size = u32::from_ne_bytes(image_size_buffer);

    let mut image_data = vec![0u8; image_size as usize];
    handle
        .read_exact(&mut image_data)
        .context("Reading image data failed")?;
    let img = image::load_from_memory(&image_data).context("Failed to load image from memory")?;

    Ok((is_white_pov, img))
}
