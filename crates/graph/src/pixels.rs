//! Antialiased RGBA graph tiles; terminal protocols and Git objects stay outside this module.
use crate::{Edge, Row};
use std::{fmt, num::NonZeroU16};
use tiny_skia::{BlendMode, FillRule, Paint, PathBuilder, Pixmap, Stroke, Transform};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Metrics {
    pub width: NonZeroU16,
    pub height: NonZeroU16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Node {
    Commit,
    Merge,
    Boundary,
}

#[derive(Debug, Clone, Copy)]
pub struct Crop {
    pub columns: NonZeroU16,
    pub pan: usize,
}

pub struct Pixels {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[derive(Debug)]
pub struct TooLarge;
impl fmt::Display for TooLarge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("graph pixel tile exceeds the 4 MiB limit")
    }
}
impl std::error::Error for TooLarge {}

const PALETTE: &[[u8; 3]] = &[
    [129, 161, 193],
    [163, 190, 140],
    [180, 142, 173],
    [136, 192, 208],
    [235, 203, 139],
    [208, 135, 143],
];

fn paint(color: usize) -> Paint<'static> {
    let [r, g, b] = PALETTE[color % PALETTE.len()];
    let mut paint = Paint::default();
    paint.set_color_rgba8(r, g, b, 255);
    paint.anti_alias = true;
    paint
}

fn edge(pixels: &mut Pixmap, route: Edge, y: f32, metrics: Metrics, crop: Crop) {
    let w = f32::from(metrics.width.get());
    let h = f32::from(metrics.height.get());
    let x = |column: usize| (column as f32 * 2.0 + 0.5 - crop.pan as f32) * w;
    let mut path = PathBuilder::new();
    path.move_to(x(route.from), y);
    // 两端保持竖直切线，跨行的曲线共享同一条几何路径，避免字符格边界产生断口。
    path.cubic_to(
        x(route.from),
        y + h * 0.55,
        x(route.to),
        y + h * 0.45,
        x(route.to),
        y + h,
    );
    pixels.stroke_path(
        &path.finish().expect("one cubic path"),
        &paint(route.color),
        &Stroke {
            width: (w / 9.0).clamp(1.25, 3.0),
            ..Stroke::default()
        },
        Transform::identity(),
        None,
    );
}

/// Paint one text row, including the clipped edges from the preceding row.
pub fn row(
    previous: Option<&Row>,
    current: &Row,
    node: Node,
    metrics: Metrics,
    crop: Crop,
) -> Result<Pixels, TooLarge> {
    let width = u32::from(crop.columns.get()) * u32::from(metrics.width.get());
    let height = u32::from(metrics.height.get());
    if u64::from(width) * u64::from(height) * 4 > 4 * 1024 * 1024 {
        return Err(TooLarge);
    }
    let full_columns = current
        .width
        .max(previous.map_or(0, |row| row.width))
        .max(crop.pan + usize::from(crop.columns.get()));
    let full_width = u32::try_from(full_columns)
        .ok()
        .and_then(|columns| columns.checked_mul(u32::from(metrics.width.get())))
        .ok_or(TooLarge)?;
    if u64::from(full_width) * u64::from(height) * 4 > 4 * 1024 * 1024 {
        return Err(TooLarge);
    }
    let mut pixels = Pixmap::new(full_width, height).ok_or(TooLarge)?;
    let full_crop = Crop { pan: 0, ..crop };
    let w = f32::from(metrics.width.get());
    let h = height as f32;
    // 先绘制连线，再覆盖节点；普通提交之间的竖线贯穿整个行高。
    for (routes, y) in [
        (
            previous.map(|row| row.outgoing.as_slice()).unwrap_or(&[]),
            -h / 2.0,
        ),
        (current.outgoing.as_slice(), h / 2.0),
    ] {
        for route in routes.iter().filter(|route| route.from == route.to) {
            edge(&mut pixels, *route, y, metrics, full_crop);
        }
        for route in routes.iter().filter(|route| route.from != route.to) {
            edge(&mut pixels, *route, y, metrics, full_crop);
        }
    }
    let x = (current.node.column as f32 * 2.0 + 0.5) * w;
    let radius = (w * 0.34).min(h * 0.22) * if node == Node::Merge { 1.15 } else { 1.0 };
    let circle = PathBuilder::from_circle(x, h / 2.0, radius).expect("positive circle radius");
    if node == Node::Boundary {
        let mut clear = Paint {
            blend_mode: BlendMode::Clear,
            ..Paint::default()
        };
        clear.anti_alias = true;
        pixels.fill_path(
            &circle,
            &clear,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
        pixels.stroke_path(
            &circle,
            &paint(current.node.color),
            &Stroke {
                width: (w / 8.0).clamp(1.5, 3.0),
                ..Stroke::default()
            },
            Transform::identity(),
            None,
        );
    } else {
        pixels.fill_path(
            &circle,
            &paint(current.node.color),
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
    // 先在相同坐标系绘制，再裁剪像素，防止平移时路径裁剪改变边缘覆盖率。
    let skip = crop.pan * usize::from(metrics.width.get());
    let rgba = pixels
        .pixels()
        .chunks(full_width as usize)
        .flat_map(|row| row[skip..skip + width as usize].iter())
        .flat_map(|pixel| {
            let color = pixel.demultiply();
            [color.red(), color.green(), color.blue(), color.alpha()]
        })
        .collect();
    Ok(Pixels {
        width,
        height,
        rgba,
    })
}

#[cfg(test)]
mod tests;
