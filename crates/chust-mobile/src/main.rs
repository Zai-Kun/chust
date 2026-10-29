use chust_lib::{get_best_chessboard_match, ChessDetection, DetectionLevel};
use std::env;
use std::time::Instant;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let image_path = args.get(1).map(|s| s.as_str()).unwrap_or("test.png");
    let model_path = args
        .get(2)
        .map(|s| s.as_str())
        .unwrap_or("chess_detection.onnx");

    println!("Loading image: {image_path}...");
    let img = imageproc::image::open(image_path)?;

    println!("Initializing model: {model_path}...");
    let init_start = Instant::now();
    let mut detector = ChessDetection::from_file(model_path, 0.7, 0.1)?;
    println!("Model initialized in {:?}", init_start.elapsed());

    // Warm-up / benchmark inference
    println!("Running detection...");
    let start = Instant::now();
    let detections = detector.detect(&img, &DetectionLevel::Basic)?;
    let elapsed = start.elapsed();

    println!("--- Results ---");
    println!("Inference time: {:?}", elapsed);
    println!("Detections shape: {:?}", detections.shape());

    if let Some((best_match, _)) = get_best_chessboard_match(&detections) {
        let fen = detector.output_to_fen(
            &detections,
            (best_match[0] as u32, best_match[1] as u32),
            (best_match[2] as u32, best_match[3] as u32),
            true, // white POV
        );
        println!("Detected FEN: {fen}");
    } else {
        println!("No board detected.");
    }

    // Fast exit to avoid Android Bionic FORTIFY abort during C++ static destructor cleanup
    #[cfg(unix)]
    unsafe {
        extern "C" {
            fn _exit(status: std::os::raw::c_int) -> !;
        }
        _exit(0);
    }

    #[allow(unreachable_code)]
    Ok(())
}
