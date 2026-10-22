//! Preserve aspect ratio and expose transparency over a neutral checkerboard.
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
    let mut canvas = RgbaImage::from_fn(width, height, |x, y| {
        let color = if (x / 8 + y / 8) % 2 == 0 { 42 } else { 56 };
        Rgba([color, color, color, 255])
    });
    overlay(
        &mut canvas,
        &image,
        i64::from((width - target_width) / 2),
        i64::from((height - target_height) / 2),
    );
    // 棋盘背景不透明，修正浮点混合将 alpha 舍入为 254 的误差。
    for pixel in canvas.pixels_mut() {
        pixel.0[3] = 255;
    }
    Ok(canvas)
}
