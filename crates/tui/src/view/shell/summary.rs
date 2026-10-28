//! Colored repository counts with width-aware report metadata placement.
use super::*;

fn counts(stats: crate::repository_list::Counts) -> Line<'static> {
    let mut spans = vec![Span::raw(" ")];
    for (index, (label, count, color)) in [
        (i18n::tr!(Clean), stats.normal, GOOD),
        (i18n::tr!(NeedsAttention), stats.needs_attention, WARN),
        (i18n::tr!(Failed), stats.failed, BAD),
    ]
    .into_iter()
    .enumerate()
    {
        if index > 0 {
            spans.push(Span::styled(" · ", Style::default().fg(MUTED)));
        }
        spans.push(Span::styled(
            format!("{label} {count}"),
            Style::default().fg(color),
        ));
    }
    Line::from(spans)
}
fn arrange(summary: Option<Line<'static>>, checked: String, width: u16) -> Vec<Line<'static>> {
    let metadata = Span::styled(checked.trim_start().to_owned(), Style::default().fg(MUTED));
    let mut lines = Vec::new();
    if let Some(mut summary) = summary {
        let padding = usize::from(width).saturating_sub(summary.width() + metadata.width() + 1);
        // 至少保留三格间隔和一格右边距；放不下时整段换行，避免截断时间。
        if padding >= 3 {
            summary.spans.push(Span::raw(" ".repeat(padding)));
            summary.spans.push(metadata);
            return vec![summary];
        }
        lines.push(summary);
    }
    lines.push(Line::from(vec![Span::raw(" "), metadata]));
    lines
}

pub(super) fn lines(screen: &Screen, width: u16) -> Vec<Line<'static>> {
    let stats = crate::repository_list::Status::counts(
        screen
            .config
            .repositories
            .iter()
            .map(|path| screen.repository_status(path)),
    );
    let summary = (stats.normal + stats.needs_attention + stats.failed > 0).then(|| counts(stats));
    let checked = match &screen.snapshot.report {
        CachedReport::Missing => i18n::tr!(NoCompleteReportYet).to_owned(),
        CachedReport::Available { report, scope } => i18n::tf!(
            LastCheck,
            local_time(&report.finished_at),
            if matches!(scope, Scope::Outdated) {
                i18n::tr!(ReportScopeChanged)
            } else {
                ""
            }
        ),
    };
    arrange(summary, checked, width)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn localized_metadata_moves_as_a_whole_at_the_available_width_boundary() {
        for language in [i18n::Language::English, i18n::Language::Chinese] {
            let _language = i18n::using(language);
            for changed in ["", i18n::tr!(ReportScopeChanged)] {
                let summary = counts(crate::repository_list::Counts {
                    normal: 9,
                    needs_attention: 17,
                    failed: 2,
                });
                assert_eq!(
                    summary.to_string(),
                    match language {
                        i18n::Language::English => " Clean 9 · Needs attention 17 · Failed 2",
                        i18n::Language::Chinese => " 正常 9 · 需关注 17 · 失败 2",
                    }
                );
                let checked = i18n::tf!(LastCheck, "2026-10-24 04:00:00", changed);
                let needed = (summary.width()
                    + checked
                        .trim_start()
                        .chars()
                        .map(|c| unicode_width::UnicodeWidthChar::width(c).unwrap_or(0))
                        .sum::<usize>()
                    + 4) as u16;
                let wrapped = arrange(Some(summary.clone()), checked.clone(), needed - 1);
                assert_eq!(wrapped.len(), 2);
                assert_eq!(wrapped[0], summary);
                assert_eq!(wrapped[1].to_string(), checked);
                let inline = arrange(Some(summary), checked.clone(), needed);
                assert_eq!(inline.len(), 1);
                assert_eq!(inline[0].width(), usize::from(needed - 1));
                assert!(inline[0].to_string().ends_with(checked.trim_start()));
                for color in [GOOD, WARN, BAD] {
                    assert!(
                        inline[0]
                            .spans
                            .iter()
                            .any(|span| span.style.fg == Some(color))
                    );
                }
            }
            let missing = i18n::tr!(NoCompleteReportYet).to_owned();
            let lines = arrange(None, missing.clone(), 60);
            assert_eq!(lines.len(), 1);
            assert_eq!(lines[0].to_string(), missing);
        }
    }
}
