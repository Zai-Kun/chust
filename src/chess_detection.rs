use imageproc::image::{imageops, imageops::FilterType, DynamicImage, GenericImageView, Rgb, RgbImage};
use ndarray::{Array, ArrayBase, Axis, Ix4, IxDyn, OwnedRepr};
use ort::inputs;
use ort::session::Session;

pub static PIECE_MAP: [char; 12] = ['p', 'r', 'n', 'b', 'q', 'k', 'P', 'R', 'N', 'B', 'Q', 'K'];

pub enum DetectionLevel {
    Basic,   // Level 1: Detect the board and pieces directly
    Refined, // Level 2: Crop & reprocess for better small-board detection
}

pub struct ChessDetection {
    session: Session,
    confidence_threshold: f32,
    refined_padding: f32,
}

impl ChessDetection {
    pub fn new(session: Session, confidence_threshold: f32, refined_padding: f32) -> Self {
        Self {
            session,
            confidence_threshold,
            refined_padding,
        }
    }

    fn predict(
        &self,
        input: ArrayBase<OwnedRepr<f32>, Ix4>,
    ) -> ort::Result<ArrayBase<OwnedRepr<f32>, IxDyn>> {
        let outputs = self.session.run(inputs!["images" => input]?)?;
        let output = outputs["output0"]
            .try_extract_tensor::<f32>()?
            .squeeze()
            .into_owned();

        Ok(output)
    }

    fn filter_and_proccess_detections(
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

    pub fn detect(
        &self,
        img: &DynamicImage,
        detection_level: &DetectionLevel,
    ) -> ort::Result<Option<ArrayBase<OwnedRepr<f32>, IxDyn>>> {
        let (input, x_offset, y_offset, scale) = process_image(img);
        let mut output = self.predict(input)?;

        if let DetectionLevel::Refined = detection_level {
            let best_detection = match get_best_chessboard_match(&output) {
                Some(detection) => detection.0,
                None => return Ok(None), // Return None if no chessboard is found
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
            output = match self.detect(&cropped_img, &DetectionLevel::Basic)? {
                Some(out) => out,
                None => return Ok(None),
            };

            for mut row in output.axis_iter_mut(ndarray::Axis(0)) {
                row[0] += new_x as f32;
                row[1] += new_y as f32;
            }
        } else {
            self.filter_and_proccess_detections(&mut output, x_offset, y_offset, scale);
        }

        Ok(Some(output))
    }
    pub fn output_to_fen(
        &self,
        output: &ArrayBase<OwnedRepr<f32>, IxDyn>,
        board_cords: (u32, u32),
        board_size: (u32, u32),
        white_pov: bool,
    ) -> String {
        let filtered_output = output
            .axis_iter(Axis(0))
            .filter(|row| row[4] >= self.confidence_threshold && row[5] != 12.0);
        let cell_size: u32 = ((board_size.0 + board_size.1) / 2) / 8;
        let half_cell_size = cell_size as f32 / 2.0;

        let mut board = [[' '; 8]; 8];

        for detection in filtered_output {
            let (x, y) = (
                (detection[0]) + half_cell_size,
                (detection[1]) + half_cell_size,
            );

            let x_location = ((x - board_cords.0 as f32) / cell_size as f32).ceil() as usize;
            let y_location = ((y - board_cords.1 as f32) / cell_size as f32).ceil() as usize;

            if x_location > 8 || x_location < 1 || y_location > 8 || y_location < 1 {
                continue;
            }

            if let Some(&piece) = PIECE_MAP.get(detection[5] as usize) {
                if white_pov {
                    board[y_location - 1][x_location - 1] = piece;
                } else {
                    board[8 - y_location][8 - x_location] = piece;
                }
            }
        }

        let mut fen = String::with_capacity(64 + 7); // 64 for the board, 7 for the slashes
        for row in board {
            let mut empty_count: u8 = 0;
            for &cell in row.iter() {
                if cell == ' ' {
                    empty_count += 1;
                } else {
                    if empty_count > 0 {
                        fen.push((b'0' + empty_count) as char);
                        empty_count = 0;
                    }
                    fen.push(cell);
                }
            }
            if empty_count > 0 {
                fen.push((b'0' + empty_count) as char);
            }
            fen.push('/');
        }
        fen.pop(); // Remove the last '/'
        fen
    }
}

pub fn crop_with_padding(
    img: &DynamicImage,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    padding: f32,
) -> (DynamicImage, u32, u32) {
    let pad_w = (w as f32 * padding) as u32;
    let pad_h = (h as f32 * padding) as u32;

    let new_x = x.saturating_sub(pad_w);
    let new_y = y.saturating_sub(pad_h);
    let new_w = (w + 2 * pad_w).min(img.width() - new_x);
    let new_h = (h + 2 * pad_h).min(img.height() - new_y);

    (img.crop_imm(new_x, new_y, new_w, new_h), new_x, new_y)
}

pub fn scale_bbox(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    x_offset: f32,
    y_offset: f32,
    scale: f32,
) -> (u32, u32, u32, u32) {
    (
        ((x - x_offset) / scale) as u32,
        ((y - y_offset) / scale) as u32,
        ((w - x) / scale) as u32,
        ((h - y) / scale) as u32,
    )
}

pub fn process_image(img: &DynamicImage) -> (ArrayBase<OwnedRepr<f32>, Ix4>, u32, u32, f32) {
    let (padded_img, x_offset, y_offset, scale) = letterbox_resize(img, 640);
    let mut input = Array::zeros((1, 3, 640, 640));
    for (x, y, pixel) in padded_img.enumerate_pixels() {
        let Rgb([r, g, b]) = *pixel;
        input[[0, 0, y as usize, x as usize]] = (r as f32) / 255.;
        input[[0, 1, y as usize, x as usize]] = (g as f32) / 255.;
        input[[0, 2, y as usize, x as usize]] = (b as f32) / 255.;
    }
    (input, x_offset, y_offset, scale)
}

pub fn letterbox_resize(img: &DynamicImage, target_size: u32) -> (RgbImage, u32, u32, f32) {
    let (orig_w, orig_h) = img.dimensions();

    // Ensure at least one side is 640 while maintaining aspect ratio
    let scale = if orig_w > orig_h {
        target_size as f32 / orig_w as f32
    } else {
        target_size as f32 / orig_h as f32
    };

    let new_w = (orig_w as f32 * scale) as u32;
    let new_h = (orig_h as f32 * scale) as u32;

    let resized = img
        .resize_exact(new_w, new_h, FilterType::Lanczos3)
        .to_rgb8();

    // Calculate padding offsets to center the image
    let x_offset = (target_size - new_w) / 2;
    let y_offset = (target_size - new_h) / 2;

    // Create a new black-padded square image
    let mut padded = RgbImage::new(target_size, target_size);
    imageops::overlay(&mut padded, &resized, x_offset.into(), y_offset.into());
    (padded, x_offset, y_offset, scale)
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
