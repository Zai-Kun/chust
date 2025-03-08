use imageproc::image::{DynamicImage, Rgba};
use ndarray::{ArrayBase, IxDyn, OwnedRepr};
use imageproc::drawing::{draw_hollow_rect_mut, draw_text_mut};
use imageproc::rect::Rect;
use ab_glyph::FontArc;

/// Draws a bounding box with the specified thickness around a detected object.
pub fn draw_bounding_box(img: &mut DynamicImage, bbox: (u32, u32, u32, u32), thickness: u32) {
    let color = Rgba([255, 0, 0, 255]);
    let (x, y, width, height) = bbox;

    for i in 0..thickness {
        let rect = Rect::at(x as i32 - i as i32, y as i32 - i as i32)
            .of_size(width + i * 2, height + i * 2);
        draw_hollow_rect_mut(img, rect, color);
    }
}

/// Draws a class label at the center of a bounding box.
pub fn draw_label(img: &mut DynamicImage, bbox: (u32, u32, u32, u32), class_id: u32) {
    let class_names = [
        "bP", "bR", "bN", "bB", "bQ", "bK", "wP", "wR", "wN", "wB", "wQ", "wK", "CB"
    ];
    let label = class_names.get(class_id as usize).unwrap_or(&"??");
    let (x, y, width, height) = bbox;

    let font_data = include_bytes!("../CaskaydiaCoveNerdFont-Bold.ttf");
    let font = FontArc::try_from_slice(font_data).expect("Failed to load font");

    let text_x = x + (width / 2) - (label.len() as u32 * 15 / 2);
    let text_y = y + (height / 2);

    draw_text_mut(img, Rgba([255, 0, 0, 255]), text_x as i32, text_y as i32, 30.0, &font, label);
}

/// Draws bounding boxes and labels for detected objects that pass the given filter function.
pub fn annotate_detections(
    img: &mut DynamicImage,
    detections: &ArrayBase<OwnedRepr<f32>, IxDyn>,
    filter: &dyn Fn(&[f32]) -> bool,
) {
    for row in detections.axis_iter(ndarray::Axis(0)) {
        let data = row.to_slice().expect("Failed to convert row to slice");
        if filter(data) {
            let (x, y, width, height) = (data[0] as u32, data[1] as u32, data[2] as u32, data[3] as u32);
            let class_id = data[5] as u32;
            draw_label(img, (x, y, width, height), class_id);
            draw_bounding_box(img, (x, y, width, height), 2);
        }
    }
}
