use crate::error::{ChustError, Result};
use crate::fen::output_to_fen;
use crate::geometry::{crop_with_padding, process_image, scale_bbox};
use imageproc::image::DynamicImage;
use ndarray::{ArrayBase, ArrayD, Ix4, IxDyn, OwnedRepr};
use ort::inputs;
use ort::session::builder::GraphOptimizationLevel;
use ort::session::Session;
use ort::value::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionLevel {
    Basic,
    Refined,
}

pub struct ChessDetection {
    session: Session,
    pub confidence_threshold: f32,
    pub refined_padding: f32,
}

impl ChessDetection {
    pub fn new(session: Session, confidence_threshold: f32, refined_padding: f32) -> Self {
        Self {
            session,
            confidence_threshold,
            refined_padding,
        }
    }

    /// Load model from a file path on disk.
    pub fn from_file(
        path: &str,
        confidence_threshold: f32,
        refined_padding: f32,
    ) -> Result<Self> {
        let session = Session::builder()
            .map_err(|e| ChustError::ModelLoad(format!("Builder init failed: {}", e)))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| ChustError::ModelLoad(format!("Optimization level error: {}", e)))?
            .with_intra_threads(2)
            .map_err(|e| ChustError::ModelLoad(format!("Thread configuration error: {}", e)))?
            .commit_from_file(path)
            .map_err(|e| ChustError::ModelLoad(format!("Failed to load ONNX model from `{}`: {}", path, e)))?;

        Ok(Self::new(session, confidence_threshold, refined_padding))
    }

    /// Load model from in-memory byte slice.
    pub fn from_memory(
        bytes: &[u8],
        confidence_threshold: f32,
        refined_padding: f32,
    ) -> Result<Self> {
        let session = Session::builder()
            .map_err(|e| ChustError::ModelLoad(format!("Builder init failed: {}", e)))?
            .with_optimization_level(GraphOptimizationLevel::Level3)
            .map_err(|e| ChustError::ModelLoad(format!("Optimization level error: {}", e)))?
            .with_intra_threads(2)
            .map_err(|e| ChustError::ModelLoad(format!("Thread configuration error: {}", e)))?
            .commit_from_memory(bytes)
            .map_err(|e| ChustError::ModelLoad(format!("Failed to initialize ONNX session from memory bytes: {}", e)))?;

        Ok(Self::new(session, confidence_threshold, refined_padding))
    }

    /// Load model embedded into the binary at compile time (requires `embed_model` feature).
    #[cfg(feature = "embed_model")]
    pub fn from_embedded(
        confidence_threshold: f32,
        refined_padding: f32,
    ) -> Result<Self> {
        Self::from_memory(
            include_bytes!("../../../assets/models/chess_detectionfp16.onnx"),
            confidence_threshold,
            refined_padding,
        )
    }

    fn predict(
        &mut self,
        input: ArrayBase<OwnedRepr<f32>, Ix4>,
    ) -> Result<ArrayBase<OwnedRepr<f32>, IxDyn>> {
        let input_value = Value::from_array(input)
            .map_err(|e| ChustError::InferenceFailed(format!("Failed to create input tensor value: {}", e)))?;
        let outputs = &mut self
            .session
            .run(inputs!["images" => input_value])
            .map_err(|e| ChustError::InferenceFailed(format!("Model session run failed: {}", e)))?;

        let (shape, data) = outputs["output0"]
            .try_extract_tensor::<f32>()
            .map_err(|e| ChustError::TensorExtraction {
                name: "output0",
                reason: e.to_string(),
            })?;

        let squeezed_shape: Vec<usize> = shape
            .iter()
            .map(|&d| d as usize)
            .filter(|&d| d != 1)
            .collect();

        let output = ArrayD::from_shape_vec(squeezed_shape, data.to_vec())
            .map_err(|e| ChustError::TensorShapeMismatch(e.to_string()))?;

        Ok(output)
    }

    fn filter_and_process_detections(
        &self,
        detections: &mut ArrayBase<OwnedRepr<f32>, IxDyn>,
        x_offset: u32,
        y_offset: u32,
        scale: f32,
    ) {
        for mut row in detections.axis_iter_mut(ndarray::Axis(0)) {
            if row[4] < self.confidence_threshold {
                continue;
            }
            let (x, y, w, h) = scale_bbox(
                row[0],
                row[1],
                row[2],
                row[3],
                x_offset as f32,
                y_offset as f32,
                scale,
            );
            row[0] = x as f32;
            row[1] = y as f32;
            row[2] = w as f32;
            row[3] = h as f32;
        }
    }

    /// Detects chess pieces and boards on an image.
    ///
    /// In `DetectionLevel::Basic`, returns all filtered detections.
    /// In `DetectionLevel::Refined`, returns `Err(ChustError::BoardNotFound)` if no chessboard
    /// is detected to perform the refined secondary pass.
    pub fn detect(
        &mut self,
        img: &DynamicImage,
        detection_level: &DetectionLevel,
    ) -> Result<ArrayBase<OwnedRepr<f32>, IxDyn>> {
        let (input, x_offset, y_offset, scale) = process_image(img);


        let mut output = self.predict(input)?;

        if let DetectionLevel::Refined = detection_level {
            let best_detection = match get_best_chessboard_match(&output) {
                Some(detection) => detection.0,
                None => return Err(ChustError::BoardNotFound),
            };

            let (x, y, w, h) = scale_bbox(
                best_detection[0],
                best_detection[1],
                best_detection[2],
                best_detection[3],
                x_offset as f32,
                y_offset as f32,
                scale,
            );
            let (cropped_img, new_x, new_y) =
                crop_with_padding(img, x, y, w, h, self.refined_padding);

            output = self.detect(&cropped_img, &DetectionLevel::Basic)?;

            for mut row in output.axis_iter_mut(ndarray::Axis(0)) {
                row[0] += new_x as f32;
                row[1] += new_y as f32;
            }
        } else {
            self.filter_and_process_detections(&mut output, x_offset, y_offset, scale);
        }

        Ok(output)
    }

    pub fn output_to_fen(
        &self,
        output: &ArrayBase<OwnedRepr<f32>, IxDyn>,
        board_cords: (u32, u32),
        board_size: (u32, u32),
        white_pov: bool,
    ) -> String {
        output_to_fen(output, board_cords, board_size, self.confidence_threshold, white_pov)
    }
}

pub fn get_best_chessboard_match(
    model_output: &ArrayBase<OwnedRepr<f32>, IxDyn>,
) -> Option<(&[f32], f32)> {
    let mut best_detection: Option<(_, f32)> = None;

    for row in model_output.axis_iter(ndarray::Axis(0)) {
        let confidence = row[4];
        let class_id = row[5] as u32;

        if class_id == 12 {
            if best_detection.is_none() || confidence > best_detection.as_ref().unwrap().1 {
                best_detection = Some((row.to_slice().unwrap(), confidence));
            }
        }
    }

    best_detection
}
