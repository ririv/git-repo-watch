//! All visible file sections share one clipped, styled viewport and scrollbar.
use super::*;
use crate::{diff::VisualRow, history::review::Rows};
use engine::local::{files::Location, history::Kind};

pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    commit: &mut Commit,
    wrap: bool,
    hits: &mut crate::pointer::Map,
) {
    let border = block(i18n::tr!(FileDiff));
    let inner = border.inner(area);
    frame.render_widget(border, area);
    if inner.is_empty() {
        return;
    }
    let indices = commit.indices();
    let selected = commit.selected_index();
    commit
        .review
        .arrange(&indices, selected, inner, wrap, &hits.links.paths);
    if indices.is_empty() {
        frame.render_widget(
            Paragraph::new(if commit.query.is_empty() {
                i18n::tr!(HistoryNoChanges)
            } else {
                i18n::tr!(HistoryNoFilesMatch)
            })
            .style(Style::default().fg(MUTED)),
            inner,
        );
        return;
    }
    let review = &commit.review;
    let offset = review.layout.offset;
    let end = offset + inner.height as usize;
    let horizontal = review.horizontal.offset();
    let loading = review.symbol();
    for section in &review.layout.sections {
        if section.start + section.height() <= offset || section.start >= end {
            continue;
        }
        let change = &commit.detail.changes[section.file];
        if section.start >= offset {
            let y = inner.y + (section.start - offset) as u16;
            let path = match &change.location {
                Location::Path(path) => path.display().to_string(),
                Location::Rename { before, after } | Location::Copy { before, after } => {
                    format!("{} → {}", before.display(), after.display())
                }
            };
            let color = match change.kind {
                Kind::Added => GOOD,
                Kind::Deleted => BAD,
                Kind::Modified | Kind::Typechange => WARN,
                Kind::Renamed | Kind::Copied => ACCENT,
            };
            let line = crate::view::files::marked_line(
                &Line::styled(safe(&path), Style::default().fg(ACCENT).bold()),
                change.kind.symbol(),
                color,
                inner.width.into(),
            );
            let header = Rect::new(inner.x, y, inner.width, 1);
            frame.render_widget(
                line.style(if selected == Some(section.file) {
                    selected_style()
                } else {
                    Style::default()
                }),
                header,
            );
            let index = indices
                .iter()
                .position(|file| *file == section.file)
                .expect("displayed file");
            hits.item(header, crate::pointer::Target::HistoryFiles, index);
        }
        let first = offset.saturating_sub(section.start + 1);
        let last = section
            .rows
            .len()
            .min(end.saturating_sub(section.start + 1));
        for row in first..last {
            let y = inner.y + (section.start + 1 + row - offset) as u16;
            let area = Rect::new(inner.x, y, inner.width, 1);
            match &section.rows {
                Rows::Ready(cache) => {
                    if let Some(line) = cache.rows.get(row) {
                        match line {
                            VisualRow::Full { line, fixed, .. } => {
                                hits.links.line(frame, area, line, *fixed, horizontal)
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
        crate::pointer::ScrollTarget::HistoryReview,
    );
    review.horizontal.render_on_border(
        frame,
        area,
        inner.x + review.layout.gutter_width().min(inner.width.into()) as u16..inner.right(),
        hits,
        crate::pointer::ScrollTarget::HistoryReviewHorizontal,
    );
}
