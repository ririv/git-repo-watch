mod heading;
mod status;
use super::*;
use crate::repository_list::Execution;
pub(super) fn repositories(frame: &mut Frame, screen: &mut Screen, area: Rect) {
    if area.width >= 120 && area.height >= 19 {
        let columns = Layout::horizontal([Constraint::Percentage(58), Constraint::Percentage(42)])
            .split(area);
        repository_table(frame, screen, columns[0]);
        let rows = screen.rows();
        let selected = rows.get(screen.selected());
        let title = selected
            .map(|row| name(&row.path))
            .unwrap_or_else(|| i18n::tr!(NoRepositorySelectedAlt).into());
        let text = selected
            .map(|row| repository_detail(screen, row))
            .unwrap_or_else(|| i18n::tr!(NoRepositories).into());
        let panel = block(title);
        let inner = panel.inner(columns[1]);
        frame.render_widget(panel, columns[1]);
        let text = screen.pointer.links.text(text);
        screen.pointer.links.paragraph(frame, inner, &text, 0);
    } else {
        repository_table(frame, screen, area)
    }
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
fn repository_table(frame: &mut Frame, screen: &mut Screen, area: Rect) {
    let rows = screen.rows();
    let content = heading::render(
        frame,
        format!("{} · {}", i18n::tr!(Repositories), rows.len()),
        screen.filter,
        screen.order,
        area,
    );
    let columns = match content.width {
        41.. => vec![
            Constraint::Min(12),
            Constraint::Length(9),
            Constraint::Length(18),
        ],
        25.. => vec![Constraint::Min(12), Constraint::Length(9)],
        _ => vec![Constraint::Min(1)],
    };
    let column_count = columns.len();
    let data = rows
        .iter()
        .map(|row| {
            let execution = screen.repository_execution(&row.path);
            let running = screen
                .check_task
                .as_ref()
                .filter(|task| task.checking(&row.path))
                .map(|task| task.spinner())
                .or_else(|| {
                    screen
                        .repository_tasks
                        .get(&row.path)
                        .map(|task| task.animation.symbol())
                });
            let (workspace, workspace_color) = status::workspace(row.result.as_ref());
            let (sync, sync_color) = status::synchronization(row.result.as_ref());
            let mut data = TableRow::new(
                [
                    Cell::from(Line::from(vec![
                        Span::styled(
                            format!("{} ", running.unwrap_or(execution.symbol())),
                            Style::default().fg(if running.is_some() {
                                ACCENT
                            } else {
                                status::color(execution)
                            }),
                        ),
                        Span::raw(name(&row.path)),
                    ])),
                    Cell::from(workspace).style(Style::default().fg(workspace_color)),
                    Cell::from(sync).style(Style::default().fg(sync_color)),
                ]
                .into_iter()
                .take(column_count),
            );
            if screen.search_mode == crate::search::Mode::Locate
                && !screen.query.is_empty()
                && row
                    .path
                    .to_string_lossy()
                    .to_lowercase()
                    .contains(&screen.query.to_lowercase())
            {
                data = data.style(Style::default().bold());
            }
            data
        })
        .collect::<Vec<_>>();
    let headings = [
        i18n::tr!(Repositories),
        i18n::tr!(Worktree),
        i18n::tr!(SyncColumn),
    ];
    let heading = u16::from(content.height > 1);
    let mut table = Table::new(data, columns)
        .row_highlight_style(selected_style())
        .column_spacing(1);
    if heading > 0 {
        table = table.header(
            TableRow::new(headings.into_iter().take(column_count))
                .style(Style::default().fg(MUTED)),
        );
    }
    screen.repository_area = Some(area);
    frame.render_stateful_widget(table, content, &mut screen.repository_viewport);
    let inner = Rect::new(
        content.x,
        content.y + heading,
        content.width,
        content.height.saturating_sub(heading),
    );
    crate::scrollbar::vertical(
        frame,
        area,
        inner,
        rows.len(),
        screen.repository_viewport.offset(),
    );
    screen.pointer.list(
        area,
        inner,
        &(0..rows.len()).collect::<Vec<_>>(),
        screen.repository_viewport.offset(),
        crate::pointer::Target::Repositories,
    );
    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new(if screen.config.repositories.is_empty() {
                i18n::tr!(NoRepositoriesMatchAddToAddOne)
            } else {
                i18n::tr!(RepositoryListNoMatches)
            }),
            inner,
        );
    }
}
fn field(label: &str, value: impl Into<String>, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            i18n::tf!(FieldLabel, label = label),
            Style::default().fg(MUTED),
        ),
        Span::styled(value.into(), Style::default().fg(color)),
    ])
}

pub(super) fn repository_detail(screen: &Screen, row: &Row) -> Text<'static> {
    let mut text = detail_text(row);
    if let Some(error) = screen.repository_errors.get(&row.path) {
        if text.lines.len() > 1 {
            text.lines[1] = field(i18n::tr!(Status), Execution::Failed.label(), BAD);
        }
        text.lines
            .push(field(i18n::tr!(OperationResult), safe(error), BAD));
    }
    if let Some(update) = screen.latest_update(&row.path) {
        text.lines.insert(
            1,
            field(
                i18n::tr!(CheckUpdatedAt),
                local_time(&update.finished_at.to_rfc3339()),
                Color::Reset,
            ),
        );
    }
    text
}
pub(super) fn detail_text(row: &Row) -> Text<'static> {
    let mut lines = vec![field(
        i18n::tr!(Path),
        safe(&row.path.to_string_lossy()),
        Color::Reset,
    )];
    let Some(result) = &row.result else {
        lines.push(field(i18n::tr!(Status), i18n::tr!(Unchecked), MUTED));
        lines.push(Line::from(i18n::tr!(NotIncludedInTheLastReportWait)));
        return Text::from(lines);
    };
    let execution = Execution::of(Some(result));
    lines.push(field(
        i18n::tr!(Status),
        execution.label(),
        status::color(execution),
    ));
    if !matches!(result, RepositoryResult::Failed { .. }) {
        let (sync, color) = status::synchronization(Some(result));
        lines.push(field(i18n::tr!(SyncColumn), sync, color));
    }
    match result {
        RepositoryResult::Failed { error, .. } => {
            let summary = match failure_kind(error) {
                FailureKind::Tls => i18n::tr!(RemoteTlsConnectionFailedSslErrorSyscall),
                FailureKind::Reference => {
                    i18n::tr!(UpstreamReferenceMismatchRemoteChangedOrFetch)
                }
                FailureKind::Other => i18n::tr!(CheckFailedSeeOriginalError),
            };
            lines.push(field(i18n::tr!(Reason), summary, BAD));
            for (label, value) in [
                (i18n::tr!(Worktree), i18n::tr!(Unknown)),
                (i18n::tr!(RemoteCheck), i18n::tr!(Incomplete)),
                (i18n::tr!(BranchCounts), i18n::tr!(Unknown)),
            ] {
                lines.push(field(label, value, MUTED));
            }
            lines.push(Line::default());
            lines.push(field(i18n::tr!(OriginalError), "", Color::Reset));
            lines.extend(
                multiline(error)
                    .lines()
                    .map(|line| Line::raw(line.to_owned())),
            );
        }
        RepositoryResult::Clean { .. } => {
            lines.push(field(i18n::tr!(Worktree), i18n::tr!(CleanAlt), GOOD));
        }
        RepositoryResult::Attention { issues, .. } => {
            for issue in issues {
                let (label, color) = match issue.kind {
                    IssueKind::Dirty => (i18n::tr!(Worktree), ACCENT),
                    IssueKind::DetachedHead => (i18n::tr!(HeadState), MUTED),
                    IssueKind::NoCommits => (i18n::tr!(CommitState), MUTED),
                    IssueKind::MissingUpstream => (i18n::tr!(RemoteUpstream), MUTED),
                    IssueKind::OutOfSync => (i18n::tr!(RemoteSync), ACCENT),
                    IssueKind::PushFailed => (i18n::tr!(PushFailed), BAD),
                    IssueKind::OperationFailed => (i18n::tr!(OperationResult), BAD),
                };
                lines.push(field(label, safe(&issue.detail), color));
            }
        }
    }
    if let Some(branches) = result.branches() {
        lines.push(Line::default());
        lines.push(field(
            i18n::tr!(NoRemoteUpstream),
            i18n::tf!(Branches, branches.len()),
            Color::Reset,
        ));
        for branch in branches {
            lines.push(Line::raw(format!(
                "• {}{}",
                safe(&branch.branch),
                if branch.reason == engine::domain::UpstreamReason::LocalUpstream {
                    i18n::tr!(TracksALocalBranch)
                } else {
                    ""
                }
            )));
        }
    }
    if !result.pushes().is_empty() {
        lines.push(Line::default());
        lines.push(field(i18n::tr!(PushHistory), "", Color::Reset));
    }
    for push in result.pushes() {
        let target = push.target();
        lines.push(field(i18n::tr!(Branch), safe(&target.branch), Color::Reset));
        lines.push(field(
            i18n::tr!(PushTarget),
            format!("{}/{}", safe(&target.remote), safe(&target.remote_ref)),
            Color::Reset,
        ));
        match push {
            Publication::Pushed { .. } => lines.push(field(
                i18n::tr!(PushResult),
                i18n::tf!(PushedCommitsAlt, target.ahead_commits),
                GOOD,
            )),
            Publication::Failed { phase, error, .. } => {
                let phase = match phase {
                    engine::domain::Phase::Validate => i18n::tr!(Validation),
                    engine::domain::Phase::Transfer => i18n::tr!(Transfer),
                    engine::domain::Phase::Verify => i18n::tr!(Verification),
                };
                lines.push(field(
                    i18n::tr!(PushResult),
                    i18n::tf!(PhasePhaseFailed, phase = phase),
                    BAD,
                ));
                lines.push(field(i18n::tr!(PushError), "", Color::Reset));
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
    use engine::domain::{Issue, Issues};

    fn publication(failed: bool) -> Publication {
        let target = engine::domain::PushTarget {
            branch: "main".into(),
            local_ref: "refs/heads/main".into(),
            local_oid: "a".repeat(40),
            remote: "origin".into(),
            remote_ref: "refs/heads/main".into(),
            remote_oid: "b".repeat(40),
            ahead_commits: 1,
        };
        if failed {
            Publication::Failed {
                target,
                phase: engine::domain::Phase::Transfer,
                error: "rejected".into(),
            }
        } else {
            Publication::Pushed { target }
        }
    }
    fn attention(kinds: Vec<IssueKind>, pushes: Vec<Publication>) -> RepositoryResult {
        let mut issues = kinds.into_iter().map(|kind| Issue {
            kind,
            detail: "test".into(),
        });
        RepositoryResult::Attention {
            synchronization: Default::default(),
            path: "/example/repo".into(),
            issues: Issues::new(issues.next().unwrap(), issues.collect()),
            branches_without_upstream: vec![],
            pushes,
        }
    }
    #[test]
    fn ordinary_repository_conditions_do_not_warn_and_actual_failures_do() {
        for kind in [
            IssueKind::Dirty,
            IssueKind::DetachedHead,
            IssueKind::NoCommits,
            IssueKind::MissingUpstream,
            IssueKind::OutOfSync,
        ] {
            assert_eq!(
                Execution::of(Some(&attention(vec![kind], vec![]))),
                Execution::Succeeded
            );
        }
        assert_eq!(
            Execution::of(Some(&attention(
                vec![IssueKind::PushFailed],
                vec![publication(false), publication(true)]
            ))),
            Execution::PartialFailure
        );
        assert_eq!(
            Execution::of(Some(&attention(
                vec![IssueKind::PushFailed],
                vec![publication(true)]
            ))),
            Execution::Failed
        );
        assert_eq!(
            Execution::of(Some(&attention(vec![IssueKind::OperationFailed], vec![]))),
            Execution::Failed
        );
        let partial = RepositoryResult::Failed {
            path: "/example/repo".into(),
            error: "partially failed".into(),
            pushes: vec![publication(false), publication(true)],
        };
        assert_eq!(status::workspace(Some(&partial)).1, WARN);
        assert_eq!(status::synchronization(Some(&partial)).1, WARN);
    }
    #[test]
    fn synchronization_counts_branches_in_both_directions_and_never_shows_push_outcomes() {
        use engine::domain::synchronization::{Comparison, Synchronization};
        let mut result = attention(vec![IssueKind::OutOfSync], vec![publication(false)]);
        assert_eq!(status::synchronization(Some(&result)), ("?".into(), MUTED));
        let RepositoryResult::Attention {
            synchronization, ..
        } = &mut result
        else {
            unreachable!()
        };
        *synchronization = Synchronization::Known {
            branches: vec![
                Comparison::Ahead,
                Comparison::Behind,
                Comparison::Diverged,
                Comparison::Equal,
            ],
        };
        assert_eq!(
            status::synchronization(Some(&result)),
            ("↑2 ↓2".into(), ACCENT)
        );
        let RepositoryResult::Attention {
            synchronization, ..
        } = &mut result
        else {
            unreachable!()
        };
        *synchronization = Synchronization::Known { branches: vec![] };
        assert_eq!(status::synchronization(Some(&result)), ("—".into(), MUTED));
    }
    #[test]
    fn detail_values_have_labels_for_each_report_state() {
        let _language = i18n::using(i18n::Language::Chinese);
        let path = std::path::PathBuf::from("/example/alpha");
        let row = |result| Row {
            path: path.clone(),
            result,
        };
        let missing = detail_text(&row(None)).to_string();
        assert!(missing.starts_with("路径：/example/alpha\n状态：未检查\n"));
        let clean = detail_text(&row(Some(RepositoryResult::Clean {
            synchronization: Default::default(),
            path: path.clone(),
            branches_without_upstream: vec![],
            pushes: vec![],
        })))
        .to_string();
        assert!(clean.contains("状态：正常\n同步：?\n工作区：干净"));
        assert!(clean.contains("未配置远程上游：0 个分支"));
        let attention = detail_text(&row(Some(RepositoryResult::Attention {
            synchronization: Default::default(),
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
        assert!(failed.contains("状态：失败"));
        assert!(failed.contains("工作区：未知\n远程检查：未完成\n分支统计：未知"));
        assert!(failed.contains("原始错误：\nSSL_ERROR_SYSCALL"));
    }
    #[test]
    fn detail_label_and_value_styles_follow_semantic_state() {
        let _language = i18n::using(i18n::Language::Chinese);
        let path = std::path::PathBuf::from("/example/alpha");
        let results = [
            (None, MUTED),
            (
                Some(RepositoryResult::Clean {
                    synchronization: Default::default(),
                    path: path.clone(),
                    branches_without_upstream: vec![],
                    pushes: vec![],
                }),
                GOOD,
            ),
            (
                Some(RepositoryResult::Attention {
                    synchronization: Default::default(),
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
                GOOD,
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
        for language in [i18n::Language::English, i18n::Language::Chinese] {
            let _language = i18n::using(language);
            let error = i18n::tf!(RemoteChangedAfterFetchOrFetchRefspec, "main");
            assert!(matches!(failure_kind(&error), FailureKind::Reference));
            let _other_language = i18n::using(if language == i18n::Language::English {
                i18n::Language::Chinese
            } else {
                i18n::Language::English
            });
            assert!(matches!(failure_kind(&error), FailureKind::Reference));
        }
    }
}
