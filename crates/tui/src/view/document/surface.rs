//! A shared image surface prepares rasters without owning panel borders.
use super::super::*;
use crate::graphics::{Display, Draw, Key, Source};

pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    source: u64,
    index: usize,
    graphics: &mut Display,
    metadata: String,
    prepare: impl FnOnce(u32, u32) -> anyhow::Result<image::RgbaImage> + Send + 'static,
) {
    let parts = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(area);
    frame.render_widget(
        Paragraph::new(metadata).style(Style::default().fg(MUTED)),
        parts[0],
    );
    if parts[1].is_empty() {
        return;
    }
    let metrics = match graphics.metrics() {
        Ok(metrics) => metrics,
        Err(error) => {
            frame.render_widget(Paragraph::new(error).wrap(Wrap { trim: false }), parts[1]);
            return;
        }
    };
    let key = Key {
        source: Source::Preview(source),
        index,
        pan: 0,
        columns: parts[1].width,
        rows: parts[1].height,
        metrics,
    };
    graphics.pin([key]);
    graphics.request(key, move || {
        prepare(
            u32::from(key.columns) * u32::from(metrics.width.get()),
            u32::from(key.rows) * u32::from(metrics.height.get()),
        )
    });
    match graphics.render(frame, key, parts[1]) {
        Draw::Ready => {}
        Draw::Loading => frame.render_widget(
            Paragraph::new(format!(
                "{} {}",
                graphics.symbol(),
                i18n::tr!(ImagePreparing)
            )),
            parts[1],
        ),
        Draw::Failed(error) => {
            frame.render_widget(Paragraph::new(error).wrap(Wrap { trim: false }), parts[1])
        }
    }
}
