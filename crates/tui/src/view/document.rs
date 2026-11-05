//! The existing file-review navigation draws text or raster versions explicitly.
mod surface;
use super::*;
use crate::{document::View, graphics::Display, preview::Side};

pub(super) enum Metadata {
    Properties,
    WithPath,
}
impl Metadata {
    fn label(&self, properties: String, path: &std::path::Path) -> String {
        match self {
            Self::Properties => properties,
            Self::WithPath => format!("{properties} · {}", safe(&path.to_string_lossy())),
        }
    }
}
pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    view: &mut View,
    scroll: &mut usize,
    preference: crate::diff::Options,
    graphics: &mut Display,
    hits: &mut crate::pointer::Map,
) {
    match view {
        View::Conflict(view) => {
            super::conflict::render(frame, area, view, scroll, preference, graphics, hits)
        }
        View::Text(text) => diff::render(frame, area, text, scroll, preference, hits),
        View::Images(images) => {
            let border = block(i18n::tr!(ImagePreview));
            let inner = border.inner(area);
            frame.render_widget(border, area);
            let rows = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(inner);
            frame.render_widget(
                Paragraph::new(safe(&images.title)).style(Style::default().fg(ACCENT)),
                rows[0],
            );
            let pairs = Layout::vertical(vec![
                Constraint::Ratio(1, images.pairs.len() as u32);
                images.pairs.len()
            ])
            .split(rows[1]);
            for (index, pair) in images.pairs.iter().enumerate() {
                let columns = // 高度不足以叠放两个版本时仍左右排列，给像素内容保留有效区域。
                if area.width >= 100 || pairs[index].height < 10 {
                    Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .split(pairs[index])
                } else {
                    Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .split(pairs[index])
                };
                side(
                    frame,
                    columns[0],
                    &pair.before,
                    &images.labels.0,
                    images.source,
                    index * 2,
                    graphics,
                );
                side(
                    frame,
                    columns[1],
                    &pair.after,
                    &images.labels.1,
                    images.source,
                    index * 2 + 1,
                    graphics,
                );
            }
        }
    }
}
fn side(
    frame: &mut Frame,
    area: Rect,
    side: &Side,
    label: &str,
    source: u64,
    index: usize,
    graphics: &mut Display,
) {
    let border = block(label);
    let inner = border.inner(area);
    frame.render_widget(border, area);
    content(
        frame,
        inner,
        side,
        source,
        index,
        graphics,
        Metadata::WithPath,
    );
}

/// Draw image content inside a panel already owned by the caller.
pub(super) fn content(
    frame: &mut Frame,
    area: Rect,
    side: &Side,
    source: u64,
    index: usize,
    graphics: &mut Display,
    metadata: Metadata,
) {
    let text = match side {
        Side::Absent => Some(i18n::tr!(ImageAbsent).into()),
        Side::Other => Some(i18n::tr!(ImageNotRaster).into()),
        Side::Failed(error) => Some(multiline(error)),
        Side::Image {
            path,
            format,
            pixels,
        } => {
            let pixels = pixels.clone();
            surface::render(
                frame,
                area,
                source,
                index,
                graphics,
                metadata.label(
                    format!(
                        "{} · {}×{}",
                        format.label(),
                        pixels.width(),
                        pixels.height()
                    ),
                    path,
                ),
                move |width, height| crate::preview::fit(&pixels, width, height),
            );
            None
        }
        Side::Vector { path, tree } => {
            let tree = tree.clone();
            let size = tree.size();
            surface::render(
                frame,
                area,
                source,
                index,
                graphics,
                metadata.label(format!("SVG · {}×{}", size.width(), size.height()), path),
                move |width, height| crate::preview::svg::raster(&tree, width, height),
            );
            None
        }
    };
    if let Some(text) = text {
        frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: false }), area);
    }
}
