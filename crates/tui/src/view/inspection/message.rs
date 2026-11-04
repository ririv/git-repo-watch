//! Message text leads; identities and parent comparison remain available in the same panel.
use super::*;
use crate::inspection::Source;
use crate::view::metadata::{field, time};
use engine::local::history::{Base, Message};

fn metadata(commit: &Session) -> Vec<Line<'static>> {
    match &commit.detail {
        Source::Commit(detail) => commit_metadata(detail),
        Source::Stash(detail) => vec![
            field(i18n::tr!(HistoryTime), time(detail.entry.time)),
            field(i18n::tr!(HistoryHash), detail.entry.oid.to_string()),
            field(i18n::tr!(StashPreviewScope), detail.scope.label()),
            field(
                i18n::tr!(StashBase),
                detail.base.to_string()[..12].to_owned(),
            ),
        ],
    }
}
fn commit_metadata(detail: &engine::local::history::Detail) -> Vec<Line<'static>> {
    let comparison = match detail.base {
        Base::Empty => format!(
            "{} → {}",
            i18n::tr!(HistoryRoot),
            &detail.oid.to_string()[..12]
        ),
        Base::Commit(oid) => {
            let index = detail
                .parents
                .iter()
                .position(|parent| *parent == oid)
                .expect("comparison uses actual parent");
            format!(
                "{} ({}/{}) → {}",
                &oid.to_string()[..12],
                index + 1,
                detail.parents.len(),
                &detail.oid.to_string()[..12]
            )
        }
    };
    let different_times = detail.author.seconds != detail.committer.seconds;
    let mut lines = vec![
        field(
            i18n::tr!(HistoryAuthor),
            format!("{} <{}>", detail.author.name, detail.author.email),
        ),
        field(
            if different_times {
                i18n::tr!(HistoryAuthorTime)
            } else {
                i18n::tr!(HistoryTime)
            },
            time(detail.author.seconds),
        ),
    ];
    if detail.author.name != detail.committer.name || detail.author.email != detail.committer.email
    {
        lines.push(field(
            i18n::tr!(HistoryCommitter),
            format!("{} <{}>", detail.committer.name, detail.committer.email),
        ));
    }
    if different_times {
        lines.push(field(
            i18n::tr!(HistoryCommitterTime),
            time(detail.committer.seconds),
        ));
    }
    lines.push(field(i18n::tr!(HistoryHash), detail.oid.to_string()));
    lines.push(field(i18n::tr!(HistoryCompare), comparison));
    lines
}
fn contents(commit: &Session, inner: Rect) -> Vec<(Rect, Text<'static>)> {
    let mut message: Vec<_> = multiline(commit.detail.message())
        .lines()
        .enumerate()
        .map(|(index, line)| {
            if index == 0 {
                Line::styled(line.to_owned(), Style::default().fg(ACCENT).bold())
            } else {
                Line::raw(line.to_owned())
            }
        })
        .collect();
    if matches!(&commit.detail, Source::Commit(detail) if matches!(detail.message, Message::Truncated(_)))
    {
        message.push(Line::styled(
            i18n::tr!(HistoryTruncated),
            Style::default().fg(WARN),
        ));
    }
    if inner.width >= 100 {
        let columns = Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(inner);
        vec![
            (columns[0], Text::from(message)),
            (columns[1], Text::from(metadata(commit))),
        ]
    } else {
        message.extend(metadata(commit));
        vec![(inner, Text::from(message))]
    }
}
pub(super) fn desired_height(commit: &Session, width: u16) -> u16 {
    let width = width.saturating_sub(2);
    if width == 0 {
        return 0;
    }
    let texts = contents(commit, Rect::new(0, 0, width, 1));
    let lines = texts
        .iter()
        .map(|(area, text)| {
            Paragraph::new(text.clone())
                .wrap(Wrap { trim: false })
                .line_count(area.width)
        })
        .max()
        .unwrap_or(0);
    lines.saturating_add(2).min(u16::MAX as usize) as u16
}
pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    commit: &mut Session,
    hits: &mut crate::pointer::Map,
) {
    let border = block(commit.detail.message_title());
    let inner = border.inner(area);
    frame.render_widget(border, area);
    if inner.is_empty() {
        return;
    }
    let texts = contents(commit, inner);
    let lines = texts
        .iter()
        .map(|(area, text)| {
            Paragraph::new(text.clone())
                .wrap(Wrap { trim: false })
                .line_count(area.width)
        })
        .max()
        .unwrap_or(0);
    commit.scroll = commit.scroll.min(lines.saturating_sub(inner.height.into()));
    for (area, text) in texts {
        let text = hits.links.text(text);
        hits.links.paragraph(frame, area, &text, commit.scroll);
    }
    crate::scrollbar::vertical(frame, area, inner, lines, commit.scroll);
    hits.vertical(
        area,
        inner,
        lines.saturating_sub(inner.height.into()),
        commit.scroll,
        crate::pointer::ScrollTarget::InspectionMessage,
    );
}
