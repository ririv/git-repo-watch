use super::*;
use std::path::Path;

fn clean(path: &str) -> RepositoryResult {
    RepositoryResult::Clean {
        path: path.into(),
        branches_without_upstream: vec![],
        pushes: vec![],
    }
}
fn attention(path: &str) -> RepositoryResult {
    RepositoryResult::Attention {
        path: path.into(),
        issues: Issues::new(
            Issue {
                kind: IssueKind::Dirty,
                detail: "modified".into(),
            },
            vec![],
        ),
        branches_without_upstream: vec![],
        pushes: vec![],
    }
}
fn failed(path: &str) -> RepositoryResult {
    RepositoryResult::Failed {
        path: path.into(),
        error: "repo fetch failed".into(),
        pushes: vec![],
    }
}
fn update(s: &mut Screen, result: RepositoryResult, at: &str, revision: u64) {
    s.merge_update(
        result.path().to_owned(),
        checks::Update {
            result,
            finished_at: chrono::DateTime::parse_from_rfc3339(at).unwrap(),
            revision,
        },
    );
}
fn paths(s: &Screen) -> Vec<PathBuf> {
    s.rows().into_iter().map(|row| row.path).collect()
}

#[test]
fn filters_distinguish_missing_results_failures_and_independent_upstream_push_records() {
    let mut no_upstream = clean("/example/clean");
    let RepositoryResult::Clean {
        branches_without_upstream,
        pushes,
        ..
    } = &mut no_upstream
    else {
        unreachable!()
    };
    branches_without_upstream.push(BranchWithoutUpstream {
        branch: "experiment".into(),
        reason: UpstreamReason::NotConfigured,
    });
    pushes.push(Publication::Pushed {
        target: PushTarget {
            branch: "main".into(),
            local_ref: "refs/heads/main".into(),
            local_oid: "a".repeat(40),
            remote: "origin".into(),
            remote_ref: "refs/heads/main".into(),
            remote_oid: "b".repeat(40),
            ahead_commits: 1,
        },
    });
    let clean = clean("/example/other");
    let attention = attention("/example/dirty");
    let failed = failed("/example/failed");
    let results = [
        None,
        Some(&clean),
        Some(&attention),
        Some(&failed),
        Some(&no_upstream),
    ];
    for (filter, expected) in [
        (Filter::All, vec![0, 1, 2, 3, 4]),
        (Filter::Problems, vec![2, 3]),
        (Filter::Clean, vec![1, 4]),
        (Filter::Failed, vec![3]),
        (Filter::Unchecked, vec![0]),
        (Filter::WithoutUpstream, vec![4]),
        (Filter::Pushes, vec![4]),
    ] {
        assert_eq!(
            results
                .iter()
                .enumerate()
                .filter_map(|(index, result)| filter.matches(*result).then_some(index))
                .collect::<Vec<_>>(),
            expected
        );
    }
}

#[test]
fn sorting_uses_status_name_full_path_or_per_repository_time_with_stable_ties() {
    let a = "/z/Alpha";
    let b = "/a/beta";
    let c = "/b/alpha";
    let d = "/a/中文";
    let (_home, mut s) = fixture([a, b, c, d].map(PathBuf::from).to_vec());
    update(&mut s, clean(a), "2026-10-23T10:00:00+08:00", 1);
    update(&mut s, failed(b), "2026-10-23T01:00:00Z", 2);
    update(&mut s, attention(c), "2026-10-23T02:00:00Z", 3);
    for (order, expected) in [
        (Order::Priority, [b, c, d, a]),
        (Order::Name, [c, a, b, d]),
        (Order::Path, [b, d, c, a]),
        (Order::Updated, [c, a, b, d]),
    ] {
        s.sort_repositories(order);
        assert_eq!(paths(&s), expected.map(PathBuf::from));
    }
    // 新的单仓库记录立即影响排序，不需要等待整轮报告或读取数据库。
    update(&mut s, clean(b), "2026-10-23T03:00:00Z", 4);
    assert_eq!(paths(&s), [b, c, a, d].map(PathBuf::from));
}

#[test]
fn updated_sort_includes_batch_results_and_prefers_newer_repository_observations() {
    let a = "/example/a";
    let b = "/example/b";
    let c = "/example/c";
    let (_home, mut s) = fixture([a, b, c].map(PathBuf::from).to_vec());
    s.snapshot.report = CachedReport::Available {
        scope: service::Scope::Current,
        report: Box::new(Report {
            schema_version: 3,
            repository_finished_at: [(a.into(), "2026-10-23T00:00:00Z".into())].into(),
            repository_revisions: BTreeMap::new(),
            push_policy: PushPolicy::Disabled,
            notifications: vec![],
            started_at: "2026-10-22T23:00:00Z".into(),
            finished_at: "2026-10-23T01:00:00Z".into(),
            status: ReportStatus::Clean,
            repositories: vec![clean(a), clean(b)],
            classification_updated_at: None,
            execution: None,
        }),
    };
    s.sort_repositories(Order::Updated);
    assert_eq!(paths(&s), [b, a, c].map(PathBuf::from));
    update(&mut s, clean(a), "2026-10-23T02:00:00Z", 1);
    assert_eq!(paths(&s), [a, b, c].map(PathBuf::from));
}

#[test]
fn list_menus_focus_current_options_cancel_without_changes_and_apply_from_empty_lists() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec!["/z/beta".into(), "/a/alpha".into()]);
        s.focus(Some("/z/beta".into()));
        let selected = s.selected_path();
        let configuration = std::fs::read(s.service.store.config_path()).unwrap();
        command(&mut s, "repos sort");
        assert_eq!(s.bar.text(), "/repos sort ");
        assert!(matches!(s.bar, bar::InputBar::Command { selected: 0, .. }));
        key(&mut s, KeyCode::Down);
        assert_eq!(s.order, Order::Priority);
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.selected_path(), selected);
        assert_eq!(s.order, Order::Priority);
        command(&mut s, "repos sort name");
        assert_eq!(s.order, Order::Name);
        assert_eq!(s.selected_path(), selected);
        command(&mut s, "repos sort");
        assert!(matches!(s.bar, bar::InputBar::Command { selected: 1, .. }));
        for (width, height) in [(60, 18), (100, 28), (180, 36)] {
            let text = render(&mut s, width, height);
            assert!(text.contains("[✓]"), "{text}");
            assert!(text.contains(i18n::tr!(RepositorySortCommand)), "{text}");
        }
        key(&mut s, KeyCode::Esc);
        command(&mut s, "repos filter failed");
        assert!(s.rows().is_empty());
        assert_eq!(s.selected_path(), None);
        assert!(render(&mut s, 60, 18).contains(i18n::tr!(RepositoryListNoMatches)));
        command(&mut s, "repos filter");
        assert!(matches!(s.bar, bar::InputBar::Command { selected: 3, .. }));
        key(&mut s, KeyCode::Home);
        // Home 编辑命令光标，不改变选项；↑ 与 Enter 仍可恢复全部。
        for _ in 0..3 {
            key(&mut s, KeyCode::Up);
        }
        key(&mut s, KeyCode::Enter);
        assert_eq!(s.filter, Filter::All);
        assert_eq!(s.selected_path(), Some("/a/alpha".into()));
        assert_eq!(
            std::fs::read(s.service.store.config_path()).unwrap(),
            configuration
        );
        assert!(s.service.store.report().unwrap().is_none());
        assert!(s.repository_tasks.is_empty());
        assert!(s.check_task.is_none());
    }
}

#[test]
fn list_commands_combine_with_search_and_keep_repo_scope_after_reordering() {
    let (_home, mut s) = fixture(vec![
        "/z/中文 beta".into(),
        "/a/中文 alpha".into(),
        "/b/other".into(),
    ]);
    update(&mut s, clean("/z/中文 beta"), "2026-10-23T00:00:00Z", 1);
    s.paste("中文");
    s.focus(Some("/z/中文 beta".into()));
    key(&mut s, KeyCode::Char('/'));
    s.paste("filter");
    key(&mut s, KeyCode::Enter);
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.query, "中文");
    assert_eq!(s.bar.editor().text, "中文");
    assert_eq!(s.filter, Filter::All);
    assert_eq!(s.selected_path(), Some("/z/中文 beta".into()));
    key(&mut s, KeyCode::Char('/'));
    s.paste("sort name");
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.query, "中文");
    assert_eq!(s.bar.editor().text, "中文");
    assert_eq!(s.selected_path(), Some("/z/中文 beta".into()));
    key(&mut s, KeyCode::Char('/'));
    s.paste("filter clean");
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.query, "中文");
    assert_eq!(s.bar.editor().text, "中文");
    assert_eq!(paths(&s), [PathBuf::from("/z/中文 beta")]);
    key(&mut s, KeyCode::Char('/'));
    s.paste("filter unchecked");
    key(&mut s, KeyCode::Enter);
    assert_eq!(paths(&s), [PathBuf::from("/a/中文 alpha")]);
    command(&mut s, "repos remove");
    assert!(
        matches!(&s.mode, Mode::Confirm { operation: Confirmation::Remove(path), .. } if path == Path::new("/a/中文 alpha"))
    );
}

#[test]
fn list_menus_support_mouse_and_do_not_take_over_other_views() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let (_home, mut s) = fixture(vec!["/example/a".into()]);
    command(&mut s, "repos filter");
    render(&mut s, 140, 30);
    let (area, _) = s
        .command_items
        .iter()
        .find(|(_, index)| {
            s.bar.commands(s.config.ai.enabled).unwrap()[*index].action
                == Action::Filter(Filter::Unchecked)
        })
        .copied()
        .unwrap();
    s.mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: area.x,
            row: area.y,
            modifiers: KeyModifiers::NONE,
        },
        &Cancel::default(),
    )
    .unwrap();
    assert_eq!(s.filter, Filter::Unchecked);
    command(&mut s, "help");
    command(&mut s, "repos sort name");
    assert_eq!(s.page, Page::Help);
    assert_eq!(s.order, Order::Priority);
    assert_eq!(s.message, i18n::tr!(RepositoryListOnly));
}
