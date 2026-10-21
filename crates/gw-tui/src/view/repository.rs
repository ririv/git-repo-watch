use super::*;
pub(super) fn repositories(frame: &mut Frame, screen: &mut Screen, area: Rect) {
    let parts = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(3),
    ])
    .split(area);
    let mut filters = Vec::new();
    for filter in [
        Filter::All,
        Filter::Problems,
        Filter::WithoutUpstream,
        Filter::Pushes,
    ]
    .iter()
    {
        let count = if *filter == Filter::All {
            format!(" {}", screen.config.repositories.len())
        } else if *filter == screen.filter {
            gw_i18n::tf!(Repos, screen.rows().len())
        } else {
            String::new()
        };
        filters.push(Span::styled(
            format!(" {}{} ", filter.name(), count),
            if *filter == screen.filter {
                selected_style()
            } else {
                Style::default().fg(MUTED)
            },
        ));
    }
    if !screen.query.is_empty() {
        filters.push(Span::raw(gw_i18n::tf!(Search, safe(&screen.query))))
    }
    frame.render_widget(Paragraph::new(Line::from(filters)), parts[0]);
    frame.render_widget(
        Paragraph::new(gw_i18n::tr!(WorktreeChangesUnknownPushSuccessFailureCounts))
            .style(Style::default().fg(MUTED)),
        parts[1],
    );
    let body = parts[2];
    if area.width >= 120 && area.height >= 19 {
        let columns = Layout::horizontal([Constraint::Percentage(58), Constraint::Percentage(42)])
            .split(body);
        repository_table(frame, screen, columns[0]);
        let rows = screen.rows();
        let selected = rows.get(screen.selected);
        let title = selected
            .map(|row| name(&row.path))
            .unwrap_or_else(|| gw_i18n::tr!(NoRepositorySelectedAlt).into());
        let text = selected
            .map(detail_text)
            .unwrap_or_else(|| gw_i18n::tr!(NoRepositories).into());
        frame.render_widget(
            Paragraph::new(text)
                .wrap(Wrap { trim: false })
                .block(block(title)),
            columns[1],
        );
    } else {
        repository_table(frame, screen, body)
    }
}
struct Badges {
    overall: &'static str,
    workspace: &'static str,
    remote: &'static str,
    experimental: String,
    push: String,
    color: Color,
}
fn badges(result: Option<&RepositoryResult>) -> Badges {
    let mut b = Badges {
        overall: "·",
        workspace: "?",
        remote: "?",
        experimental: "?".into(),
        push: "—".into(),
        color: MUTED,
    };
    let Some(result) = result else { return b };
    match result {
        RepositoryResult::Clean {
            branches_without_upstream,
            ..
        } => {
            b.overall = "✓";
            b.workspace = gw_i18n::tr!(CleanAlt);
            b.remote = gw_i18n::tr!(Passed);
            b.experimental = branches_without_upstream.len().to_string();
            b.color = GOOD
        }
        RepositoryResult::Attention {
            issues,
            branches_without_upstream,
            ..
        } => {
            b.overall = "!";
            b.workspace = gw_i18n::tr!(CleanAlt);
            b.remote = gw_i18n::tr!(Passed);
            b.experimental = branches_without_upstream.len().to_string();
            b.color = WARN;
            for issue in issues {
                match issue.kind {
                    IssueKind::Dirty => b.workspace = gw_i18n::tr!(Changes),
                    IssueKind::OutOfSync => b.remote = gw_i18n::tr!(OutOfSync),
                    IssueKind::MissingUpstream => b.remote = gw_i18n::tr!(MissingUpstream),
                    IssueKind::PushFailed => b.remote = gw_i18n::tr!(PushFailed),
                    IssueKind::NoCommits => b.remote = gw_i18n::tr!(NoCommits),
                    _ => {}
                }
            }
        }
        RepositoryResult::Failed { error, .. } => {
            b.overall = "×";
            b.remote = failure_label(error);
            b.color = BAD
        }
    }
    let good = result
        .pushes()
        .iter()
        .filter(|p| matches!(p, Publication::Pushed { .. }))
        .count();
    let bad = result.pushes().len() - good;
    if good + bad > 0 {
        b.push = format!("↑{good} !{bad}")
    }
    b
}
#[derive(Clone, Copy)]
enum FailureKind {
    Tls,
    Reference,
    Other,
}
fn failure_kind(error: &str) -> FailureKind {
    if error.contains("SSL_ERROR_SYSCALL") {
        FailureKind::Tls
    } else if error.contains("fetch refspec") {
        FailureKind::Reference
    } else {
        FailureKind::Other
    }
}
fn failure_label(error: &str) -> &'static str {
    match failure_kind(error) {
        FailureKind::Tls => gw_i18n::tr!(TlsFailed),
        FailureKind::Reference => gw_i18n::tr!(RefMismatch),
        FailureKind::Other => gw_i18n::tr!(CheckFailed),
    }
}
fn repository_table(frame: &mut Frame, screen: &Screen, area: Rect) {
    let rows = screen.rows();
    let data = rows
        .iter()
        .map(|row| {
            let b = badges(row.result.as_ref());
            TableRow::new(vec![
                Cell::from(format!("{} {}", b.overall, name(&row.path))),
                Cell::from(b.workspace),
                Cell::from(b.remote),
                Cell::from(b.experimental),
                Cell::from(b.push),
            ])
            .style(Style::default().fg(b.color))
        })
        .collect::<Vec<_>>();
    let table = Table::new(
        data,
        [
            Constraint::Min(12),
            Constraint::Length(9),
            Constraint::Length(12),
            Constraint::Length(6),
            Constraint::Length(8),
        ],
    )
    .header(
        TableRow::new([
            gw_i18n::tr!(Repositories),
            gw_i18n::tr!(Worktree),
            gw_i18n::tr!(Remote),
            gw_i18n::tr!(NoUpstreamColumn),
            gw_i18n::tr!(Pushes),
        ])
        .style(Style::default().fg(MUTED)),
    )
    .block(block(if screen.order == Order::Priority {
        gw_i18n::tr!(ListProblemsFirst)
    } else {
        gw_i18n::tr!(ListPathOrder)
    }))
    .row_highlight_style(selected_style())
    .highlight_symbol("› ")
    .column_spacing(1);
    let mut state = TableState::default().with_selected(if rows.is_empty() {
        None
    } else {
        Some(screen.selected)
    });
    frame.render_stateful_widget(table, area, &mut state);
    if rows.is_empty() {
        let inner = Rect {
            y: area.y + 2,
            height: area.height.saturating_sub(3),
            x: area.x + 1,
            width: area.width.saturating_sub(2),
        };
        frame.render_widget(
            Paragraph::new(gw_i18n::tr!(NoRepositoriesMatchAddToAddOne)),
            inner,
        );
    }
}
fn field(label: &str, value: impl Into<String>, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            gw_i18n::tf!(FieldLabel, label = label),
            Style::default().fg(MUTED),
        ),
        Span::styled(value.into(), Style::default().fg(color)),
    ])
}

pub(super) fn detail_text(row: &Row) -> Text<'static> {
    let mut lines = vec![field(
        gw_i18n::tr!(Path),
        safe(&row.path.to_string_lossy()),
        Color::Reset,
    )];
    let Some(result) = &row.result else {
        lines.push(field(gw_i18n::tr!(Status), gw_i18n::tr!(Unchecked), MUTED));
        lines.push(Line::from(gw_i18n::tr!(NotIncludedInTheLastReportWait)));
        return Text::from(lines);
    };
    let color = match result {
        RepositoryResult::Clean { .. } => GOOD,
        RepositoryResult::Attention { .. } => WARN,
        RepositoryResult::Failed { .. } => BAD,
    };
    lines.push(field(gw_i18n::tr!(Status), result.label(), color));
    match result {
        RepositoryResult::Failed { error, .. } => {
            let summary = match failure_kind(error) {
                FailureKind::Tls => gw_i18n::tr!(RemoteTlsConnectionFailedSslErrorSyscall),
                FailureKind::Reference => {
                    gw_i18n::tr!(UpstreamReferenceMismatchRemoteChangedOrFetch)
                }
                FailureKind::Other => gw_i18n::tr!(CheckFailedSeeOriginalError),
            };
            lines.push(field(gw_i18n::tr!(Reason), summary, BAD));
            for (label, value) in [
                (gw_i18n::tr!(Worktree), gw_i18n::tr!(Unknown)),
                (gw_i18n::tr!(RemoteCheck), gw_i18n::tr!(Incomplete)),
                (gw_i18n::tr!(BranchCounts), gw_i18n::tr!(Unknown)),
            ] {
                lines.push(field(label, value, MUTED));
            }
            lines.push(Line::default());
            lines.push(field(gw_i18n::tr!(OriginalError), "", Color::Reset));
            lines.extend(
                multiline(error)
                    .lines()
                    .map(|line| Line::raw(line.to_owned())),
            );
        }
        RepositoryResult::Clean { .. } => {
            lines.push(field(gw_i18n::tr!(Worktree), gw_i18n::tr!(CleanAlt), GOOD));
            lines.push(field(gw_i18n::tr!(RemoteCheck), gw_i18n::tr!(Passed), GOOD));
        }
        RepositoryResult::Attention { issues, .. } => {
            for issue in issues {
                let (label, color) = match issue.kind {
                    IssueKind::Dirty => (gw_i18n::tr!(Worktree), WARN),
                    IssueKind::DetachedHead => (gw_i18n::tr!(HeadState), WARN),
                    IssueKind::NoCommits => (gw_i18n::tr!(CommitState), WARN),
                    IssueKind::MissingUpstream => (gw_i18n::tr!(RemoteUpstream), WARN),
                    IssueKind::OutOfSync => (gw_i18n::tr!(RemoteSync), WARN),
                    IssueKind::PushFailed => (gw_i18n::tr!(PushFailed), BAD),
                };
                lines.push(field(label, safe(&issue.detail), color));
            }
        }
    }
    if let Some(branches) = result.branches() {
        lines.push(Line::default());
        lines.push(field(
            gw_i18n::tr!(NoRemoteUpstream),
            gw_i18n::tf!(Branches, branches.len()),
            Color::Reset,
        ));
        for branch in branches {
            lines.push(Line::raw(format!(
                "• {}{}",
                safe(&branch.branch),
                if branch.reason == gw_core::domain::UpstreamReason::LocalUpstream {
                    gw_i18n::tr!(TracksALocalBranch)
                } else {
                    ""
                }
            )));
        }
    }
    if !result.pushes().is_empty() {
        lines.push(Line::default());
        lines.push(field(gw_i18n::tr!(PushHistory), "", Color::Reset));
    }
    for push in result.pushes() {
        let target = push.target();
        lines.push(field(
            gw_i18n::tr!(Branch),
            safe(&target.branch),
            Color::Reset,
        ));
        lines.push(field(
            gw_i18n::tr!(PushTarget),
            format!("{}/{}", safe(&target.remote), safe(&target.remote_ref)),
            Color::Reset,
        ));
        match push {
            Publication::Pushed { .. } => lines.push(field(
                gw_i18n::tr!(PushResult),
                gw_i18n::tf!(PushedCommitsAlt, target.ahead_commits),
                GOOD,
            )),
            Publication::Failed { phase, error, .. } => {
                let phase = match phase {
                    gw_core::domain::Phase::Validate => gw_i18n::tr!(Validation),
                    gw_core::domain::Phase::Transfer => gw_i18n::tr!(Transfer),
                    gw_core::domain::Phase::Verify => gw_i18n::tr!(Verification),
                };
                lines.push(field(
                    gw_i18n::tr!(PushResult),
                    gw_i18n::tf!(PhasePhaseFailed, phase = phase),
                    BAD,
                ));
                lines.push(field(gw_i18n::tr!(PushError), "", Color::Reset));
                lines.extend(
                    multiline(error)
                        .lines()
                        .map(|line| Line::raw(line.to_owned())),
                );
            }
        }
    }
    Text::from(lines)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gw_core::domain::{Issue, Issues};

    #[test]
    fn detail_values_have_labels_for_each_report_state() {
        let _language = gw_i18n::using(gw_i18n::Language::Chinese);
        let path = std::path::PathBuf::from("/example/alpha");
        let row = |result| Row {
            path: path.clone(),
            result,
        };
        let missing = detail_text(&row(None)).to_string();
        assert!(missing.starts_with("路径：/example/alpha\n状态：未检查\n"));
        let clean = detail_text(&row(Some(RepositoryResult::Clean {
            path: path.clone(),
            branches_without_upstream: vec![],
            pushes: vec![],
        })))
        .to_string();
        assert!(clean.contains("状态：正常\n工作区：干净\n远程检查：通过"));
        assert!(clean.contains("未配置远程上游：0 个分支"));
        let attention = detail_text(&row(Some(RepositoryResult::Attention {
            path: path.clone(),
            issues: Issues::new(
                Issue {
                    kind: IssueKind::Dirty,
                    detail: "有未提交文件".into(),
                },
                vec![Issue {
                    kind: IssueKind::OutOfSync,
                    detail: "main 落后 2 个提交".into(),
                }],
            ),
            branches_without_upstream: vec![],
            pushes: vec![],
        })))
        .to_string();
        assert!(attention.contains("工作区：有未提交文件"));
        assert!(attention.contains("远程同步：main 落后 2 个提交"));
        let failed = detail_text(&row(Some(RepositoryResult::Failed {
            path: path.clone(),
            error: "SSL_ERROR_SYSCALL".into(),
            pushes: vec![],
        })))
        .to_string();
        assert!(failed.contains("状态：检查失败"));
        assert!(failed.contains("工作区：未知\n远程检查：未完成\n分支统计：未知"));
        assert!(failed.contains("原始错误：\nSSL_ERROR_SYSCALL"));
    }
    #[test]
    fn detail_label_and_value_styles_follow_semantic_state() {
        let _language = gw_i18n::using(gw_i18n::Language::Chinese);
        let path = std::path::PathBuf::from("/example/alpha");
        let results = [
            (None, MUTED),
            (
                Some(RepositoryResult::Clean {
                    path: path.clone(),
                    branches_without_upstream: vec![],
                    pushes: vec![],
                }),
                GOOD,
            ),
            (
                Some(RepositoryResult::Attention {
                    path: path.clone(),
                    issues: Issues::new(
                        Issue {
                            kind: IssueKind::Dirty,
                            detail: "未提交文件".into(),
                        },
                        vec![],
                    ),
                    branches_without_upstream: vec![],
                    pushes: vec![],
                }),
                WARN,
            ),
            (
                Some(RepositoryResult::Failed {
                    path: path.clone(),
                    error: "连接失败\n地址：https://example.invalid".into(),
                    pushes: vec![],
                }),
                BAD,
            ),
        ];
        for (result, expected) in results {
            let text = detail_text(&Row {
                path: path.clone(),
                result,
            });
            assert_eq!(text.lines[0].spans[0].style.fg, Some(MUTED));
            assert_eq!(text.lines[0].spans[1].style.fg, Some(Color::Reset));
            assert_eq!(text.lines[1].spans[0].style.fg, Some(MUTED));
            assert_eq!(text.lines[1].spans[1].style.fg, Some(expected));
            if expected == BAD {
                let error = text
                    .lines
                    .iter()
                    .find(|line| line.to_string().starts_with("地址："))
                    .unwrap();
                assert_eq!(error.spans.len(), 1);
                assert_eq!(error.spans[0].style.fg, None);
            }
        }
    }
    #[test]
    fn stored_reference_errors_are_recognized_in_both_languages() {
        for language in [gw_i18n::Language::English, gw_i18n::Language::Chinese] {
            let _language = gw_i18n::using(language);
            let error = gw_i18n::tf!(RemoteChangedAfterFetchOrFetchRefspec, "main");
            assert!(matches!(failure_kind(&error), FailureKind::Reference));
            let _other_language = gw_i18n::using(if language == gw_i18n::Language::English {
                gw_i18n::Language::Chinese
            } else {
                gw_i18n::Language::English
            });
            assert!(matches!(failure_kind(&error), FailureKind::Reference));
        }
    }
}
