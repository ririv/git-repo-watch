//! Preserve aspect ratio with transparent margins and checkerboard-backed image content.
use anyhow::{Result, ensure};
use image::{
    Rgba, RgbaImage,
    imageops::{FilterType, overlay, resize},
};
pub(crate) fn fit(source: &RgbaImage, width: u32, height: u32) -> Result<RgbaImage> {
    ensure!(
        width > 0 && height > 0 && u128::from(width) * u128::from(height) * 4 <= 32 * 1024 * 1024,
        i18n::tr!(GraphicsTooLarge)
    );
    let scale = (f64::from(width) / f64::from(source.width()))
        .min(f64::from(height) / f64::from(source.height()));
    let target_width = (f64::from(source.width()) * scale).floor().max(1.0) as u32;
    let target_height = (f64::from(source.height()) * scale).floor().max(1.0) as u32;
    let image = resize(source, target_width, target_height, FilterType::Triangle);
    let left = (width - target_width) / 2;
    let top = (height - target_height) / 2;
    let mut canvas = RgbaImage::new(width, height);
    // 棋盘仅位于图像范围内；外围留白保持透明，直接显示终端背景。
    for y in 0..target_height {
        for x in 0..target_width {
            let color = if (x / 8 + y / 8) % 2 == 0 { 42 } else { 56 };
            canvas.put_pixel(left + x, top + y, Rgba([color, color, color, 255]));
        }
    }
    overlay(&mut canvas, &image, i64::from(left), i64::from(top));
    // 图像内的棋盘不透明，修正混合将 alpha 舍入为 254 的误差，保留外围透明留白。
    for pixel in canvas.pixels_mut() {
        if pixel.0[3] != 0 {
            pixel.0[3] = 255;
        }
    }
    Ok(canvas)
}
