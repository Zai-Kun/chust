use image::{DynamicImage, Rgba};
use ndarray::{ArrayBase, IxDyn, OwnedRepr};

use imageproc::drawing::{draw_hollow_rect_mut, draw_text_mut};
use imageproc::rect::Rect;

use ab_glyph::FontArc;

pub fn draw_box(img: &mut DynamicImage, detection: (u32, u32, u32, u32), thickness: u32) {
    let color = Rgba([255, 0, 0, 255]);

    let (x, y, w, h) = detection;

    // Draw multiple rectangles to create thickness
    for i in 0..thickness {
        let rect = Rect::at(x as i32 - i as i32, y as i32 - i as i32).of_size(w + i * 2, h + i * 2);
        draw_hollow_rect_mut(img, rect, color);
    }
}

pub fn draw_text(img: &mut DynamicImage, detection: (u32, u32, u32, u32), class: u32) {
    let class_names = [
        "bP", "bR", "bN", "bB", "bQ", "bK", "wP", "wR", "wN", "wB", "wQ", "wK", "CB",
    ];

    let text = class_names.get(class as usize).unwrap_or(&"??");

    let (x, y, w, h) = detection;

    let font_data = include_bytes!("/usr/share/fonts/TTF/CaskaydiaCoveNerdFont-Bold.ttf");
    let font = FontArc::try_from_slice(font_data).unwrap();

    let text_x = x + (w / 2) - (text.len() as u32 * 15 / 2);
    let text_y = y + (h / 2);

    draw_text_mut(
        img,
        Rgba([255, 0, 0, 255]),
        text_x as i32,
        text_y as i32,
        30.0,
        &font,
        text,
    );
}

pub fn draw_detections(
    img: &mut DynamicImage,
    detections: &ArrayBase<OwnedRepr<f32>, IxDyn>,
    filter: &Box<dyn Fn(&[f32]) -> bool>,
) {
    for row in detections.axis_iter(ndarray::Axis(0)) {
        if filter(row.to_slice().unwrap()) {
            let (x, y, w, h) = (row[0] as u32, row[1] as u32, row[2] as u32, row[3] as u32);
            draw_text(img, (x, y, w, h), row[5] as u32);
            draw_box(img, (x, y, w, h), 2);
        }
    }
}
