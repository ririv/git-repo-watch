use super::*;
use crate::search::{Mode as SearchMode, Support};

fn change_mode(s: &mut Screen, name: &str) {
    // 直接打开全局命令，不能先 Esc 清掉待保留的搜索词。
    s.paste(&format!("//search {name}"));
    key(s, KeyCode::Enter);
}

fn assert_matches_survive_row_selection(s: &mut Screen) {
    let mut terminal = Terminal::new(TestBackend::new(80, 26)).unwrap();
    terminal.draw(|frame| view::render(frame, s)).unwrap();
    let buffer = terminal.backend().buffer();
    let matches = buffer
        .content
        .iter()
        .filter(|cell| cell.symbol() == "中")
        .collect::<Vec<_>>();
    assert!(
        matches
            .iter()
            .any(|cell| cell.bg == ratatui::style::Color::Rgb(124, 83, 24))
    );
    assert!(
        matches
            .iter()
            .any(|cell| cell.bg == ratatui::style::Color::Rgb(81, 65, 37))
    );
    assert!(
        buffer
            .content
            .iter()
            .any(|cell| cell.bg == ratatui::style::Color::Rgb(25, 65, 95))
    );
}

#[test]
fn repositories_default_to_filter_and_locate_cycles_matches_without_hiding_rows() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let a = PathBuf::from("/example/中文 A");
        let b = PathBuf::from("/example/other");
        let c = PathBuf::from("/example/中文 B");
        let (_home, mut s) = fixture(vec![a.clone(), b.clone(), c.clone()]);
        assert_eq!(s.search_support(), Support::Both(SearchMode::Filter));
        s.paste("中文");
        assert_eq!(s.rows().len(), 2);
        assert_matches_survive_row_selection(&mut s);
        key(&mut s, KeyCode::Down);
        assert_eq!(s.selected_path(), Some(c.clone()));
        change_mode(&mut s, "locate");
        assert_eq!(s.query, "中文");
        assert_eq!(s.bar.text(), "中文");
        assert_eq!(s.rows().len(), 3);
        assert_eq!(s.selected_path(), Some(c.clone()));
        for (code, expected, progress) in [
            (KeyCode::Down, a.clone(), "1/2"),
            (KeyCode::Up, c.clone(), "2/2"),
            (KeyCode::Up, a.clone(), "1/2"),
        ] {
            key(&mut s, code);
            assert_eq!(s.selected_path(), Some(expected));
            for width in [35, 80, 160] {
                let text = render(&mut s, width, 26);
                assert!(text.contains(progress), "{text}");
                assert!(text.contains(i18n::tr!(SearchLocate)), "{text}");
            }
        }
        change_mode(&mut s, "filter");
        assert_eq!(s.rows().len(), 2);
        assert_eq!(s.selected_path(), Some(a));
        assert_eq!(s.bar.text(), "中文");
        change_mode(&mut s, "locate");
        key(&mut s, KeyCode::Esc);
        s.paste("no match");
        let selected = s.selected_path();
        assert_eq!(s.rows().len(), 3);
        assert!(s.search_title().contains("0/0"));
        key(&mut s, KeyCode::Up);
        key(&mut s, KeyCode::Down);
        assert_eq!(s.selected_path(), selected);
        key(&mut s, KeyCode::Esc);
        let index = s.selected();
        key(&mut s, KeyCode::Down);
        assert_eq!(s.selected(), (index + 1).min(2));
    }
}

#[test]
fn search_menu_marks_current_mode_and_keeps_status_filters_and_order() {
    let _language = i18n::using(i18n::Language::English);
    let (_home, mut s) = fixture(vec!["/example/beta".into(), "/example/alpha".into()]);
    s.sort_repositories(Order::Name);
    s.filter_repositories(Filter::Unchecked);
    s.paste("alpha");
    let selected = s.selected_path();
    let hint = "Press / to open the command menu";
    let search_title = s.search_title();
    let combined_title = format!("{search_title} ❘ {}", i18n::tr!(CommandsAlt).trim());
    assert!(render(&mut s, 40, 24).contains(hint));
    assert_eq!(s.bar.editor().cursor, "alpha".len());
    for exit_keys in [
        &[KeyCode::Esc][..],
        &[KeyCode::Backspace],
        &[KeyCode::Home, KeyCode::Delete],
    ] {
        key(&mut s, KeyCode::Char('/'));
        assert_eq!(s.bar.text(), "/");
        let preview = render(&mut s, 40, 24);
        assert!(preview.contains("› alpha ❘ /"), "{preview}");
        assert!(preview.contains(&combined_title), "{preview}");
        assert!(!s.command_items.is_empty());
        assert_eq!(s.selected_path(), selected);
        for &code in exit_keys {
            key(&mut s, code);
        }
        assert_eq!(s.bar.text(), "alpha");
        assert_eq!(s.query, "alpha");
        assert_eq!(s.selected_path(), selected);
        let preview = render(&mut s, 40, 24);
        assert!(preview.contains(&search_title), "{preview}");
        assert!(!preview.contains(&combined_title), "{preview}");
    }
    s.paste("//search");
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.bar.text(), "/search ");
    assert_eq!(s.search_commands().unwrap().len(), 1);
    assert!(
        matches!(&s.bar, bar::InputBar::Command { menu: command::completion::Menu::Options(values), .. } if values.options[values.selected].value.action() == Action::Search(SearchMode::Filter))
    );
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.search_mode, SearchMode::Locate);
    assert_eq!(s.query, "alpha");
    assert_eq!(s.order, Order::Name);
    assert_eq!(s.filter, Filter::Unchecked);
    assert_eq!(s.rows().len(), 2);
    assert!(render(&mut s, 40, 24).contains(hint));
    s.paste("//search");
    key(&mut s, KeyCode::Enter);
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.bar.text(), "alpha");
}

#[test]
fn worktree_locates_both_stages_keeps_checked_identities_and_survives_readback() {
    let f = support::Fixture::create();
    support::write(f.repo.join("中文 a.txt"), "staged\n");
    support::git(&f.repo, &["add", "."]);
    support::write(f.repo.join("中文 a.txt"), "unstaged\n");
    support::write(f.repo.join("other.txt"), "new\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    open_files(&mut s);
    s.paste("中文");
    assert_eq!(s.mode.file_session().unwrap().rows().len(), 2);
    assert_matches_survive_row_selection(&mut s);
    change_mode(&mut s, "locate");
    let files = s.mode.file_session().unwrap();
    let count = files.rows().len();
    assert_eq!(count, 3);
    let first = files.current().unwrap().area;
    key(&mut s, KeyCode::Insert);
    key(&mut s, KeyCode::Down);
    let files = s.mode.file_session().unwrap();
    assert_ne!(files.current().unwrap().area, first);
    assert!(s.search_title().contains("2/2"));
    key(&mut s, KeyCode::Insert);
    let before = s.mode.file_session().unwrap().selection();
    assert_eq!(before.len(), 1);
    let snapshot = s.service.files(&f.repo, &Cancel::default()).unwrap();
    s.merge_mutation_view(crate::mutations::readback::Readback::Files(snapshot));
    let files = s.mode.file_session().unwrap();
    assert_eq!(files.search_mode, SearchMode::Locate);
    assert_eq!(files.query, "中文");
    assert_eq!(files.selection(), before);
    assert_eq!(files.rows().len(), count);
    key(&mut s, KeyCode::Up);
    assert!(s.search_title().contains("1/2"));
    // Ctrl+A 只勾选当前组的可见文件，不被搜索导航占用。
    s.key(
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL),
        &Cancel::default(),
    )
    .unwrap();
    let files = s.mode.file_session().unwrap();
    assert_eq!(files.marked_changes().count(), 1);
    assert!(
        files
            .marked_changes()
            .all(|change| change.area == Area::Staged)
    );
}

#[test]
fn branches_and_stashes_locate_multiple_matches_and_details_do_not_search_hidden_lists() {
    let f = support::Fixture::create();
    for branch in ["topic/中文-a", "topic/中文-b", "other"] {
        support::git(&f.repo, &["branch", branch]);
    }
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo branch");
    wait(&mut s);
    s.paste("中文");
    change_mode(&mut s, "locate");
    let branch = |s: &Screen| {
        let Mode::Branches(view) = &s.mode else {
            panic!()
        };
        view.session().selected().unwrap().reference.clone()
    };
    let first = branch(&s);
    key(&mut s, KeyCode::Down);
    assert_ne!(branch(&s), first);
    key(&mut s, KeyCode::Down);
    assert_eq!(branch(&s), first);
    let snapshot = s.service.branches(&f.repo, &Cancel::default()).unwrap();
    s.merge_mutation_view(crate::mutations::readback::Readback::Branches(snapshot));
    assert_eq!(s.search_support(), Support::Both(SearchMode::Locate));
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.search_support(), Support::Unavailable);
    s.paste("hidden");
    assert!(s.bar.text().is_empty());
    assert!(
        matches!(&s.mode, Mode::Branches(view) if matches!(view.as_ref(), crate::branches::View::Detail { .. }))
    );
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.bar.text(), "中文");
    assert_eq!(branch(&s), first);

    for (index, message) in ["中文-a", "unrelated", "中文-b"].iter().enumerate() {
        support::write(f.repo.join("stash.txt"), &format!("{index}\n"));
        support::git(&f.repo, &["add", "."]);
        support::git(&f.repo, &["stash", "push", "-m", message]);
    }
    command(&mut s, "repo stash");
    wait(&mut s);
    s.paste("中文");
    change_mode(&mut s, "locate");
    let stash = |s: &Screen| {
        let Mode::Stashes(view) = &s.mode else {
            panic!()
        };
        view.selected().unwrap().oid
    };
    let first = stash(&s);
    key(&mut s, KeyCode::Down);
    assert_ne!(stash(&s), first);
    key(&mut s, KeyCode::Up);
    assert_eq!(stash(&s), first);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(s.search_support(), Support::Both(SearchMode::Filter));
    s.paste("stash.txt");
    assert_eq!(s.mode.inspection().unwrap().query, "stash.txt");
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.bar.text(), "中文");
    assert_eq!(s.search_support(), Support::Both(SearchMode::Locate));
}

#[test]
fn browser_and_graph_offer_only_supported_modes_while_static_pages_offer_none() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    for (entry, support, allowed) in [
        ("repo files", Support::Filter, SearchMode::Filter),
        ("repo log", Support::Locate, SearchMode::Locate),
    ] {
        command(&mut s, entry);
        wait(&mut s);
        assert_eq!(s.search_support(), support);
        s.paste("//search ");
        let commands = s.search_commands().unwrap();
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].action, Action::SearchMenu);
        assert!(
            matches!(&s.bar, bar::InputBar::Command { menu: command::completion::Menu::Options(values), .. } if values.options.len() == 1 && values.options[0].value.action() == Action::Search(allowed))
        );
        key(&mut s, KeyCode::Esc);
    }
    for entry in ["settings", "help", "schedule"] {
        command(&mut s, entry);
        let page = s.page;
        assert_eq!(s.search_support(), Support::Unavailable);
        s.paste("not a query");
        key(&mut s, KeyCode::Char('x'));
        assert_eq!(s.page, page);
        assert!(s.bar.text().is_empty());
        s.paste("//search");
        assert_eq!(s.search_commands().unwrap()[0].action, Action::SearchMenu);
        key(&mut s, KeyCode::Enter);
        assert_eq!(s.page, page);
        assert_eq!(s.message, i18n::tr!(SearchUnavailable));
        key(&mut s, KeyCode::Esc);
    }
}

#[test]
fn empty_filter_can_switch_to_locate_and_zero_row_lists_remain_navigable() {
    let (_home, mut s) = fixture(vec!["/example/alpha".into()]);
    s.paste("absent");
    assert!(s.rows().is_empty());
    change_mode(&mut s, "locate");
    assert_eq!(s.rows().len(), 1);
    assert!(s.search_title().contains("0/0"));
    change_mode(&mut s, "filter");
    assert!(s.rows().is_empty());
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.rows().len(), 1);
    let (_empty, mut s) = fixture(vec![]);
    change_mode(&mut s, "locate");
    s.paste("anything");
    key(&mut s, KeyCode::Up);
    key(&mut s, KeyCode::Down);
    assert_eq!(s.selected_path(), None);
    assert!(s.search_title().contains("0/0"));
}

#[test]
fn wheel_scrolls_rows_while_keyboard_arrows_locate_matches() {
    use crossterm::event::{MouseEvent, MouseEventKind};
    let paths = ["a-match", "b", "c", "d", "e-match", "f"]
        .map(|name| PathBuf::from(format!("/example/{name}")))
        .to_vec();
    let (_home, mut s) = fixture(paths);
    change_mode(&mut s, "locate");
    s.paste("match");
    render(&mut s, 100, 28);
    let area = s.repository_area.unwrap();
    s.mouse(
        MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: area.x + 3,
            row: area.y + 4,
            modifiers: KeyModifiers::NONE,
        },
        &Cancel::default(),
    )
    .unwrap();
    assert_eq!(s.selected(), 3);
    assert!(s.search_title().contains("—/2"));
    key(&mut s, KeyCode::Down);
    assert_eq!(s.selected(), 4);
    key(&mut s, KeyCode::Down);
    assert_eq!(s.selected(), 0);
}

#[test]
fn locating_files_does_not_take_arrows_from_focus_mode_or_the_diff_preview() {
    let f = support::Fixture::create();
    for name in ["a-match.txt", "b.txt", "c-match.txt"] {
        support::write(f.repo.join(name), "new\n");
    }
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    change_mode(&mut s, "locate");
    s.paste("match");
    render(&mut s, 160, 32);
    s.paste("//focus");
    key(&mut s, KeyCode::Enter);
    let cursor = s.mode.file_session().unwrap().cursor.clone();
    key(&mut s, KeyCode::Down);
    assert_eq!(s.mode.file_session().unwrap().cursor, cursor);
    key(&mut s, KeyCode::Right);
    key(&mut s, KeyCode::Enter);
    assert!(
        matches!(&s.mode, Mode::Changes(session) if session.focus == crate::changes::Focus::Preview)
    );
    key(&mut s, KeyCode::Down);
    assert_eq!(s.mode.file_session().unwrap().cursor, cursor);
    s.cycle_pane(false);
    key(&mut s, KeyCode::Down);
    assert_ne!(s.mode.file_session().unwrap().cursor, cursor);
    assert_eq!(s.bar.text(), "match");
    s.shutdown();
}

#[test]
fn historical_files_support_both_modes_and_diff_returns_to_the_original_query() {
    let f = support::Fixture::create();
    for name in ["a-match.txt", "b.txt", "c-match.txt"] {
        support::write(f.repo.join(name), "historical\n");
    }
    support::git(&f.repo, &["add", "."]);
    support::git(&f.repo, &["commit", "-m", "history match"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(s.search_support(), Support::Both(SearchMode::Filter));
    s.paste("match");
    change_mode(&mut s, "locate");
    key(&mut s, KeyCode::Down);
    assert!(s.search_title().contains("2/2"));
    let Mode::History(view) = &s.mode else {
        panic!()
    };
    let crate::history::View::Commit(commit) = view.as_ref() else {
        panic!()
    };
    assert_eq!(commit.inspect.rows().len(), 3);
    let selected = commit.inspect.files.selected();
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(s.search_support(), Support::Unavailable);
    assert!(s.bar.text().is_empty());
    s.paste("hidden query");
    assert!(
        matches!(&s.mode, Mode::History(view) if matches!(view.as_ref(), crate::history::View::Diff { .. }))
    );
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.search_support(), Support::Both(SearchMode::Locate));
    assert_eq!(s.bar.text(), "match");
    let Mode::History(view) = &s.mode else {
        panic!()
    };
    let crate::history::View::Commit(commit) = view.as_ref() else {
        panic!()
    };
    assert_eq!(commit.inspect.files.selected(), selected);
}

#[test]
fn changing_a_merge_parent_keeps_the_file_search_mode_and_query() {
    let f = support::Fixture::create();
    support::git(&f.repo, &["checkout", "-b", "feature"]);
    f.commit("feature.txt", "feature\n");
    support::git(&f.repo, &["checkout", "main"]);
    f.commit("main.txt", "main\n");
    support::git(
        &f.repo,
        &["merge", "--no-ff", "-m", "merge feature", "feature"],
    );
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    s.paste("txt");
    change_mode(&mut s, "locate");
    command(&mut s, "repo log parent");
    wait(&mut s);
    assert_eq!(s.search_support(), Support::Both(SearchMode::Locate));
    assert_eq!(s.bar.text(), "txt");
    assert!(s.search_title().contains("1/1"));
    let Mode::History(view) = &s.mode else {
        panic!()
    };
    let crate::history::View::Commit(commit) = view.as_ref() else {
        panic!()
    };
    assert_eq!(
        commit.inspect.rows()[0].location.name(),
        std::path::Path::new("main.txt")
    );
}
