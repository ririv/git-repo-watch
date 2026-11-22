use super::*;

pub(super) fn wait_overview(s: &mut Screen) {
    let start = Instant::now();
    while matches!(
        s.overview.content,
        crate::overview::Content::Loading(_) | crate::overview::Content::Refreshing(_)
    ) {
        assert!(
            start.elapsed() < Duration::from_secs(8),
            "仓库概览读取没有结束"
        );
        s.overview.poll(&s.service);
        thread::sleep(Duration::from_millis(3));
    }
}

#[test]
fn overview_is_live_localized_and_distinct_from_cached_preview() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        f.commit("overview.ts", "export const overview = true;\n");
        support::write(
            f.repo.join("中文 pending.ts"),
            "export const pending = true;\n",
        );
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        let config = std::fs::read(s.service.store.config_path()).unwrap();
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        key(&mut s, KeyCode::Enter);
        assert_eq!(s.page, Page::Detail);
        assert!(matches!(s.mode, Mode::Browsing));
        assert!(matches!(
            s.overview.content,
            crate::overview::Content::Loading(_)
        ));
        assert!(render(&mut s, 120, 32).contains(i18n::tr!(OverviewLoading)));
        wait_overview(&mut s);
        wait_workspace(&mut s);
        assert!(matches!(
            s.overview.content,
            crate::overview::Content::Ready(_)
        ));
        let text = render(&mut s, 160, 65);
        for value in [
            i18n::tr!(OverviewUntracked),
            i18n::tr!(WorkspaceOverview),
            i18n::tr!(WorkspaceHistory),
            "origin/main",
        ] {
            assert!(text.contains(value), "{value}: {text}");
        }
        assert!(!text.contains(i18n::tr!(NoCompleteReportYet)));
        assert!(!text.contains(i18n::tr!(OverviewCheck)));
        assert!(!text.contains(i18n::tr!(CheckUpdatedAt)));
        assert!(text.contains("中文 pending.ts"), "{text}");
        let area = s
            .workspace_areas
            .iter()
            .find(|(_, section)| *section == crate::workspace::Section::Overview)
            .unwrap()
            .0;
        let overview = text
            .lines()
            .skip(area.y.into())
            .take(area.height.into())
            .map(|line| line.chars().take(area.width.into()).collect::<String>())
            .collect::<String>();
        assert!(!overview.contains("中文 pending.ts"));
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        assert_eq!(
            std::fs::read(s.service.store.config_path()).unwrap(),
            config
        );
        assert!(s.service.store.report().unwrap().is_none());
        assert!(s.service.store.repository_reports().unwrap().is_empty());
    }
}

#[test]
fn overview_and_branch_list_color_each_commit_count_independently() {
    use crate::workspace::Section;
    use ratatui::style::Color;

    let f = support::Fixture::create();
    f.commit("ahead.txt", "one local commit\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    key(&mut s, KeyCode::Enter);
    wait_overview(&mut s);
    wait_workspace(&mut s);

    let mut terminal = Terminal::new(TestBackend::new(180, 40)).unwrap();
    terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
    let buffer = terminal.backend().buffer();
    for section in [Section::Overview, Section::Branches] {
        let area = s
            .workspace_areas
            .iter()
            .find(|(_, pane)| *pane == section)
            .unwrap()
            .0;
        let mut counts = Vec::new();
        for y in area.y..area.bottom() {
            for x in area.x..area.right().saturating_sub(1) {
                let arrow = &buffer[(x, y)];
                if matches!(arrow.symbol(), "↑" | "↓") {
                    let number = &buffer[(x + 1, y)];
                    counts.push((arrow.symbol(), number.symbol(), arrow.fg, number.fg));
                }
            }
        }
        assert_eq!(
            counts,
            [
                ("↑", "1", Color::Yellow, Color::Yellow),
                ("↓", "0", Color::LightGreen, Color::LightGreen),
            ]
        );
    }
}

#[test]
fn compact_overview_keeps_navigation_visible_while_scrolling_and_resizing() {
    let f = support::Fixture::create();
    for n in 0..12 {
        support::git(&f.repo, &["branch", &format!("feature-{n}")]);
    }
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let cancel = Cancel::default();
    s.service.run(&cancel, |_| {}).unwrap();
    s.reload(&cancel).unwrap();
    key(&mut s, KeyCode::Right);
    wait_overview(&mut s);
    key(&mut s, KeyCode::Enter);
    let text = render(&mut s, 60, 18);
    assert!(text.contains("repos"));
    assert!(text.contains(i18n::tr!(Branch)));
    let mut seen = String::new();
    for _ in 0..20 {
        seen.push_str(&render(&mut s, 60, 18));
        key(&mut s, KeyCode::PageDown);
    }
    for label in [
        i18n::tr!(WorkspaceOverview),
        i18n::tr!(WorkspaceHistory),
        i18n::tr!(OverviewCheck),
    ] {
        assert!(seen.contains(label), "{label}: {seen}");
    }
    key(&mut s, KeyCode::End);
    render(&mut s, 160, 65);
    assert!(s.scroll < 100);
    key(&mut s, KeyCode::Home);
    assert_eq!(s.scroll, 0);
    assert!(render(&mut s, 110, 32).contains(i18n::tr!(CleanAlt)));
}

#[test]
fn refreshing_keeps_old_content_and_commands_return_to_updated_overview() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    key(&mut s, KeyCode::Enter);
    wait_overview(&mut s);
    support::write(f.repo.join("new-file"), "new\n");
    s.key(
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
        &Cancel::default(),
    )
    .unwrap();
    assert!(matches!(
        s.overview.content,
        crate::overview::Content::Refreshing(_)
    ));
    assert!(render(&mut s, 140, 40).contains(i18n::tr!(CleanAlt)));
    wait_overview(&mut s);
    assert_eq!(
        s.overview
            .content
            .snapshot()
            .unwrap()
            .changes
            .as_ref()
            .unwrap()
            .untracked,
        1
    );
    open_files(&mut s);
    wait(&mut s);
    assert!(matches!(s.mode, Mode::Files(_)));
    key(&mut s, KeyCode::Left);
    wait_overview(&mut s);
    assert!(matches!(s.mode, Mode::Browsing));
    assert_eq!(s.page, Page::Detail);
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.page, Page::Repositories);
    assert_eq!(s.selected_path(), Some(f.repo));
}

#[test]
fn replacing_a_request_never_displays_the_previous_repository_result() {
    let first = support::Fixture::create();
    let second = support::Fixture::create();
    let (_home, mut s) = fixture(vec![first.repo.clone(), second.repo.clone()]);
    s.overview.open(first.repo.clone(), &s.service);
    s.overview.open(second.repo.clone(), &s.service);
    wait_overview(&mut s);
    assert_eq!(s.overview.content.snapshot().unwrap().path, second.repo);
    s.overview.open("/missing-repository".into(), &s.service);
    wait_overview(&mut s);
    assert!(matches!(
        s.overview.content,
        crate::overview::Content::Failed { .. }
    ));
}

#[test]
fn full_check_errors_are_reachable_after_scrolling_and_resizing() {
    let _language = i18n::using(i18n::Language::English);
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    s.merge_check(
        f.repo.clone(),
        crate::checks::Update {
            revision: 0,
            result: RepositoryResult::Failed {
                path: f.repo.clone(),
                error: format!("{}last error detail", "remote error\n".repeat(500)),
                pushes: vec![],
            },
            finished_at: chrono::Utc::now().fixed_offset(),
        },
    );
    key(&mut s, KeyCode::Enter);
    wait_overview(&mut s);
    key(&mut s, KeyCode::Enter);
    for (width, height) in [(60, 18), (160, 32), (110, 18)] {
        key(&mut s, KeyCode::End);
        let text = render(&mut s, width, height);
        assert!(text.contains("last error detail"), "{text}");
        assert!(s.scroll < 600);
    }
    key(&mut s, KeyCode::Home);
    assert!(render(&mut s, 60, 18).contains("Branch:"));
}

#[test]
fn overview_only_shows_checks_recorded_for_the_open_repository() {
    let checked = support::Fixture::create();
    let added = support::Fixture::create();
    let (_home, mut s) = fixture(vec![checked.repo.clone()]);
    let cancel = Cancel::default();
    let report = s.service.run(&cancel, |_| {}).unwrap();
    s.service.add(std::slice::from_ref(&added.repo)).unwrap();
    s.reload(&cancel).unwrap();

    let (local, _) = engine::check::local_repository(&added.repo, &cancel);
    s.live = crate::live::State::start();
    let crate::live::State::Active(controller) = &mut s.live else {
        panic!("local observation controller");
    };
    controller.results.insert(added.repo.clone(), local);
    assert!(s.result_for(&added.repo).is_some());
    s.open_repository(added.repo.clone());
    wait_overview(&mut s);
    wait_workspace(&mut s);
    let overview = render(&mut s, 180, 45);
    assert!(
        overview.contains(i18n::tr!(RemoteRepositories)),
        "{overview}"
    );
    assert!(!overview.contains(i18n::tr!(OverviewCheck)), "{overview}");
    assert!(!overview.contains(i18n::tr!(CheckUpdatedAt)), "{overview}");

    support::write(added.repo.join("new.txt"), "local change\n");
    let files = s.service.files(&added.repo, &cancel).unwrap();
    s.service
        .edit_index(
            &files,
            engine::local::files::IndexAction::Stage,
            &files.changes,
            &cancel,
        )
        .unwrap();
    s.reload(&cancel).unwrap();
    assert!(s.stored_result_for(&added.repo).is_some());
    assert!(s.repository_check(&added.repo).is_none());
    assert!(!render(&mut s, 180, 45).contains(i18n::tr!(OverviewCheck)));

    s.open_repository(checked.repo.clone());
    wait_overview(&mut s);
    wait_workspace(&mut s);
    assert!(render(&mut s, 180, 45).contains(i18n::tr!(OverviewCheck)));
    let previous = s.repository_check(&checked.repo).unwrap();
    assert_eq!(
        previous.finished_at,
        chrono::DateTime::parse_from_rfc3339(&report.repository_finished_at[&checked.repo])
            .unwrap()
    );

    s.service
        .check_repository(&added.repo, &cancel, |_| {})
        .unwrap();
    s.reload(&cancel).unwrap();
    let finished = s.repository_check(&added.repo).unwrap().finished_at;
    let files = s.service.files(&added.repo, &cancel).unwrap();
    s.service
        .edit_index(
            &files,
            engine::local::files::IndexAction::Unstage,
            &files.changes,
            &cancel,
        )
        .unwrap();
    s.reload(&cancel).unwrap();
    assert_eq!(
        s.repository_check(&added.repo).unwrap().finished_at,
        finished
    );
    s.open_repository(added.repo);
    wait_overview(&mut s);
    wait_workspace(&mut s);
    let overview = render(&mut s, 180, 45);
    assert!(overview.contains(i18n::tr!(OverviewCheck)), "{overview}");
    assert!(overview.contains(i18n::tr!(CheckUpdatedAt)), "{overview}");
    s.shutdown();
}
