use super::*;

fn graph(s: &Screen) -> &crate::history::Session {
    let Mode::History(view) = &s.mode else {
        panic!("{:?}", s.mode)
    };
    let crate::history::View::Graph(session) = view.as_ref() else {
        panic!("{view:?}")
    };
    session
}

fn detail(s: &Screen) -> &crate::history::Commit {
    let Mode::History(view) = &s.mode else {
        panic!("{:?}", s.mode)
    };
    let crate::history::View::Commit(commit) = view.as_ref() else {
        panic!("{view:?}")
    };
    commit
}

fn merged() -> support::Fixture {
    let f = support::Fixture::create();
    f.commit("base.ts", "export const base = \"base\";\n");
    support::git(&f.repo, &["checkout", "-b", "feature"]);
    f.commit(
        "feature.ts",
        "export const feature = \"historical\"; // comment\n",
    );
    support::git(&f.repo, &["checkout", "main"]);
    f.commit("main.ts", "export const main = \"main\";\n");
    support::git(
        &f.repo,
        &["merge", "--no-ff", "-m", "merge feature", "feature"],
    );
    support::git(&f.repo, &["tag", "-a", "v1", "-m", "release"]);
    f
}

#[test]
fn graph_is_localized_colored_and_navigable_at_narrow_and_wide_sizes() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = merged();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        let config = std::fs::read(s.service.store.config_path()).unwrap();
        command(&mut s, "repo log");
        assert!(!s.repository_tasks.is_empty());
        assert!(!matches!(s.mode, Mode::Busy(_)));
        wait(&mut s);
        assert_eq!(graph(&s).records.len(), 5);
        for (width, height) in [(60, 18), (100, 28), (160, 36)] {
            let text = render(&mut s, width, height);
            for value in [
                i18n::tr!(HistoryTitle),
                i18n::tr!(HistorySearch),
                "HEAD→main",
                "tag:v1",
                "merge feature",
                "\u{10eeee}",
            ] {
                assert!(text.contains(value), "{value}: {text}");
            }
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
            let colors: BTreeSet<_> = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .filter(|cell| cell.symbol().contains('\u{10eeee}'))
                .map(|cell| format!("{:?}", cell.fg))
                .collect();
            assert!(colors.len() >= 2);
        }
        key(&mut s, KeyCode::End);
        assert_eq!(graph(&s).state.selected(), Some(4));
        key(&mut s, KeyCode::Up);
        assert_eq!(graph(&s).state.selected(), Some(3));
        key(&mut s, KeyCode::Home);
        key(&mut s, KeyCode::Down);
        assert_eq!(graph(&s).state.selected(), Some(1));
        key(&mut s, KeyCode::Left);
        assert!(matches!(s.mode, Mode::Browsing));
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        assert_eq!(
            std::fs::read(s.service.store.config_path()).unwrap(),
            config
        );
        assert!(s.service.store.report().unwrap().is_none());
    }
}

#[test]
fn merge_details_switch_parent_and_open_colored_historical_diff_without_worktree_changes() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = merged();
        support::write(
            f.repo.join("feature.ts"),
            "export const feature = \"current-index\";\n",
        );
        support::git(&f.repo, &["add", "feature.ts"]);
        support::write(
            f.repo.join("feature.ts"),
            "export const feature = \"current-worktree\";\n",
        );
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        command(&mut s, "repo log");
        wait(&mut s);
        let selected = graph(&s).selected().unwrap().oid;
        key(&mut s, KeyCode::Right);
        wait(&mut s);
        assert_eq!(detail(&s).detail.oid, selected);
        assert_eq!(detail(&s).detail.parents.len(), 2);
        assert_eq!(
            detail(&s).detail.changes[0].location.name(),
            std::path::Path::new("feature.ts")
        );
        for (width, height) in [(60, 18), (120, 30), (180, 36)] {
            let text = render(&mut s, width, height);
            assert!(text.contains(i18n::tr!(HistoryAuthor)));
            assert!(text.contains("feature.ts"));
        }
        key(&mut s, KeyCode::Right);
        wait(&mut s);
        for width in [120, 180] {
            let text = render(&mut s, width, 36);
            assert!(text.contains("TypeScript"), "{text}");
            assert!(text.contains("historical"), "{text}");
            assert!(!text.contains("current-index") && !text.contains("current-worktree"));
            let Mode::History(view) = &s.mode else {
                panic!()
            };
            let crate::history::View::Diff {
                content, commit, ..
            } = view.as_ref()
            else {
                panic!()
            };
            let labels = content.labels();
            assert_eq!(labels.1, selected.to_string()[..12]);
            assert!(matches!(
                commit.detail.base,
                engine::local::history::Base::Commit(_)
            ));
            let mut terminal = Terminal::new(TestBackend::new(width, 36)).unwrap();
            terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
            let colors: std::collections::HashSet<_> = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .filter(|cell| cell.bg == crate::diff::ADDED)
                .map(|cell| cell.fg)
                .collect();
            assert!(colors.len() >= 3);
        }
        command(&mut s, "repo diff layout unified");
        assert_eq!(s.diff_preference, crate::diff::Preference::Unified);
        command(&mut s, "repo diff");
        wait(&mut s);
        assert_eq!(s.page, Page::View(crate::navigation::View::Diff));
        assert!(matches!(s.mode, Mode::Changes(_)));
        key(&mut s, KeyCode::Esc);
        assert!(
            matches!(&s.mode, Mode::History(view) if matches!(view.as_ref(), crate::history::View::Diff {..}))
        );
        key(&mut s, KeyCode::Left);
        key(&mut s, KeyCode::Tab);
        wait(&mut s);
        let commit = detail(&s);
        assert_eq!(
            commit.detail.base,
            engine::local::history::Base::Commit(commit.detail.parents[1])
        );
        assert_eq!(
            commit.detail.changes[0].location.name(),
            std::path::Path::new("main.ts")
        );
        key(&mut s, KeyCode::Left);
        assert_eq!(graph(&s).selected().unwrap().oid, selected);
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        assert_eq!(
            std::fs::read_to_string(f.repo.join("feature.ts")).unwrap(),
            "export const feature = \"current-worktree\";\n"
        );
        assert!(s.service.store.report().unwrap().is_none());
    }
}

#[test]
fn search_locates_hash_subject_author_and_refs_without_hiding_graph_nodes() {
    let f = merged();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    let count = graph(&s).records.len();
    s.paste("feature");
    let first = graph(&s).selected().unwrap().oid;
    key(&mut s, KeyCode::Tab);
    assert_ne!(graph(&s).selected().unwrap().oid, first);
    key(&mut s, KeyCode::BackTab);
    assert_eq!(graph(&s).selected().unwrap().oid, first);
    assert_eq!(graph(&s).records.len(), count);
    key(&mut s, KeyCode::Esc);
    s.paste("v1");
    assert_eq!(graph(&s).state.selected(), Some(0));
    key(&mut s, KeyCode::Esc);
    let hash = graph(&s).records[2].entry.oid.to_string()[..8].to_owned();
    s.paste(&hash);
    assert_eq!(graph(&s).state.selected(), Some(2));
    key(&mut s, KeyCode::Esc);
    s.paste("no match anywhere");
    assert!(graph(&s).state.selected().is_none());
    assert!(render(&mut s, 160, 32).contains(i18n::tr!(HistoryNoMatch)));
    key(&mut s, KeyCode::Enter);
    assert!(matches!(s.mode, Mode::History(_)));
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Left);
    assert!(matches!(s.mode, Mode::Browsing));
}

#[test]
fn paging_keeps_routes_selection_and_origin_while_loading_then_refresh_starts_a_new_snapshot() {
    let f = support::Fixture::create();
    let repo = engine::local::open(&f.repo).unwrap();
    let initial = repo.head().unwrap().peel_to_commit().unwrap();
    let tree = initial.tree().unwrap();
    let signature = initial.author();
    let mut oid = initial.id();
    for number in 0..121 {
        let parent = repo.find_commit(oid).unwrap();
        oid = repo
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                &format!("history {number}"),
                &tree,
                &[&parent],
            )
            .unwrap();
    }
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    assert_eq!(graph(&s).records.len(), 100);
    let route = graph(&s).records[99].route.clone();
    let index = graph(&s).state.selected();
    key(&mut s, KeyCode::End);
    key(&mut s, KeyCode::Down);
    assert!(!s.repository_tasks.is_empty());
    assert!(!matches!(s.mode, Mode::Busy(_)));
    let loading = render(&mut s, 100, 28);
    assert!(loading.contains("history 21"));
    wait(&mut s);
    assert_eq!(graph(&s).records.len(), 122);
    assert_eq!(graph(&s).state.selected(), Some(99));
    assert_eq!(graph(&s).records[99].route, route);
    key(&mut s, KeyCode::Down);
    assert_eq!(graph(&s).state.selected(), Some(100));
    command(&mut s, "repo log more");
    assert!(s.message.contains(i18n::tr!(HistoryNoMore)));
    key(&mut s, KeyCode::Char('/'));
    key(&mut s, KeyCode::Char('/'));
    assert!(
        s.bar
            .commands()
            .unwrap()
            .iter()
            .all(|command| !command.name.contains(' '))
    );
    key(&mut s, KeyCode::Esc);
    s.key(
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
        &Cancel::default(),
    )
    .unwrap();
    wait(&mut s);
    assert_eq!(graph(&s).state.selected(), index);
    assert_eq!(graph(&s).records.len(), 100);
}

#[test]
fn historical_file_search_and_diff_back_navigation_preserve_the_graph() {
    let f = merged();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    let oid = graph(&s).selected().unwrap().oid;
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    s.paste("feature");
    assert_eq!(detail(&s).rows().len(), 1);
    assert!(render(&mut s, 100, 28).contains(i18n::tr!(SearchFiles)));
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    key(&mut s, KeyCode::Esc);
    assert!(
        matches!(&s.mode, Mode::History(view) if matches!(view.as_ref(), crate::history::View::Diff {..}))
    );
    key(&mut s, KeyCode::Left);
    assert!(detail(&s).query.is_empty());
    key(&mut s, KeyCode::Left);
    assert_eq!(graph(&s).selected().unwrap().oid, oid);
}

#[test]
fn empty_graph_and_root_without_changes_are_explicit_and_read_errors_restore_the_origin() {
    let temp = tempfile::tempdir().unwrap();
    support::init(temp.path());
    let path = std::fs::canonicalize(temp.path()).unwrap();
    let (_home, mut s) = fixture(vec![path]);
    command(&mut s, "repo log");
    wait(&mut s);
    assert!(graph(&s).records.is_empty());
    assert!(render(&mut s, 60, 18).contains(i18n::tr!(HistoryEmpty)));
    key(&mut s, KeyCode::Enter);
    assert!(matches!(s.mode, Mode::History(_)));
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(detail(&s).detail.changes.is_empty());
    assert!(render(&mut s, 100, 28).contains(i18n::tr!(HistoryNoChanges)));
    key(&mut s, KeyCode::Left);
    let oid = graph(&s).selected().unwrap().oid;
    s.start("delayed read", Effect::ReadOnly, |_, cancel, _| {
        while !cancel.is_cancelled() {
            thread::sleep(Duration::from_millis(5));
        }
        cancel.check()?;
        unreachable!()
    });
    let text = render(&mut s, 100, 28);
    assert!(text.contains(&oid.to_string()[..8]));
    thread::sleep(Duration::from_millis(110));
    assert!(s.animate_task());
    key(&mut s, KeyCode::Esc);
    wait(&mut s);
    assert!(matches!(s.mode, Mode::Error { .. }));
    key(&mut s, KeyCode::Enter);
    assert_eq!(graph(&s).selected().unwrap().oid, oid);
    assert!(s.service.store.report().unwrap().is_none());
}
