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

fn detail(s: &Screen) -> &crate::inspection::Session {
    let Mode::History(view) = &s.mode else {
        panic!("{:?}", s.mode)
    };
    let crate::history::View::Commit(commit) = view.as_ref() else {
        panic!("{view:?}")
    };
    &commit.inspect
}

fn source(s: &Screen) -> &engine::local::history::Detail {
    let crate::inspection::Source::Commit(source) = &detail(s).detail else {
        panic!("commit")
    };
    source
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
                i18n::tr!(SearchTargetCommits),
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
        key(&mut s, KeyCode::Esc);
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
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert_eq!(detail(&s).detail.oid(), selected);
        assert_eq!(source(&s).parents.len(), 2);
        assert_eq!(
            source(&s).changes[0].location.name(),
            std::path::Path::new("feature.ts")
        );
        for (width, height) in [(60, 18), (120, 30), (180, 36)] {
            let text = render(&mut s, width, height);
            assert!(text.contains(i18n::tr!(HistoryAuthor)));
            assert!(text.contains("feature.ts"));
        }
        key(&mut s, KeyCode::Enter);
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
            let crate::inspection::Source::Commit(source) = &commit.inspect.detail else {
                panic!("commit")
            };
            assert!(matches!(
                source.base,
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
        key(&mut s, KeyCode::Esc);
        command(&mut s, "repo log parent");
        wait(&mut s);
        let commit = detail(&s);
        let crate::inspection::Source::Commit(source) = &commit.detail else {
            panic!("commit")
        };
        assert_eq!(
            source.base,
            engine::local::history::Base::Commit(source.parents[1])
        );
        assert_eq!(
            source.changes[0].location.name(),
            std::path::Path::new("main.ts")
        );
        key(&mut s, KeyCode::Esc);
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
    key(&mut s, KeyCode::Down);
    assert_ne!(graph(&s).selected().unwrap().oid, first);
    key(&mut s, KeyCode::Up);
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
    let previous = graph(&s).state.selected();
    s.paste("no match anywhere");
    assert_eq!(graph(&s).state.selected(), previous);
    key(&mut s, KeyCode::Up);
    key(&mut s, KeyCode::Down);
    assert_eq!(graph(&s).state.selected(), previous);
    assert!(render(&mut s, 160, 32).contains(i18n::tr!(HistoryNoMatch)));
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Esc);
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
        s.search_commands()
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
    assert!(render(&mut s, 100, 28).contains(i18n::tr!(SearchFilter)));
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    key(&mut s, KeyCode::Esc);
    assert_eq!(detail(&s).query, "feature");
    assert_eq!(s.bar.text(), "feature");
    key(&mut s, KeyCode::Esc);
    assert!(detail(&s).query.is_empty());
    key(&mut s, KeyCode::Esc);
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
    assert!(source(&s).changes.is_empty());
    assert!(render(&mut s, 100, 28).contains(i18n::tr!(HistoryNoChanges)));
    key(&mut s, KeyCode::Esc);
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
    let Mode::Busy(pending) = &mut s.mode else {
        panic!()
    };
    let crate::loading::Feedback::Quiet { since, .. } = &pending.feedback else {
        panic!()
    };
    assert!(pending.tick(*since + crate::loading::FEEDBACK_DELAY));
    key(&mut s, KeyCode::Esc);
    wait(&mut s);
    assert!(matches!(s.mode, Mode::Error { .. }));
    key(&mut s, KeyCode::Enter);
    assert_eq!(graph(&s).selected().unwrap().oid, oid);
    assert!(s.service.store.report().unwrap().is_none());
}

fn preview(s: &mut Screen, width: u16, height: u16) -> String {
    render(s, width, height);
    let start = Instant::now();
    while matches!(
        s.history_preview.content,
        crate::history::preview::Content::Loading(_)
    ) {
        s.poll(&Cancel::default());
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "history preview did not finish"
        );
        thread::sleep(Duration::from_millis(2));
    }
    render(s, width, height)
}

#[test]
fn graph_preview_lists_first_parent_and_root_changes_and_clears_stale_files() {
    let f = merged();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    command(&mut s, "repo log");
    wait(&mut s);
    let text = preview(&mut s, 160, 36);
    assert!(text.contains("feature.ts"), "{text}");
    assert!(text.contains(&format!("{} · 1", i18n::tr!(HistoryChanges))));
    assert!(!text.contains("export const feature"));
    let crate::history::preview::Content::Ready {
        key: request,
        changes,
    } = &s.history_preview.content
    else {
        panic!()
    };
    assert_eq!(request.oid, graph(&s).selected().unwrap().oid);
    assert_eq!(changes[0].kind, engine::local::history::Kind::Added);
    key(&mut s, KeyCode::Down);
    let text = render(&mut s, 160, 36);
    let sidebar = text
        .lines()
        .filter_map(|line| line.split_once("││").map(|(_, right)| right))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !sidebar.contains("feature.ts"),
        "previous commit files survived selection: {text}"
    );
    let text = preview(&mut s, 160, 36);
    assert!(text.contains("main.ts"), "{text}");
    key(&mut s, KeyCode::End);
    let text = preview(&mut s, 160, 36);
    assert!(text.contains(i18n::tr!(HistoryNoChanges)), "{text}");
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert!(s.service.store.report().unwrap().is_none());
    render(&mut s, 80, 24);
    assert!(matches!(
        s.history_preview.content,
        crate::history::preview::Content::Hidden
    ));
    assert!(s.history_preview.area.is_none());
    assert!(preview(&mut s, 160, 36).contains(i18n::tr!(HistoryNoChanges)));
}

#[test]
fn graph_preview_has_independent_keyboard_and_wheel_scrolling() {
    let f = support::Fixture::create();
    for n in 0..40 {
        support::write(
            f.repo.join(format!("file-{n:02}.txt")),
            "body should not be previewed\n",
        );
    }
    support::git(&f.repo, &["add", "."]);
    support::git(&f.repo, &["commit", "-m", "many files"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    let text = preview(&mut s, 160, 36);
    assert!(
        text.contains("file-00.txt") && !text.contains("file-39.txt"),
        "{text}"
    );
    assert!(!text.contains("body should not be previewed"));
    let selected = graph(&s).state.selected();
    command(&mut s, "focus");
    render(&mut s, 160, 36);
    assert_eq!(s.focus_mode, crate::focus::State::Selecting);
    key(&mut s, KeyCode::Right);
    render(&mut s, 160, 36);
    key(&mut s, KeyCode::Enter);
    assert_eq!(graph(&s).focus, crate::history::GraphFocus::Files);
    key(&mut s, KeyCode::End);
    let text = render(&mut s, 160, 36);
    assert!(
        text.contains("file-39.txt") && !text.contains("file-00.txt"),
        "{text}"
    );
    assert_eq!(graph(&s).state.selected(), selected);
    let area = s.history_preview.area.unwrap();
    let offset = s.history_preview.offset;
    s.mouse(
        crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::ScrollUp,
            column: area.x + 1,
            row: area.y + 1,
            modifiers: KeyModifiers::NONE,
        },
        &Cancel::default(),
    )
    .unwrap();
    assert_eq!(s.history_preview.offset, offset - 3);
    assert_eq!(graph(&s).state.selected(), selected);
    key(&mut s, KeyCode::Left);
    assert_eq!(graph(&s).focus, crate::history::GraphFocus::Graph);
    key(&mut s, KeyCode::Down);
    assert_ne!(graph(&s).state.selected(), selected);
    preview(&mut s, 160, 36);
    command(&mut s, "focus");
    render(&mut s, 160, 36);
    assert_eq!(s.focus_mode, crate::focus::State::Selecting);
    key(&mut s, KeyCode::Right);
    render(&mut s, 160, 36);
    key(&mut s, KeyCode::Enter);
    render(&mut s, 80, 24);
    assert_eq!(graph(&s).focus, crate::history::GraphFocus::Graph);
}

#[test]
fn graph_preview_renders_deleted_modified_and_renamed_paths_with_trailing_markers() {
    let f = support::Fixture::create();
    f.commit("old-name.txt", "unchanged rename contents\n");
    f.commit("delete.txt", "delete\n");
    f.commit("modify.txt", "before\n");
    support::git(&f.repo, &["mv", "old-name.txt", "新名字.txt"]);
    support::git(&f.repo, &["rm", "delete.txt"]);
    support::write(f.repo.join("modify.txt"), "after\n");
    support::git(&f.repo, &["add", "."]);
    support::git(&f.repo, &["commit", "-m", "varied changes"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    let text = preview(&mut s, 160, 36);
    for (path, marker) in [
        ("delete.txt", 'D'),
        ("modify.txt", 'M'),
        ("新名字.txt", 'R'),
    ] {
        let line = text
            .lines()
            .filter_map(|line| line.split_once("││").map(|(_, right)| right))
            .find(|line| line.contains(path))
            .unwrap_or_else(|| panic!("{path}: {text}"));
        assert!(
            line.trim_end_matches('│').trim_end().ends_with(marker),
            "{line}"
        );
    }
    assert!(text.contains("old-name.txt → 新名字.txt"), "{text}");
}

#[test]
fn maximized_repository_history_focuses_file_preview_and_keeps_repository_navigation() {
    let f = merged();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    key(&mut s, KeyCode::Enter);
    wait_workspace(&mut s);
    s.switch_repository_section(crate::workspace::Section::History);
    command(&mut s, "zoom");
    assert_eq!(s.page, Page::Detail);
    assert!(preview(&mut s, 160, 36).contains("feature.ts"));
    command(&mut s, "focus");
    render(&mut s, 160, 36);
    key(&mut s, KeyCode::Right);
    key(&mut s, KeyCode::Enter);
    assert_eq!(graph(&s).focus, crate::history::GraphFocus::Files);
    let selected = graph(&s).state.selected();
    key(&mut s, KeyCode::Down);
    assert_eq!(graph(&s).state.selected(), selected);
    key(&mut s, KeyCode::Esc);
    assert_eq!(graph(&s).focus, crate::history::GraphFocus::Graph);
    command(&mut s, "focus");
    key(&mut s, KeyCode::Left);
    key(&mut s, KeyCode::Enter);
    assert!(matches!(s.mode, Mode::Files(_)));
    assert_eq!(s.page, Page::Detail);
}
