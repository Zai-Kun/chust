#[cfg(target_arch = "wasm32")]
use chust_lib::{get_best_chessboard_match, ChessDetection, DetectionLevel};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use ort::set_api;

#[cfg(target_arch = "wasm32")]
use ort_web::Dist;

#[cfg(target_arch = "wasm32")]
use web_sys::console::log_1;

#[cfg(target_arch = "wasm32")]
async fn init_onnx_runtime() -> Result<(), String> {
    let window = web_sys::window()
        .ok_or("Window is unavailable")?;

    let page_url = window
        .location()
        .href()
        .map_err(|e| format!("Failed to get page URL: {e:?}"))?;

    let runtime_url = web_sys::Url::new_with_base("./ort/", &page_url)
        .map_err(|e| format!("Failed to resolve runtime URL: {e:?}"))?
        .href();

    let dist = Dist::new(runtime_url)
        .with_script_name("ort.wasm.min.js");

    let api = ort_web::api(dist)
        .await
        .map_err(|e| format!("Failed to initialize ONNX runtime: {e}"))?;

    set_api(api);

    Ok(())
}


#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

#[cfg(target_arch = "wasm32")]
fn now_ms() -> f64 {
    web_sys::window()
        .expect("window not available")
        .performance()
        .expect("performance API not available")
        .now()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub async fn detect_chess_board(
    image_bytes: &[u8],
    model_bytes: &[u8],
) -> Result<String, JsError> {
    // 1. Decode image from in-memory bytes
    let mut start = now_ms();

    let img = image::load_from_memory(image_bytes)
        .map_err(|e| JsError::new(&format!("Failed to decode image: {e}")))?;

    log_1(
        &format!("Decoded image in {:.2} ms", now_ms() - start).into(),
    );

    // 2. Load ONNX model
    start = now_ms();

    init_onnx_runtime().await.map_err(|e| JsError::new(&format!("Failed to initialize ONNX runtime: {e}")))?;
    let mut detector = ChessDetection::from_memory(model_bytes, 0.7, 0.1)
        .await
        .map_err(|e| JsError::new(&format!("Failed to load ONNX model: {e}")))?;

    log_1(
        &format!("Loaded model in {:.2} ms", now_ms() - start).into(),
    );

    // 3. Run detection
    start = now_ms();

    let detections = detector
        .detect(&img, &DetectionLevel::Basic)
        .await
        .map_err(|e| JsError::new(&format!("Detection error: {e}")))?;

    log_1(
        &format!("Detected in {:.2} ms", now_ms() - start).into(),
    );

    // 4. Convert detections to FEN
    if let Some((best_match, _)) = get_best_chessboard_match(&detections) {
        let fen = detector.output_to_fen(
            &detections,
            (best_match[0] as u32, best_match[1] as u32),
            (best_match[2] as u32, best_match[3] as u32),
            true, // White POV
        );

        Ok(fen)
    } else {
        Ok("No chessboard found".to_string())
    }
}
