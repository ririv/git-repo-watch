//! Commit metadata and changed files are separate panes; merge parents switch explicitly.
use super::*;
use crate::history::Commit;
use engine::local::history::{Base, Message};

pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    commit: &mut Commit,
    hits: &mut crate::pointer::Map,
) {
    let parts = if area.width >= 110 {
        Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]).split(area)
    } else {
        Layout::vertical([Constraint::Percentage(55), Constraint::Percentage(45)]).split(area)
    };
    let detail = &commit.detail;
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
    let mut lines = vec![
        field(i18n::tr!(HistoryHash), detail.oid.to_string()),
        field(
            i18n::tr!(HistoryAuthor),
            format!("{} <{}>", detail.author.name, detail.author.email),
        ),
        field(i18n::tr!(HistoryTime), time(detail.author.seconds)),
        field(
            i18n::tr!(HistoryCommitter),
            format!("{} <{}>", detail.committer.name, detail.committer.email),
        ),
        field(i18n::tr!(HistoryTime), time(detail.committer.seconds)),
        field(i18n::tr!(HistoryCompare), comparison),
        Line::default(),
        Line::styled(
            i18n::tr!(HistoryMessage),
            Style::default().fg(ACCENT).bold(),
        ),
    ];
    lines.extend(
        multiline(detail.message.text())
            .lines()
            .map(|line| Line::raw(line.to_owned())),
    );
    if matches!(detail.message, Message::Truncated(_)) {
        lines.push(Line::styled(
            i18n::tr!(HistoryTruncated),
            Style::default().fg(WARN),
        ));
    }
    let border = block(format!(
        "{} · {}",
        name(&detail.path),
        &detail.oid.to_string()[..12]
    ));
    let inner = border.inner(parts[0]);
    let text = hits.links.text(Text::from(lines));
    let paragraph = Paragraph::new(text.plain()).wrap(Wrap { trim: false });
    let lines = paragraph.line_count(inner.width);
    commit.scroll = commit
        .scroll
        .min(lines.saturating_sub(inner.height as usize));
    frame.render_widget(border, parts[0]);
    hits.links.paragraph(frame, inner, &text, commit.scroll);
    crate::scrollbar::vertical(frame, parts[0], inner, lines, commit.scroll);
    hits.vertical(
        parts[0],
        inner,
        lines.saturating_sub(inner.height as usize),
        commit.scroll,
        crate::pointer::ScrollTarget::HistoryMessage,
    );
    let rows = commit.rows();
    let count = rows.len();
    let title = format!("{} · {}", i18n::tr!(HistoryChanges), rows.len());
    let border = block(title);
    let inner = border.inner(parts[1]);
    frame.render_widget(border, parts[1]);
    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new(if commit.query.is_empty() {
                i18n::tr!(HistoryNoChanges)
            } else {
                i18n::tr!(HistoryNoFilesMatch)
            })
            .wrap(Wrap { trim: false })
            .style(Style::default().fg(MUTED)),
            inner,
        );
    } else {
        let items: Vec<_> = rows
            .iter()
            .map(|change| {
                ListItem::new(format!(
                    "{} {}",
                    change.kind.symbol(),
                    safe(&change.location.display())
                ))
            })
            .collect();
        frame.render_stateful_widget(
            List::new(items).highlight_style(selected_style()),
            inner,
            &mut commit.files,
        );
        crate::scrollbar::vertical(frame, parts[1], inner, count, commit.files.offset());
        hits.list(
            parts[1],
            inner,
            &(0..count).collect::<Vec<_>>(),
            commit.files.offset(),
            crate::pointer::Target::HistoryFiles,
        );
    }
}
