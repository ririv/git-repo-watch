//! Map commit rows to shared RGBA image tiles without changing list navigation.
use super::*;
use crate::graphics::{Display, Draw, Key};
use graph::pixels::{Crop, Metrics, Node};
use std::num::NonZeroU16;

pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    session: &Session,
    display: &mut Display,
    metrics: Metrics,
    columns: u16,
    status: Rect,
) {
    let Some(width) = NonZeroU16::new(columns) else {
        return;
    };
    let first = session.state.offset();
    let end = (first + usize::from(area.height)).min(session.records.len());
    let key = |index| Key {
        source: crate::graphics::Source::Graph(session.source),
        index,
        pan: session.pan,
        columns,
        rows: 1,
        metrics,
    };
    display.pin((first..end).map(key));
    // 优先准备屏幕内的行，再预取上下相邻的行，滚动时复用已上传的图块。
    for index in (first..end)
        .chain(first.saturating_sub(12)..first)
        .chain(end..(end + 12).min(session.records.len()))
    {
        let records = session.records.clone();
        let pan = session.pan;
        display.request(key(index), move || {
            let record = &records[index];
            let node = match &record.entry.ancestry {
                engine::local::history::Ancestry::Boundary(_) => Node::Boundary,
                _ if record.entry.parents().len() > 1 => Node::Merge,
                _ => Node::Commit,
            };
            let pixels = graph::pixels::row(
                index.checked_sub(1).map(|index| &records[index].route),
                &record.route,
                node,
                metrics,
                Crop {
                    columns: width,
                    pan,
                },
            )
            .map_err(|_| anyhow::anyhow!(i18n::tr!(GraphicsTooLarge)))?;
            Ok(
                image::RgbaImage::from_raw(pixels.width, pixels.height, pixels.rgba)
                    .expect("RGBA pixel dimensions match storage"),
            )
        });
    }
    let mut loading = false;
    let mut error = None;
    for index in first..end {
        match display.render(
            frame,
            key(index),
            Rect::new(area.x, area.y + (index - first) as u16, columns, 1),
        ) {
            Draw::Ready => {}
            Draw::Loading => loading = true,
            Draw::Failed(message) => error = Some(message),
        }
    }
    let text = if let Some(error) = error {
        Some(error)
    } else if loading {
        Some(format!(
            "{} {}",
            display.symbol(),
            i18n::tr!(GraphicsPreparing)
        ))
    } else {
        None
    };
    if let Some(text) = text {
        frame.render_widget(Clear, status);
        frame.render_widget(
            Paragraph::new(text).style(Style::default().fg(WARN)),
            Rect {
                height: 1,
                ..status
            },
        );
    }
}
