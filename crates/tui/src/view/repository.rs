use super::*;
pub(super) fn repositories(frame: &mut Frame, screen: &mut Screen, area: Rect) {
    let parts = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(3),
    ])
    .split(area);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("filter ", Style::default().fg(MUTED)),
            Span::styled(screen.filter.label(), Style::default().fg(ACCENT)),
            Span::styled(" · sort ", Style::default().fg(MUTED)),
            Span::styled(screen.order.label(), Style::default().fg(ACCENT)),
        ])),
        parts[0],
    );
    frame.render_widget(
        Paragraph::new(i18n::tr!(WorktreeAndSyncLegend)).style(Style::default().fg(MUTED)),
        parts[1],
    );
    let body = parts[2];
    if area.width >= 120 && area.height >= 19 {
        let columns = Layout::horizontal([Constraint::Percentage(58), Constraint::Percentage(42)])
            .split(body);
        repository_table(frame, screen, columns[0]);
        let rows = screen.rows();
        let selected = rows.get(screen.selected());
        let title = selected
            .map(|row| name(&row.path))
            .unwrap_or_else(|| i18n::tr!(NoRepositorySelectedAlt).into());
        let text = selected
            .map(|row| repository_detail(screen, row))
            .unwrap_or_else(|| i18n::tr!(NoRepositories).into());
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
    sync: SyncStatus,
    color: Color,
}
#[derive(Debug, PartialEq, Eq)]
enum SyncStatus {
    Unknown,
    CheckFailed(&'static str),
    PushFailed(usize),
    Pending,
    Unavailable(&'static str),
    Synced { pushed: usize },
}
impl SyncStatus {
    fn from_result(result: &RepositoryResult) -> Self {
        if let RepositoryResult::Failed { error, .. } = result {
            return Self::CheckFailed(failure_label(error));
        }
        let failed = result
            .pushes()
            .iter()
            .filter(|p| matches!(p, Publication::Failed { .. }))
            .count();
        if failed > 0 {
            return Self::PushFailed(failed);
        }
        if let RepositoryResult::Attention { issues, .. } = result {
            // 按明确优先级选取状态，避免成功记录或问题顺序掩盖尚未解决的同步问题。
            for (kind, label) in [
                (IssueKind::PushFailed, i18n::tr!(PushFailed)),
                (IssueKind::MissingUpstream, i18n::tr!(MissingUpstream)),
                (IssueKind::NoCommits, i18n::tr!(NoCommits)),
            ] {
                if issues.iter().any(|issue| issue.kind == kind) {
                    return Self::Unavailable(label);
                }
            }
            if issues
                .iter()
                .any(|issue| issue.kind == IssueKind::OutOfSync)
            {
                return Self::Pending;
            }
        }
        Self::Synced {
            pushed: result.pushes().len(),
        }
    }
    fn label(&self) -> String {
        match self {
            Self::Unknown => "?".into(),
            Self::CheckFailed(label) | Self::Unavailable(label) => (*label).into(),
            Self::PushFailed(count) => i18n::tf!(SyncPushFailedCount, count),
            Self::Pending => i18n::tr!(OutOfSync).into(),
            Self::Synced { pushed: 0 } => i18n::tr!(Synced).into(),
            Self::Synced { pushed } => i18n::tf!(SyncPushedCount, pushed),
        }
    }
}
fn badges(result: Option<&RepositoryResult>) -> Badges {
    let mut b = Badges {
        overall: "·",
        workspace: "?",
        sync: SyncStatus::Unknown,
        color: MUTED,
    };
    let Some(result) = result else { return b };
    b.sync = SyncStatus::from_result(result);
    match result {
        RepositoryResult::Clean { .. } => {
            b.overall = "✓";
            b.workspace = i18n::tr!(CleanAlt);
            b.color = GOOD;
        }
        RepositoryResult::Attention { issues, .. } => {
            b.overall = "!";
            b.workspace = i18n::tr!(CleanAlt);
            b.color = WARN;
            if issues.iter().any(|issue| issue.kind == IssueKind::Dirty) {
                b.workspace = i18n::tr!(Changes);
            }
        }
        RepositoryResult::Failed { .. } => {
            b.overall = "×";
            b.color = BAD;
        }
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
        FailureKind::Tls => i18n::tr!(TlsFailed),
        FailureKind::Reference => i18n::tr!(RefMismatch),
        FailureKind::Other => i18n::tr!(CheckFailed),
    }
}
fn repository_table(frame: &mut Frame, screen: &mut Screen, area: Rect) {
    let rows = screen.rows();
    let data = rows
        .iter()
        .map(|row| {
            let b = badges(row.result.as_ref());
            TableRow::new(vec![
                Cell::from(format!(
                    "{} {}",
                    screen
                        .check_task
                        .as_ref()
                        .filter(|task| task.checking(&row.path))
                        .map(|task| task.spinner())
                        .or_else(|| screen
                            .repository_tasks
                            .get(&row.path)
                            .map(|task| task.animation.symbol()))
                        .unwrap_or(b.overall),
                    name(&row.path)
                )),
                Cell::from(b.workspace),
                Cell::from(b.sync.label()),
            ])
            .style(Style::default().fg(b.color))
        })
        .collect::<Vec<_>>();
    let border = block(format!("{} · {}", i18n::tr!(Repositories), rows.len()))
        .padding(ratatui::widgets::Padding::left(1));
    let content = border.inner(area);
    let table = Table::new(
        data,
        [
            Constraint::Min(12),
            Constraint::Length(9),
            Constraint::Length(18),
        ],
    )
    .header(
        TableRow::new([
            i18n::tr!(Repositories),
            i18n::tr!(Worktree),
            i18n::tr!(SyncColumn),
        ])
        .style(Style::default().fg(MUTED)),
    )
    .block(border)
    .row_highlight_style(selected_style())
    .column_spacing(1);
    screen.repository_area = Some(area);
    frame.render_stateful_widget(table, area, &mut screen.repository_viewport);
    let inner = Rect::new(
        content.x,
        content.y + 1,
        content.width,
        content.height.saturating_sub(1),
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
    let color = match result {
        RepositoryResult::Clean { .. } => GOOD,
        RepositoryResult::Attention { .. } => WARN,
        RepositoryResult::Failed { .. } => BAD,
    };
    lines.push(field(i18n::tr!(Status), result.label(), color));
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
            lines.push(field(i18n::tr!(RemoteCheck), i18n::tr!(Passed), GOOD));
        }
        RepositoryResult::Attention { issues, .. } => {
            for issue in issues {
                let (label, color) = match issue.kind {
                    IssueKind::Dirty => (i18n::tr!(Worktree), WARN),
                    IssueKind::DetachedHead => (i18n::tr!(HeadState), WARN),
                    IssueKind::NoCommits => (i18n::tr!(CommitState), WARN),
                    IssueKind::MissingUpstream => (i18n::tr!(RemoteUpstream), WARN),
                    IssueKind::OutOfSync => (i18n::tr!(RemoteSync), WARN),
                    IssueKind::PushFailed => (i18n::tr!(PushFailed), BAD),
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
            path: "/example/repo".into(),
            issues: Issues::new(issues.next().unwrap(), issues.collect()),
            branches_without_upstream: vec![],
            pushes,
        }
    }
    #[test]
    fn sync_state_prioritizes_failures_and_pending_branches_over_successful_pushes() {
        assert_eq!(
            SyncStatus::from_result(&RepositoryResult::Clean {
                path: "/example/repo".into(),
                branches_without_upstream: vec![],
                pushes: vec![]
            }),
            SyncStatus::Synced { pushed: 0 }
        );
        assert_eq!(
            SyncStatus::from_result(&attention(vec![IssueKind::Dirty], vec![publication(false)])),
            SyncStatus::Synced { pushed: 1 }
        );
        assert_eq!(
            SyncStatus::from_result(&attention(
                vec![IssueKind::OutOfSync],
                vec![publication(false)]
            )),
            SyncStatus::Pending
        );
        assert_eq!(
            SyncStatus::from_result(&attention(
                vec![IssueKind::OutOfSync, IssueKind::PushFailed],
                vec![publication(false), publication(true)]
            )),
            SyncStatus::PushFailed(1)
        );
        let failed = RepositoryResult::Failed {
            path: "/example/repo".into(),
            error: "SSL_ERROR_SYSCALL".into(),
            pushes: vec![publication(false), publication(true)],
        };
        assert_eq!(
            SyncStatus::from_result(&failed),
            SyncStatus::CheckFailed(i18n::tr!(TlsFailed))
        );
        for kinds in [
            vec![IssueKind::OutOfSync, IssueKind::MissingUpstream],
            vec![IssueKind::MissingUpstream, IssueKind::OutOfSync],
        ] {
            assert_eq!(
                SyncStatus::from_result(&attention(kinds, vec![])),
                SyncStatus::Unavailable(i18n::tr!(MissingUpstream))
            );
        }
    }
    #[test]
    fn sync_labels_are_localized_without_losing_push_counts() {
        for (language, synced, pushed, failed, pending) in [
            (
                i18n::Language::English,
                "Synced",
                "Pushed 2",
                "Push failed · 1",
                "Out of sync",
            ),
            (
                i18n::Language::Chinese,
                "已同步",
                "已推送 2",
                "推送失败 · 1",
                "待同步",
            ),
        ] {
            let _language = i18n::using(language);
            assert_eq!(SyncStatus::Unknown.label(), "?");
            assert_eq!(SyncStatus::Synced { pushed: 0 }.label(), synced);
            assert_eq!(SyncStatus::Synced { pushed: 2 }.label(), pushed);
            assert_eq!(SyncStatus::PushFailed(1).label(), failed);
            assert_eq!(SyncStatus::Pending.label(), pending);
        }
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
        let _language = i18n::using(i18n::Language::Chinese);
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
