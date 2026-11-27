//! All visible file sections share one clipped, styled viewport and scrollbar.
use super::*;
use crate::horizontal::Columns;
use crate::{
    diff::VisualRow,
    inspection::review::{Rows, TITLE_STATUS_WIDTH, TitleRows},
};

pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    commit: &mut Session,
    options: crate::diff::Options,
    hits: &mut crate::pointer::Map,
) {
    let border = block(i18n::tr!(FileDiff));
    let inner = border.inner(area);
    frame.render_widget(border, area);
    if inner.is_empty() {
        commit.review.horizontal.constrain(0);
        return;
    }
    let indices = commit.indices();
    let selected = commit.selected_index();
    commit
        .review
        .arrange(&indices, selected, inner, options, &hits.links.paths);
    if indices.is_empty() {
        commit.review.horizontal.constrain(0);
        frame.render_widget(
            Paragraph::new(commit.detail.empty_message(&commit.query))
                .style(Style::default().fg(MUTED)),
            inner,
        );
        return;
    }
    let review = &mut commit.review;
    let titles = Columns::new(inner, 0, TITLE_STATUS_WIDTH);
    let title_overflow = if options.wrap {
        0
    } else {
        let widest = indices
            .iter()
            .map(|&file| review.title(file).width)
            .max()
            .unwrap_or(0);
        titles.overflow(widest)
    };
    let body_overflow = review.layout.horizontal_maximum();
    review
        .horizontal
        .constrain(title_overflow.max(body_overflow));
    let body_offset = review.horizontal.offset().min(body_overflow);
    let offset = review.layout.offset;
    let end = offset + inner.height as usize;
    let loading = review.symbol();
    for section in &review.layout.sections {
        if section.start + section.height() <= offset || section.start >= end {
            continue;
        }
        let title_start = section.start.max(offset);
        let title_end = section.body_start().min(end);
        if title_start < title_end {
            let title = review.title(section.file);
            let change = commit.detail.change(section.file);
            let title_style = Style::default().fg(ACCENT).bold();
            let row_style = if selected == Some(section.file) {
                selected_style()
            } else {
                Style::default()
            };
            for row in title_start..title_end {
                let first_line = row == section.start;
                let (text, horizontal) = match &section.title {
                    TitleRows::Single => (
                        Line::styled(title.text.as_str(), title_style),
                        review.horizontal.offset().min(titles.overflow(title.width)),
                    ),
                    TitleRows::Wrapped(lines) => {
                        (lines[row - section.start].clone().style(title_style), 0)
                    }
                };
                let status = first_line.then(|| crate::view::files::history_status(change.kind));
                let line = titles.with_offset(horizontal).line(&text, status);
                let area = Rect::new(inner.x, inner.y + (row - offset) as u16, inner.width, 1);
                frame.render_widget(line.style(row_style), area);
            }
            let index = indices
                .binary_search(&section.file)
                .expect("displayed file");
            hits.item(
                Rect::new(
                    inner.x,
                    inner.y + (title_start - offset) as u16,
                    inner.width,
                    (title_end - title_start) as u16,
                ),
                crate::pointer::Target::InspectionFiles,
                index,
            );
        }
        let body_start = section.body_start();
        let first = offset.saturating_sub(body_start);
        let last = section.rows.len().min(end.saturating_sub(body_start));
        for row in first..last {
            let y = inner.y + (body_start + row - offset) as u16;
            let area = Rect::new(inner.x, y, inner.width, 1);
            match &section.rows {
                Rows::Ready(cache) => {
                    if let Some(line) = cache.rows.get(row) {
                        match line {
                            VisualRow::Full { line, fixed, .. } => {
                                hits.links.line(frame, area, line, *fixed, body_offset)
                            }
                            VisualRow::Columns { .. } => {
                                unreachable!("continuous review uses unified rows")
                            }
                        }
                    }
                }
                Rows::Failed(error) => frame.render_widget(
                    Paragraph::new(safe(error)).style(Style::default().fg(BAD)),
                    area,
                ),
                Rows::Pending => {
                    if let Some(symbol) = loading {
                        frame.render_widget(
                            Paragraph::new(format!("{symbol} {}", i18n::tr!(HistoryReadDiff)))
                                .style(Style::default().fg(MUTED)),
                            area,
                        );
                    }
                }
            }
        }
        if let Some(separator) = section.separator()
            && (offset..end).contains(&separator)
        {
            frame.render_widget(
                Line::styled("─".repeat(inner.width.into()), Style::default().fg(BORDER)),
                Rect::new(
                    inner.x,
                    inner.y + (separator - offset) as u16,
                    inner.width,
                    1,
                ),
            );
        }
    }
    crate::scrollbar::vertical(frame, area, inner, review.layout.height, offset);
    hits.vertical(
        area,
        inner,
        review.layout.maximum(),
        offset,
        crate::pointer::ScrollTarget::InspectionReview,
    );
    let track_start = if title_overflow > 0 {
        inner.x
    } else {
        inner.x + review.layout.gutter_width().min(inner.width.into()) as u16
    };
    review.horizontal.render_on_border(
        frame,
        area,
        track_start..inner.right(),
        hits,
        crate::horizontal::Pane::InspectionDiff,
    );
}
