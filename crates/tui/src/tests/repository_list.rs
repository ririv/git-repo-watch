use super::*;
use std::path::Path;

fn clean(path: &str) -> RepositoryResult {
    RepositoryResult::Clean {
        synchronization: Default::default(),
        path: path.into(),
        branches_without_upstream: vec![],
        pushes: vec![],
    }
}
fn attention(path: &str) -> RepositoryResult {
    RepositoryResult::Attention {
        synchronization: Default::default(),
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
            remote_oid: engine::domain::RemoteTip::Existing("b".repeat(40)),
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
                .filter_map(|(index, result)| filter
                    .matches(*result, crate::repository_list::Status::of(*result))
                    .then_some(index))
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
        let mut configuration = s.service.store.config().unwrap();
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
        assert!(
            matches!(s.bar, bar::InputBar::Command { menu: command::completion::Menu::Options(ref values), .. } if values.selected == 1)
        );
        for (width, height) in [(60, 18), (100, 28), (180, 36)] {
            let text = render(&mut s, width, height);
            assert!(
                text.lines()
                    .any(|line| line.contains("name") && line.contains('✓')),
                "{text}"
            );
            assert!(!text.contains("[✓]"), "{text}");
            assert!(!text.contains(i18n::tr!(RepositorySortCommand)), "{text}");
        }
        key(&mut s, KeyCode::Esc);
        command(&mut s, "repos filter failed");
        assert!(s.rows().is_empty());
        assert_eq!(s.selected_path(), None);
        assert!(render(&mut s, 60, 18).contains(i18n::tr!(RepositoryListNoMatches)));
        command(&mut s, "repos filter");
        assert!(
            matches!(s.bar, bar::InputBar::Command { menu: command::completion::Menu::Options(ref values), .. } if values.selected == 3)
        );
        // 参数候选中的 ↑ 与 Enter 可恢复全部。
        for _ in 0..3 {
            key(&mut s, KeyCode::Up);
        }
        key(&mut s, KeyCode::Enter);
        assert_eq!(s.filter, Filter::All);
        assert_eq!(s.selected_path(), Some("/a/alpha".into()));
        wait(&mut s);
        configuration.view.repository_order = engine::state::view::RepositoryOrder::Name;
        assert_eq!(s.service.store.config().unwrap(), configuration);
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
    key(&mut s, KeyCode::Home);
    key(&mut s, KeyCode::Char('/'));
    s.paste("repos filter");
    key(&mut s, KeyCode::Enter);
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.query, "中文");
    assert_eq!(s.bar.editor().text, "中文");
    assert_eq!(s.filter, Filter::All);
    assert_eq!(s.selected_path(), Some("/z/中文 beta".into()));
    key(&mut s, KeyCode::Home);
    key(&mut s, KeyCode::Char('/'));
    s.paste("repos sort name");
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.query, "中文");
    assert_eq!(s.bar.editor().text, "中文");
    assert_eq!(s.selected_path(), Some("/z/中文 beta".into()));
    key(&mut s, KeyCode::Home);
    key(&mut s, KeyCode::Char('/'));
    s.paste("repos filter clean");
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.query, "中文");
    assert_eq!(s.bar.editor().text, "中文");
    assert_eq!(paths(&s), [PathBuf::from("/z/中文 beta")]);
    key(&mut s, KeyCode::Home);
    key(&mut s, KeyCode::Char('/'));
    s.paste("repos filter unchecked");
    key(&mut s, KeyCode::Enter);
    assert_eq!(paths(&s), [PathBuf::from("/a/中文 alpha")]);
    command(&mut s, "repos remove");
    assert!(
        matches!(&s.mode, Mode::Confirm { operation: Confirmation::Remove { path, .. }, .. } if path == Path::new("/a/中文 alpha"))
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
            matches!(&s.bar, bar::InputBar::Command { menu: command::completion::Menu::Options(values), .. } if values.options[*index].value.action() == Action::Filter(Filter::Unchecked))
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

#[test]
fn repositories_without_remotes_remain_normal_after_scheduled_and_manual_fetch() {
    let _language = i18n::using(i18n::Language::Chinese);
    let f = support::Fixture::create();
    support::git(&f.repo, &["remote", "remove", "origin"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    s.service.run_scheduled(&Cancel::default(), |_| {}).unwrap();
    s.reload(&Cancel::default()).unwrap();
    let text = render(&mut s, 100, 28);
    assert!(text.contains("正常 1 · 需关注 0 · 失败 0"), "{text}");
    assert!(
        text.lines()
            .any(|line| line.contains("✓ repo") && line.contains("—")),
        "{text}"
    );
    s.filter_repositories(Filter::Problems);
    assert!(s.rows().is_empty());
    s.filter_repositories(Filter::Clean);
    assert_eq!(s.rows().len(), 1);
    command(&mut s, "repo fetch");
    wait(&mut s);
    assert_eq!(s.message, i18n::tr!(NoRemoteConfiguredSkipped));
    assert!(render(&mut s, 100, 28).contains("✓ repo"));
    assert!(s.repository_errors.is_empty());
    let records = s.service.store.tasks(10).unwrap();
    assert!(records.iter().all(|record| !record.pending()));
    let manual = records
        .iter()
        .find(|record| matches!(record.spec.retry, engine::tasks::Retry::Fetch { .. }))
        .unwrap();
    assert!(matches!(&manual.state,
        engine::tasks::State::Finished { outcome: engine::tasks::Outcome::Succeeded { detail }, .. }
        if detail.contains(i18n::tr!(NoRemoteConfiguredSkipped))
    ));
    s.shutdown();
}

#[test]
fn home_uses_the_checked_out_branch_and_the_branch_view_keeps_other_branch_differences() {
    let _language = i18n::using(i18n::Language::Chinese);
    let f = support::Fixture::create();
    support::git(
        &f.repo,
        &["checkout", "-b", "feature", "--track", "origin/main"],
    );
    for message in ["one", "two", "three"] {
        support::git(&f.repo, &["commit", "--allow-empty", "-m", message]);
    }
    support::git(&f.repo, &["checkout", "main"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    s.service.run(&Cancel::default(), |_| {}).unwrap();
    s.reload(&Cancel::default()).unwrap();
    let home = render(&mut s, 100, 28);
    assert!(
        home.lines()
            .any(|line| line.contains("✓ repo") && line.contains("↑0 ↓0")),
        "{home}"
    );
    let wide_home = render(&mut s, 180, 32);
    assert!(wide_home.contains("同步：↑0 ↓0"), "{wide_home}");
    assert!(wide_home.contains("分支：main"), "{wide_home}");
    assert!(
        !wide_home.contains("feature") && !wide_home.contains("分支同步：feature"),
        "{wide_home}"
    );
    command(&mut s, "repo branch");
    wait(&mut s);
    let branches = render(&mut s, 100, 28);
    assert!(
        branches
            .lines()
            .any(|line| line.contains("feature") && line.contains("↑3 ↓0")),
        "{branches}"
    );
    assert!(
        branches
            .lines()
            .any(|line| line.contains("* main") && line.contains("↑0 ↓0")),
        "{branches}"
    );
    command(&mut s, "repos");
    command(&mut s, "repo branch switch feature");
    wait(&mut s);
    assert!(matches!(
        s.mode,
        Mode::Confirm {
            operation: Confirmation::Mutation { .. },
            ..
        }
    ));
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    command(&mut s, "repos");
    let switched = render(&mut s, 100, 28);
    assert!(
        switched
            .lines()
            .any(|line| line.contains("! repo") && line.contains("↑3 ↓0")),
        "{switched}"
    );
    assert!(render(&mut s, 180, 32).contains("分支：feature"));
    s.shutdown();
}

#[test]
fn home_branch_is_in_selected_details_and_the_list_keeps_its_existing_columns() {
    let f = support::Fixture::create();
    support::git(&f.repo, &["remote", "remove", "origin"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    s.service.run_scheduled(&Cancel::default(), |_| {}).unwrap();
    s.reload(&Cancel::default()).unwrap();
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let branch = i18n::tf!(FieldLabel, label = i18n::tr!(Branch));
        for width in [120, 160, 200] {
            let text = render(&mut s, width, 30);
            assert!(text.contains(&format!("{branch}main")), "{width}\n{text}");
            let header = text
                .lines()
                .find(|line| {
                    line.contains(i18n::tr!(Worktree)) && line.contains(i18n::tr!(SyncColumn))
                })
                .unwrap();
            assert!(!header.contains(i18n::tr!(Branch)), "{header}");
        }
        for width in [60, 90, 100] {
            let text = render(&mut s, width, 30);
            assert!(!text.contains(&format!("{branch}main")), "{width}\n{text}");
            assert!(
                text.lines()
                    .any(|line| line.contains("✓ repo") && !line.contains("main")),
                "{width}\n{text}"
            );
        }
    }
    support::git(&f.repo, &["checkout", "--detach"]);
    s.service
        .check_repository(&f.repo, &Cancel::default(), |_| {})
        .unwrap();
    s.reload(&Cancel::default()).unwrap();
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let expected = format!(
            "{}{}",
            i18n::tf!(FieldLabel, label = i18n::tr!(Branch)),
            i18n::tr!(OverviewDetached)
        );
        assert!(render(&mut s, 160, 30).contains(&expected));
    }
    s.shutdown();
}

#[test]
fn home_attention_uses_current_branch_commits_and_actual_failures_take_priority() {
    let _language = i18n::using(i18n::Language::Chinese);
    let f = support::Fixture::create();
    support::git(&f.repo, &["branch", "experiment"]);
    support::git(
        &f.repo,
        &["branch", "--set-upstream-to=origin/main", "experiment"],
    );
    support::git(&f.repo, &["commit", "--allow-empty", "-m", "ahead"]);
    support::git(&f.repo, &["commit", "--allow-empty", "-m", "ahead two"]);
    support::git(&f.repo, &["commit", "--allow-empty", "-m", "ahead three"]);
    support::git(&f.repo, &["branch", "--force", "experiment", "main"]);
    support::write(f.repo.join("untracked"), "keep\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    s.service.run(&Cancel::default(), |_| {}).unwrap();
    s.reload(&Cancel::default()).unwrap();
    for (width, height) in [(60, 18), (100, 28), (180, 36)] {
        let text = render(&mut s, width, height);
        assert!(text.contains("正常 0 · 需关注 1 · 失败 0"), "{text}");
        assert!(
            text.lines()
                .any(|line| line.contains("! repo") && line.contains("↑3 ↓0")),
            "{text}"
        );
        assert!(
            !text.contains("已同步") && !text.contains("待同步"),
            "{text}"
        );
    }
    s.filter_repositories(Filter::Problems);
    assert_eq!(s.rows().len(), 1);
    s.filter_repositories(Filter::Clean);
    assert!(s.rows().is_empty());
    s.repository_errors
        .insert(f.repo.clone(), "actual operation failure".into());
    s.filter_repositories(Filter::Problems);
    assert_eq!(s.rows().len(), 1);
    assert!(render(&mut s, 100, 28).contains("× repo"));
    s.service.run(&Cancel::default(), |_| {}).unwrap();
    s.reload(&Cancel::default()).unwrap();
    assert!(render(&mut s, 100, 28).contains("! repo"));
    s.filter_repositories(Filter::Failed);
    assert!(s.rows().is_empty());
}

#[test]
fn home_colors_distinguish_unknown_data_and_failed_checks_from_independent_operation_errors() {
    use engine::domain::synchronization::{Checkout, Synchronization};
    use ratatui::style::Color;
    fn colors(screen: &mut Screen, symbol: &str, expected: (Color, Color, Color)) {
        let mut terminal = Terminal::new(TestBackend::new(100, 28)).unwrap();
        terminal.draw(|frame| view::render(frame, screen)).unwrap();
        let buffer = terminal.backend().buffer();
        let point = (0..28)
            .flat_map(|y| (0..100).map(move |x| (x, y)))
            .find(|point| buffer[*point].symbol() == symbol)
            .unwrap();
        let workspace = (0..100)
            .find(|x| buffer[(*x, point.1 - 1)].symbol() == "工")
            .unwrap();
        let sync = (0..100)
            .find(|x| buffer[(*x, point.1 - 1)].symbol() == "同")
            .unwrap();
        assert_eq!(buffer[point].fg, expected.0);
        assert_eq!(buffer[(workspace, point.1)].fg, expected.1);
        assert_eq!(buffer[(sync, point.1)].fg, expected.2);
        assert_eq!(buffer[(point.0 + 2, point.1)].fg, Color::Reset);
    }
    let _language = i18n::using(i18n::Language::Chinese);
    let path = "/example/repo";
    let (_home, mut s) = fixture(vec![path.into()]);
    update(&mut s, clean(path), "2026-10-28T04:00:00Z", 1);
    colors(
        &mut s,
        "✓",
        (Color::LightGreen, Color::DarkGray, Color::DarkGray),
    );
    update(&mut s, failed(path), "2026-10-28T04:01:00Z", 2);
    colors(
        &mut s,
        "×",
        (Color::LightRed, Color::LightRed, Color::LightRed),
    );
    let mut known = clean(path);
    let RepositoryResult::Clean {
        synchronization, ..
    } = &mut known
    else {
        unreachable!()
    };
    *synchronization = Synchronization::Current {
        remote_branches: None,
        checkout: Checkout::Compared {
            branch: "main".into(),
            ahead: 0,
            behind: 0,
        },
    };
    update(&mut s, known, "2026-10-28T04:02:00Z", 3);
    s.repository_errors
        .insert(path.into(), "a separate write failed".into());
    colors(
        &mut s,
        "×",
        (Color::LightRed, Color::DarkGray, Color::LightGreen),
    );
    s.repository_errors.clear();
    let mut modified = attention(path);
    let RepositoryResult::Attention {
        synchronization, ..
    } = &mut modified
    else {
        unreachable!()
    };
    *synchronization = Synchronization::Current {
        remote_branches: None,
        checkout: Checkout::Compared {
            branch: "main".into(),
            ahead: 3,
            behind: 1,
        },
    };
    update(&mut s, modified.clone(), "2026-10-28T04:03:00Z", 4);
    colors(&mut s, "!", (Color::Yellow, Color::Yellow, Color::Yellow));
    let RepositoryResult::Attention { issues, pushes, .. } = &mut modified else {
        unreachable!()
    };
    *issues = Issues::new(
        Issue {
            kind: IssueKind::Dirty,
            detail: "modified".into(),
        },
        vec![Issue {
            kind: IssueKind::PushFailed,
            detail: "rejected".into(),
        }],
    );
    let target = |branch: &str| PushTarget {
        branch: branch.into(),
        local_ref: format!("refs/heads/{branch}"),
        local_oid: "a".repeat(40),
        remote: "origin".into(),
        remote_ref: format!("refs/heads/{branch}"),
        remote_oid: engine::domain::RemoteTip::Existing("b".repeat(40)),
        ahead_commits: 1,
    };
    pushes.extend([
        Publication::Pushed {
            target: target("main"),
        },
        Publication::Failed {
            target: target("feature"),
            phase: engine::domain::Phase::Transfer,
            error: "rejected".into(),
        },
    ]);
    update(&mut s, modified, "2026-10-28T04:04:00Z", 5);
    colors(&mut s, "×", (Color::LightRed, Color::Yellow, Color::Yellow));
    let text = render(&mut s, 100, 28);
    assert!(
        text.contains("正常 0 · 需关注 0 · 失败 1") && !text.contains("部分失败"),
        "{text}"
    );
    for filter in [Filter::Failed, Filter::Problems] {
        s.filter_repositories(filter);
        assert_eq!(s.rows().len(), 1);
    }
}
