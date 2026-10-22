//! The existing file-review navigation draws text or raster versions explicitly.
use super::*;
use crate::{
    document::View,
    graphics::{Display, Draw, Key, Source},
    preview::Side,
};
pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    view: &mut View,
    scroll: &mut usize,
    preference: crate::diff::Preference,
    graphics: &mut Display,
) {
    match view {
        View::Text(text) => diff::render(frame, area, text, scroll, preference),
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
    let text = match side {
        Side::Absent => Some(i18n::tr!(ImageAbsent).into()),
        Side::Other => Some(i18n::tr!(ImageNotRaster).into()),
        Side::Failed(error) => Some(multiline(error)),
        Side::Image {
            path,
            format,
            pixels,
        } => {
            let parts = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(inner);
            frame.render_widget(
                Paragraph::new(format!(
                    "{} · {}×{} · {}",
                    format.label(),
                    pixels.width(),
                    pixels.height(),
                    safe(&path.to_string_lossy())
                ))
                .style(Style::default().fg(MUTED)),
                parts[0],
            );
            if parts[1].width == 0 || parts[1].height == 0 {
                return;
            }
            match graphics.metrics() {
                Err(error) => {
                    frame.render_widget(Paragraph::new(error).wrap(Wrap { trim: false }), parts[1]);
                    None
                }
                Ok(metrics) => {
                    let key = Key {
                        source: Source::Preview(source),
                        index,
                        pan: 0,
                        columns: parts[1].width,
                        rows: parts[1].height,
                        metrics,
                    };
                    graphics.pin([key]);
                    let pixels = pixels.clone();
                    graphics.request(key, move || {
                        crate::preview::fit(
                            &pixels,
                            u32::from(key.columns) * u32::from(metrics.width.get()),
                            u32::from(key.rows) * u32::from(metrics.height.get()),
                        )
                    });
                    match graphics.render(frame, key, parts[1]) {
                        Draw::Ready => None,
                        Draw::Loading => {
                            frame.render_widget(
                                Paragraph::new(format!(
                                    "{} {}",
                                    graphics.symbol(),
                                    i18n::tr!(ImagePreparing)
                                )),
                                parts[1],
                            );
                            None
                        }
                        Draw::Failed(error) => {
                            frame.render_widget(
                                Paragraph::new(error).wrap(Wrap { trim: false }),
                                parts[1],
                            );
                            None
                        }
                    }
                }
            }
        }
    };
    if let Some(text) = text {
        frame.render_widget(Paragraph::new(text).wrap(Wrap { trim: false }), inner);
    }
}
