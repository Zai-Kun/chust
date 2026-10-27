use imageproc::image::{imageops, imageops::FilterType, DynamicImage, GenericImageView, Rgb, RgbImage};
use ndarray::{Array, ArrayBase, Ix4, OwnedRepr};

/// Crops an image around a bounding box with an added percentage padding.
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

/// Rescales bounding box coordinates from letterboxed image space back to original image space.
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

/// Preprocesses a dynamic image for model input: letterboxes to target size and converts to normalized CHW tensor.
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

/// Resizes the image to fit within target_size x target_size maintaining aspect ratio,
/// padding with black borders.
pub fn letterbox_resize(img: &DynamicImage, target_size: u32) -> (RgbImage, u32, u32, f32) {
    let (orig_w, orig_h) = img.dimensions();

    // Ensure at least one side is target_size while maintaining aspect ratio
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
