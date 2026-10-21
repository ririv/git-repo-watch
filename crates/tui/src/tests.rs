use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use engine::{
    domain::*,
    scheduler::{Manager, Registration, Schedule},
    service::{Activity, CachedReport},
    state::Store,
};
use ratatui::backend::TestBackend;
use std::time::Instant;
use test_support as support;
fn screen(service: Service) -> Screen {
    let config = service.store.config().unwrap();
    let repository_viewport = ratatui::widgets::TableState::default()
        .with_selected((!config.repositories.is_empty()).then_some(0));
    Screen {
        snapshot: Snapshot {
            registration: Registration::Loaded,
            activity: Activity::Idle,
            configured_interval_seconds: config.interval_seconds,
            push_policy: config.push_policy,
            repository_count: config.repositories.len(),
            excluded_roots: config.excluded_roots.clone(),
            schedule: Schedule::Saved {
                interval_seconds: 21600,
            },
            report: CachedReport::Missing,
            max_concurrency: config.max_concurrency,
        },
        service: Arc::new(service),
        config,
        page: Page::Repositories,
        history: vec![],
        mode: Mode::Browsing,
        bar: crate::bar::InputBar::default(),
        repository_viewport,
        scroll_input: input::scroll::ScrollInput::default(),
        repository_area: None,
        body_area: None,
        filter: Filter::All,
        query: String::new(),
        order: Order::Priority,
        setting_selected: 0,
        scroll: 0,
        message: String::new(),
        check_task: None,
        updates: BTreeMap::new(),
    }
}
fn fixture(paths: Vec<PathBuf>) -> (tempfile::TempDir, Screen) {
    let temp = tempfile::tempdir().unwrap();
    let store = Store {
        home: temp.path().to_owned(),
    };
    store
        .save_config(&Config {
            interval_seconds: 21600,
            repositories: paths,
            excluded_roots: vec![],
            push_policy: PushPolicy::Disabled,
            max_concurrency: 4,
        })
        .unwrap();
    let manager = Manager::for_user(temp.path().to_owned(), unsafe { libc::getuid() }).unwrap();
    let service = Service::new(store.home, manager);
    (temp, screen(service))
}
fn key(s: &mut Screen, code: KeyCode) -> bool {
    s.key(KeyEvent::new(code, KeyModifiers::NONE), &Cancel::default())
        .unwrap()
}
fn command(s: &mut Screen, text: &str) {
    if !s.bar.editor().text.is_empty() {
        key(s, KeyCode::Esc);
    }
    s.paste(&format!("/{text}"));
    key(s, KeyCode::Enter);
}
fn wait(s: &mut Screen) {
    let started = Instant::now();
    while matches!(s.mode, Mode::Busy(_)) {
        assert!(
            started.elapsed() < Duration::from_secs(8),
            "工作线程没有结束"
        );
        s.poll(&Cancel::default());
        thread::sleep(Duration::from_millis(5));
    }
}
fn render(s: &mut Screen, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| view::render(frame, s)).unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .chunks(width as usize)
        .map(|row| {
            let mut skip = 0;
            let mut text = String::new();
            for cell in row {
                if skip > 0 {
                    skip -= 1;
                    continue;
                }
                let symbol = cell.symbol();
                text.push_str(symbol);
                skip = unicode_width::UnicodeWidthStr::width(symbol).saturating_sub(1);
            }
            text
        })
        .collect::<Vec<_>>()
        .join("\n")
}
#[test]
fn command_completion_unknown_commands_and_frozen_scope() {
    let _language = i18n::using(i18n::Language::Chinese);
    let (_temp, mut s) = fixture(vec!["/path/to/a".into(), "/path/to/b".into()]);
    key(&mut s, KeyCode::Char('/'));
    s.paste("sta");
    key(&mut s, KeyCode::Tab);
    assert!(matches!(&s.bar,crate::bar::InputBar::Command{editor,..}if editor.text=="/stage"));
    key(&mut s, KeyCode::Esc);
    command(&mut s, "check nope");
    assert!(matches!(s.bar, crate::bar::InputBar::Command { .. }));
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Char('/'));
    s.paste("remove");
    s.select(1);
    key(&mut s, KeyCode::Enter);
    assert!(
        matches!(&s.mode,Mode::Confirm{operation:Confirmation::Remove(p),..}if p.as_path()==std::path::Path::new("/path/to/a"))
    );
}
#[test]
fn unicode_editor_moves_and_deletes_at_character_boundaries() {
    let _language = i18n::using(i18n::Language::Chinese);
    let mut editor = Editor::default();
    editor.insert("中文abc");
    editor.key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
    editor.key(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));
    editor.key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
    assert_eq!(editor.text, "中abc");
    editor.key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
    assert_eq!(editor.text, "abc");
    editor.insert("新\n内容");
    assert_eq!(editor.text, "新内容abc");
    assert!(editor.text.is_char_boundary(editor.cursor));
}
#[test]
fn narrow_wide_command_files_diff_and_confirmation_remain_usable() {
    let _language = i18n::using(i18n::Language::Chinese);
    let (_temp, mut s) = fixture(vec!["/path/to/a".into()]);
    for size in [(60, 18), (80, 24), (150, 32)] {
        let text = render(&mut s, size.0, size.1);
        assert!(text.contains("搜索仓库"));
        assert!(!text.contains("1 / 1"));
        key(&mut s, KeyCode::Char('/'));
        let text = render(&mut s, size.0, size.1);
        assert!(text.contains("当前仓库"));
        assert!(text.contains("Tab 补全"));
        key(&mut s, KeyCode::Esc);
        command(&mut s, "remove");
        let text = render(&mut s, size.0, size.1);
        assert!(text.contains("移除"));
        assert!(text.contains("Enter 确认"));
        key(&mut s, KeyCode::Esc);
    }
}
#[test]
fn file_selection_commit_and_push_through_command_bar() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        support::write(f.repo.join("one.txt"), "one\n");
        support::write(f.repo.join("two.txt"), "two\n");
        let (_temp, mut s) = fixture(vec![f.repo.clone()]);
        command(&mut s, "stage");
        wait(&mut s);
        assert!(matches!(s.mode, Mode::Files(_)));
        key(&mut s, KeyCode::Char(' '));
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert!(matches!(&s.mode,Mode::Diff{content,..}if content.contains("+one")));
        key(&mut s, KeyCode::Esc);
        key(&mut s, KeyCode::Char('/'));
        key(&mut s, KeyCode::Esc);
        assert!(matches!(&s.mode,Mode::Files(session)if session.selected.len()==1));
        command(&mut s, "stage");
        wait(&mut s);
        assert_eq!(
            support::git(&f.repo, &["diff", "--cached", "--name-only"]),
            "one.txt"
        );
        key(&mut s, KeyCode::Tab);
        command(&mut s, "unstage");
        wait(&mut s);
        assert!(support::git(&f.repo, &["diff", "--cached"]).is_empty());
        key(&mut s, KeyCode::Tab);
        command(&mut s, "stage");
        wait(&mut s);
        command(&mut s, "commit");
        wait(&mut s);
        assert!(matches!(s.mode, Mode::CommitMessage { .. }));
        s.paste("通过命令栏提交");
        key(&mut s, KeyCode::Enter);
        assert!(matches!(s.mode, Mode::CommitConfirm { .. }));
        let before = support::git(&f.repo, &["rev-parse", "HEAD"]);
        key(&mut s, KeyCode::Char('y'));
        assert!(matches!(s.mode, Mode::CommitConfirm { .. }));
        key(&mut s, KeyCode::Right);
        key(&mut s, KeyCode::Enter);
        assert!(
            matches!(&s.mode, Mode::CommitMessage { editor, .. } if editor.text == "通过命令栏提交")
        );
        key(&mut s, KeyCode::Enter);
        key(&mut s, KeyCode::Esc);
        assert!(
            matches!(&s.mode, Mode::CommitMessage { editor, .. } if editor.text == "通过命令栏提交")
        );
        key(&mut s, KeyCode::Enter);
        key(&mut s, KeyCode::Tab);
        key(&mut s, KeyCode::Left);
        for size in [(60, 18), (80, 24), (150, 32)] {
            let text = render(&mut s, size.0, size.1);
            assert!(text.contains(i18n::tr!(ConfirmCommit)));
            assert!(text.contains(if language == i18n::Language::English {
                "Enter confirm"
            } else {
                "Enter 确认"
            }));
        }
        assert_eq!(before, support::git(&f.repo, &["rev-parse", "HEAD"]));
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert!(s.message.contains(if language == i18n::Language::English {
            "Committed"
        } else {
            "已提交"
        }));
        assert!(support::git(&f.repo, &["status", "--porcelain"]).contains("two.txt"));
        command(&mut s, "push");
        wait(&mut s);
        assert!(matches!(s.mode, Mode::PushConfirm { .. }));
        let remote_before = support::git(&f.remote, &["rev-parse", "main"]);
        key(&mut s, KeyCode::Tab);
        key(&mut s, KeyCode::Enter);
        assert!(matches!(s.mode, Mode::Browsing));
        assert_eq!(
            remote_before,
            support::git(&f.remote, &["rev-parse", "main"])
        );
        command(&mut s, "push");
        wait(&mut s);
        key(&mut s, KeyCode::Right);
        key(&mut s, KeyCode::BackTab);
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert!(s.message.contains(if language == i18n::Language::English {
            "Pushed"
        } else {
            "已推送"
        }));
        assert_eq!(
            support::git(&f.repo, &["rev-parse", "HEAD"]),
            support::git(&f.remote, &["rev-parse", "main"])
        );
    }
}

#[test]
fn default_browsing_and_commands_do_not_open_repositories() {
    let _language = i18n::using(i18n::Language::Chinese);
    let (_temp, mut s) = fixture(vec!["/path/to/nonexistent/repository".into()]);
    for size in [(60, 18), (150, 32)] {
        assert!(render(&mut s, size.0, size.1).contains("repository"));
    }
    command(&mut s, "stage");
    wait(&mut s);
    assert!(matches!(s.mode, Mode::Error { .. }));
    let text = render(&mut s, 60, 18);
    assert!(text.contains("操作结果"));
    assert!(text.contains("Enter/Esc 返回"));
}

#[test]
fn persistent_search_accepts_shortcut_letters_and_preserves_navigation() {
    let _language = i18n::using(i18n::Language::Chinese);
    let (_temp, mut s) = fixture(vec![
        "/path/to/alpha".into(),
        "/path/to/alpha-two".into(),
        "/path/to/quit".into(),
    ]);
    for c in "alpha".chars() {
        assert!(!key(&mut s, KeyCode::Char(c)));
    }
    assert_eq!(s.rows().len(), 2);
    key(&mut s, KeyCode::Down);
    assert_eq!(s.selected(), 1);
    key(&mut s, KeyCode::Left);
    assert_eq!(s.selected(), 1);
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.page, Page::Detail);
    assert_eq!(s.query, "alpha");
    key(&mut s, KeyCode::Esc);
    assert!(s.query.is_empty());
    assert_eq!(s.page, Page::Repositories);
    assert_eq!(s.rows().len(), 3);
    for c in "quit".chars() {
        assert!(!key(&mut s, KeyCode::Char(c)));
    }
    assert_eq!(s.query, "quit");
    assert!(matches!(s.mode, Mode::Browsing));
    key(&mut s, KeyCode::Esc);
    assert!(!key(&mut s, KeyCode::Char('/')));
    s.paste("quit");
    assert!(key(&mut s, KeyCode::Enter));
}
#[test]
fn pasted_commands_are_explicit_and_prefix_deletion_returns_to_search() {
    let _language = i18n::using(i18n::Language::Chinese);
    let (_temp, mut s) = fixture(vec!["/path/to/settings".into()]);
    s.paste("settings");
    assert_eq!(s.query, "settings");
    assert_eq!(s.page, Page::Repositories);
    key(&mut s, KeyCode::Esc);
    s.paste("/settings");
    assert_eq!(s.page, Page::Repositories);
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.page, Page::Settings);
    s.paste("/push");
    key(&mut s, KeyCode::Home);
    key(&mut s, KeyCode::Delete);
    assert!(matches!(s.bar, crate::bar::InputBar::Search(_)));
    assert_eq!(s.query, "push");
    assert_eq!(s.page, Page::Repositories);
}
#[test]
fn full_width_command_panel_keeps_list_and_input_positions_stable() {
    let _language = i18n::using(i18n::Language::Chinese);
    let (_temp, mut s) = fixture(vec!["/path/to/alpha".into()]);
    for (w, h) in [(60, 18), (100, 28), (150, 32)] {
        let before = render(&mut s, w, h);
        let before_lines = before.lines().collect::<Vec<_>>();
        let list_line = before_lines
            .iter()
            .position(|line| line.contains("列表 ·"))
            .unwrap();
        let input_line = before_lines
            .iter()
            .position(|line| line.contains("输入关键词搜索"))
            .unwrap();
        key(&mut s, KeyCode::Char('/'));
        let during = render(&mut s, w, h);
        let during_lines = during.lines().collect::<Vec<_>>();
        assert!(during_lines[list_line].contains("列表 ·"));
        assert!(during_lines[input_line].contains("› /"));
        assert!(during.contains("当前仓库"));
        let top = during_lines
            .iter()
            .position(|line| line.contains("命令 · alpha"))
            .unwrap();
        assert!(during_lines[top].starts_with('┌'));
        assert!(during_lines[top].ends_with('┐'));
        assert_eq!(
            unicode_width::UnicodeWidthStr::width(during_lines[top]),
            w as usize
        );
        assert!(during_lines[top + 1].starts_with('│'));
        assert!(during_lines[top + 1].ends_with('│'));
        for _ in 0..command::COMMANDS.len() {
            key(&mut s, KeyCode::Down);
        }
        let scrolled = render(&mut s, w, h);
        assert!(scrolled.contains("/quit"));
        s.paste("nonexistent");
        assert!(render(&mut s, w, h).contains("没有匹配命令"));

        key(&mut s, KeyCode::Esc);
    }
}
#[test]
fn filtered_file_selection_uses_snapshot_identity() {
    let _language = i18n::using(i18n::Language::Chinese);
    let f = support::Fixture::create();
    support::write(f.repo.join("one.txt"), "one");
    support::write(f.repo.join("two.txt"), "two");
    let (_temp, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "stage");
    wait(&mut s);
    s.paste("two");
    assert!(matches!(&s.mode,Mode::Files(session)if session.rows().len()==1));
    key(&mut s, KeyCode::Insert);
    key(&mut s, KeyCode::Esc);
    assert!(
        matches!(&s.mode,Mode::Files(session)if session.rows().len()==2&&session.selected.len()==1)
    );
    command(&mut s, "stage");
    wait(&mut s);
    assert_eq!(
        support::git(&f.repo, &["diff", "--cached", "--name-only"]),
        "two.txt"
    );
}

#[test]
fn numeric_input_and_control_key_filters_do_not_conflict() {
    let _language = i18n::using(i18n::Language::Chinese);
    let (_temp, mut s) = fixture(vec!["/path/to/1234".into()]);
    for c in "1234".chars() {
        key(&mut s, KeyCode::Char(c));
    }
    assert_eq!(s.query, "1234");
    assert_eq!(s.filter, Filter::All);
    assert_eq!(s.rows().len(), 1);
    for expected in [
        Filter::Problems,
        Filter::WithoutUpstream,
        Filter::Pushes,
        Filter::All,
    ] {
        s.key(
            KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
            &Cancel::default(),
        )
        .unwrap();
        assert_eq!(s.filter, expected);
        assert_eq!(s.query, "1234");
    }
    key(&mut s, KeyCode::Esc);
    s.paste("/");
    s.key(
        KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
        &Cancel::default(),
    )
    .unwrap();
    assert_eq!(s.filter, Filter::All);
    assert_eq!(s.bar.editor().text, "/");
}

#[test]
fn input_border_placeholder_text_and_cursor_are_distinct() {
    let _language = i18n::using(i18n::Language::Chinese);
    use ratatui::style::{Color, Modifier};
    let (_temp, mut s) = fixture(vec!["/path/to/alpha".into()]);
    for (width, height) in [(60, 18), (80, 24), (150, 32)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
        assert!(terminal.backend().cursor_visible());
        let cursor = terminal.backend().cursor_position();
        assert_eq!(cursor, (3, height - 3).into());
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(0, height - 4)].symbol(), "╭");
        assert_eq!(buffer[(width - 1, height - 2)].symbol(), "╯");
        let placeholder = &buffer[(3, height - 3)];
        assert_eq!(placeholder.fg, Color::DarkGray);
        assert!(placeholder.modifier.contains(Modifier::ITALIC));
        s.paste("中文1234");
        terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
        assert_eq!(
            terminal.backend().cursor_position(),
            (11, height - 3).into()
        );
        let text = &terminal.backend().buffer()[(3, height - 3)];
        assert_eq!(text.fg, Color::Reset);
        assert!(!text.modifier.contains(Modifier::ITALIC));
        key(&mut s, KeyCode::Esc);
        command(&mut s, "concurrency");
        key(&mut s, KeyCode::Enter);
        terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
        assert!(matches!(s.mode, Mode::Confirm { .. }));
        assert!(!terminal.backend().cursor_visible());
        key(&mut s, KeyCode::Esc);
        key(&mut s, KeyCode::Esc);
    }
}

#[test]
fn input_viewport_keeps_unicode_cursor_inside_box() {
    let _language = i18n::using(i18n::Language::Chinese);
    let mut editor = Editor::new("很长的中文内容1234567890".into());
    for code in [KeyCode::End, KeyCode::Home, KeyCode::Right, KeyCode::Right] {
        editor.key(KeyEvent::new(code, KeyModifiers::NONE));
        let (text, cursor) = editor.viewport(8);
        assert!(cursor < 8);
        assert!(unicode_width::UnicodeWidthStr::width(text.as_str()) <= 8);
    }
}

#[test]
fn wide_detail_title_tracks_selected_repository_name() {
    let _language = i18n::using(i18n::Language::Chinese);
    let (_temp, mut s) = fixture(vec!["/example/alpha".into(), "/example/beta".into()]);
    let first = render(&mut s, 150, 32);
    assert!(first.lines().any(|line| line.contains("┌alpha")));
    assert!(first.contains("路径：/example/alpha"));
    assert!(first.contains("状态：未检查"));
    assert!(!first.contains("选中仓库"));
    key(&mut s, KeyCode::Down);
    let second = render(&mut s, 150, 32);
    assert!(second.lines().any(|line| line.contains("┌beta")));
    assert!(second.contains("路径：/example/beta"));
    s.paste("nonexistent");
    let empty = render(&mut s, 150, 32);
    assert!(empty.contains("无选中仓库"));
    assert!(!empty.contains("路径：/example/beta"));
}

#[test]
fn english_views_and_commands_remain_usable_at_supported_sizes() {
    let _language = i18n::using(i18n::Language::English);
    let (_temp, mut s) = fixture(vec!["/example/alpha".into()]);
    for (width, height) in [(60, 18), (80, 24), (150, 32)] {
        let text = render(&mut s, width, height);
        assert!(text.contains("Search repositories"));
        assert!(text.contains("Type to search"));
        assert!(text.contains("No up"));
        assert!(!text.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)));
        s.paste("/");
        let candidates = render(&mut s, width, height);
        assert!(candidates.contains("Current repo"));
        assert!(candidates.contains("Tab complete"));
        assert!(candidates.contains("/stage"));
        key(&mut s, KeyCode::Esc);
        command(&mut s, "concurrency");
        let input = render(&mut s, width, height);
        assert!(input.contains("Configuration input"));
        key(&mut s, KeyCode::Enter);
        let confirm = render(&mut s, width, height);
        assert!(confirm.contains("Enter confirm"));
        key(&mut s, KeyCode::Esc);
        assert!(matches!(s.mode, Mode::Input { .. }));
        key(&mut s, KeyCode::Esc);
    }
    key(&mut s, KeyCode::Enter);
    let detail = render(&mut s, 80, 24);
    assert!(detail.contains("Path: /example/alpha"));
    assert!(detail.contains("Status: Unchecked"));
    assert!(
        !detail
            .chars()
            .any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c))
    );
}

#[test]
fn english_worker_errors_follow_the_ui_language() {
    let _language = i18n::using(i18n::Language::English);
    let (_temp, mut s) = fixture(vec!["/example/unavailable".into()]);
    command(&mut s, "stage");
    wait(&mut s);
    let text = render(&mut s, 80, 24);
    assert!(text.contains("Operation result"));
    assert!(text.contains("Enter/Esc back"));
    assert!(!text.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)));
}

#[test]
fn terminal_command_completes_and_freezes_repository_scope() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_temp, mut s) = fixture(vec![
            "/example/unavailable-alpha".into(),
            "/example/unavailable-beta".into(),
        ]);
        s.paste("/te");
        key(&mut s, KeyCode::Tab);
        assert_eq!(s.bar.editor().text, "/terminal");
        assert_eq!(command::exact("terminal"), Some(Action::Terminal));
        assert!(matches!(s.mode, Mode::Browsing));
        s.select(1);
        assert!(
            matches!(&s.bar, crate::bar::InputBar::Command { path: Some(path), .. } if path == &PathBuf::from("/example/unavailable-alpha"))
        );
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert!(matches!(s.mode, Mode::Error { .. }));
        let text = render(&mut s, 80, 24);
        assert!(text.contains(i18n::tr!(TerminalDirectoryMissing)));
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.selected(), 1);
    }
}

#[test]
fn terminal_requires_selection_and_preserves_origin_on_success() {
    let _language = i18n::using(i18n::Language::English);
    let (_temp, mut empty) = fixture(vec![]);
    command(&mut empty, "terminal");
    assert!(matches!(empty.mode, Mode::Browsing));
    assert_eq!(empty.message, "Select a repository first");
    let f = support::Fixture::create();
    let (_temp, mut s) = fixture(vec![f.repo.clone()]);
    let origin = Mode::Files(FileSession {
        snapshot: s.service.files(&f.repo, &Cancel::default()).unwrap(),
        area: FileArea::Index,
        cursor: 0,
        selected: BTreeSet::new(),
        query: "filter".into(),
    });
    s.finish(
        Outcome::TerminalOpened(f.repo.clone()),
        origin,
        &Cancel::default(),
    );
    assert!(
        matches!(&s.mode, Mode::Files(session) if session.area == FileArea::Index && session.query == "filter")
    );
    assert!(s.message.starts_with("Opened terminal:"));
    assert_eq!(s.bar.editor().text, "filter");
    assert!(!s.service.store.state_dir().join("latest.json").exists());
}

#[test]
fn command_panel_shows_eight_candidates_and_scrolls_to_remaining_commands() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_temp, mut s) = fixture(vec!["/example/alpha".into()]);
        for (width, height) in [(60, 24), (100, 28), (150, 32)] {
            s.paste("/");
            let text = render(&mut s, width, height);
            let visible = |text: &str| {
                text.lines()
                    .filter(|line| {
                        command::COMMANDS
                            .iter()
                            .any(|item| line.contains(&format!("/{:<13}", item.name)))
                    })
                    .count()
            };
            assert_eq!(visible(&text), 8, "{text}");
            for item in command::COMMANDS.iter().take(8) {
                assert!(text.contains(&format!("/{:<13}", item.name)));
            }
            for _ in 0..command::COMMANDS.len() {
                key(&mut s, KeyCode::Down);
            }
            let last = render(&mut s, width, height);
            assert_eq!(visible(&last), 8);
            assert!(last.contains("/quit"));
            key(&mut s, KeyCode::Esc);
        }
    }
}

#[test]
fn repository_table_shows_worktree_and_combined_sync_in_both_languages() {
    for (language, worktree, sync, removed) in [
        (
            i18n::Language::English,
            "Worktree",
            "Sync",
            ["Remote", "No up", "Pushes"],
        ),
        (
            i18n::Language::Chinese,
            "工作区",
            "同步",
            ["远程", "无上游", "推送"],
        ),
    ] {
        let _language = i18n::using(language);
        let (_temp, mut screen) = fixture(vec!["/example/repository".into()]);
        for width in [60, 90, 130, 180] {
            let output = render(&mut screen, width, 30);
            let header = output
                .lines()
                .find(|line| {
                    line.contains(i18n::tr!(Repositories))
                        && line.contains(worktree)
                        && line.contains(sync)
                })
                .unwrap();
            for label in removed {
                assert!(!header.contains(label), "{header}");
            }
        }
    }
}

fn wheel(s: &mut Screen, kind: crossterm::event::MouseEventKind, column: u16, row: u16) -> bool {
    s.mouse(
        crossterm::event::MouseEvent {
            kind,
            column,
            row,
            modifiers: KeyModifiers::NONE,
        },
        &Cancel::default(),
    )
    .unwrap()
}
#[test]
fn repository_viewport_stays_in_place_when_moving_up_and_handles_resizing_and_search() {
    let paths = (0..80)
        .map(|i| PathBuf::from(format!("/example/repository{i:03}")))
        .collect();
    let (_temp, mut s) = fixture(paths);
    for (width, height) in [(60, 18), (90, 24), (140, 30)] {
        for _ in 0..8 {
            key(&mut s, KeyCode::PageDown);
        }
        let output = render(&mut s, width, height);
        assert!(output.contains("repository079"));
        let offset = s.repository_viewport.offset();
        assert!(offset > 0);
        key(&mut s, KeyCode::Up);
        let output = render(&mut s, width, height);
        assert!(output.contains("repository078"));
        assert_eq!(s.repository_viewport.offset(), offset);
        for _ in 0..8 {
            key(&mut s, KeyCode::PageUp);
        }
        assert!(render(&mut s, width, height).contains("repository000"));
        assert_eq!(s.repository_viewport.offset(), 0);
    }
    for _ in 0..8 {
        key(&mut s, KeyCode::PageDown);
    }
    render(&mut s, 90, 24);
    for c in "repository002".chars() {
        key(&mut s, KeyCode::Char(c));
    }
    let output = render(&mut s, 90, 24);
    assert!(output.contains("repository002"));
    assert_eq!(s.selected(), 0);
    assert_eq!(s.repository_viewport.offset(), 0);
}
#[test]
fn wheel_scrolls_main_list_with_bounds_and_respects_input_and_preview_regions() {
    use crossterm::event::MouseEventKind::{ScrollDown, ScrollUp};
    let paths = (0..50)
        .map(|i| PathBuf::from(format!("/example/repository{i:03}")))
        .collect();
    let (_temp, mut s) = fixture(paths);
    render(&mut s, 140, 30);
    let area = s.repository_area.unwrap();
    let (x, y) = (area.x + 2, area.y + 3);
    assert!(wheel(&mut s, ScrollDown, x, y));
    assert_eq!(s.selected(), 3);
    assert!(!wheel(&mut s, ScrollDown, area.right() + 2, y));
    assert!(!wheel(&mut s, ScrollDown, x, 29));
    assert_eq!(s.selected(), 3);
    for _ in 0..30 {
        wheel(&mut s, ScrollDown, x, y);
    }
    assert_eq!(s.selected(), 49);
    assert!(render(&mut s, 140, 30).contains("repository049"));
    let offset = s.repository_viewport.offset();
    wheel(&mut s, ScrollUp, x, y);
    assert_eq!(s.selected(), 46);
    render(&mut s, 140, 30);
    assert_eq!(s.repository_viewport.offset(), offset);
    for _ in 0..30 {
        wheel(&mut s, ScrollUp, x, y);
    }
    assert_eq!(s.selected(), 0);
    render(&mut s, 20, 10);
    assert!(!wheel(&mut s, ScrollDown, x, y));
    assert_eq!(s.selected(), 0);
}
#[test]
fn wheel_keeps_command_scope_frozen_and_does_not_change_confirmation_or_empty_lists() {
    use crossterm::event::MouseEventKind::ScrollDown;
    let (_temp, mut s) = fixture(vec!["/example/alpha".into(), "/example/beta".into()]);
    render(&mut s, 90, 24);
    let area = s.repository_area.unwrap();
    key(&mut s, KeyCode::Char('/'));
    render(&mut s, 90, 24);
    assert!(wheel(&mut s, ScrollDown, area.x + 2, area.y + 3));
    assert_eq!(s.selected(), 0);
    match &s.bar {
        crate::bar::InputBar::Command { path, selected, .. } => {
            assert_eq!(
                path.as_deref(),
                Some(std::path::Path::new("/example/alpha"))
            );
            assert_eq!(*selected, 3);
        }
        _ => panic!("command mode expected"),
    }
    key(&mut s, KeyCode::Esc);
    command(&mut s, "remove");
    render(&mut s, 90, 24);
    assert!(!wheel(&mut s, ScrollDown, area.x + 2, area.y + 3));
    assert_eq!(s.selected(), 0);
    key(&mut s, KeyCode::Esc);
    s.config.repositories.clear();
    render(&mut s, 90, 24);
    wheel(&mut s, ScrollDown, area.x + 2, area.y + 3);
    assert_eq!(s.selected(), 0);
}

#[test]
fn keyboard_reverses_wheel_at_both_list_boundaries_without_an_extra_keypress() {
    use crossterm::event::MouseEventKind::ScrollDown;
    let paths = (0..52)
        .map(|i| PathBuf::from(format!("/example/repo{i:03}")))
        .collect();
    let (_temp, mut s) = fixture(paths);
    for (width, height) in [(60, 18), (90, 24), (140, 30)] {
        render(&mut s, width, height);
        let area = s.repository_area.unwrap();
        for _ in 0..30 {
            wheel(&mut s, ScrollDown, area.x + 2, area.y + 3);
            render(&mut s, width, height);
        }
        assert_eq!(s.selected(), 51);
        assert_eq!(s.repository_viewport.selected(), Some(51));
        key(&mut s, KeyCode::Up);
        let output = render(&mut s, width, height);
        assert_eq!(s.selected(), 50);
        assert_eq!(s.repository_viewport.selected(), Some(50));
        let highlighted = output.lines().find(|line| line.contains('›')).unwrap();
        assert!(highlighted.contains("repo050"), "{highlighted}");
        for _ in 0..30 {
            wheel(
                &mut s,
                crossterm::event::MouseEventKind::ScrollUp,
                area.x + 2,
                area.y + 3,
            );
            render(&mut s, width, height);
        }
        assert_eq!(s.selected(), 0);
        key(&mut s, KeyCode::Down);
        let output = render(&mut s, width, height);
        assert_eq!(s.selected(), 1);
        assert_eq!(s.repository_viewport.selected(), Some(1));
        let highlighted = output.lines().find(|line| line.contains('›')).unwrap();
        assert!(highlighted.contains("repo001"), "{highlighted}");
    }
}

#[test]
fn shrinking_list_at_bottom_keeps_display_and_navigation_on_the_same_selection() {
    let paths = (0..52)
        .map(|i| PathBuf::from(format!("/example/repo{i:03}")))
        .collect();
    let (_temp, mut s) = fixture(paths);
    for _ in 0..6 {
        key(&mut s, KeyCode::PageDown);
    }
    render(&mut s, 90, 24);
    assert_eq!(s.selected(), 51);
    // 列表减少一项后，渲染器会将高亮移到新的末项，导航必须立即使用同一个位置。
    s.config.repositories.pop();
    let output = render(&mut s, 90, 24);
    let highlighted = output.lines().find(|line| line.contains('›')).unwrap();
    assert!(highlighted.contains("repo050"), "{highlighted}");
    key(&mut s, KeyCode::Up);
    let output = render(&mut s, 90, 24);
    assert_eq!(s.selected_path(), Some(PathBuf::from("/example/repo049")));
    assert!(
        output
            .lines()
            .find(|line| line.contains('›'))
            .unwrap()
            .contains("repo049")
    );
    for _ in 0..10 {
        key(&mut s, KeyCode::PageDown);
    }
    key(&mut s, KeyCode::Up);
    assert_eq!(s.selected_path(), Some(PathBuf::from("/example/repo049")));
}

#[test]
fn keyboard_keeps_selection_when_momentum_arrives_after_leaving_either_boundary() {
    use crossterm::event::MouseEventKind::{ScrollDown, ScrollUp};
    let paths = (0..52)
        .map(|i| PathBuf::from(format!("/example/repo{i:03}")))
        .collect();
    let (_temp, mut s) = fixture(paths);
    render(&mut s, 90, 24);
    let area = s.repository_area.unwrap();
    for _ in 0..30 {
        wheel(&mut s, ScrollDown, area.x + 2, area.y + 3);
    }
    key(&mut s, KeyCode::Up);
    assert_eq!(s.selected(), 50);
    for _ in 0..10 {
        assert!(!wheel(&mut s, ScrollDown, area.x + 2, area.y + 3));
    }
    assert_eq!(s.selected(), 50);
    assert!(
        render(&mut s, 90, 24)
            .lines()
            .find(|line| line.contains('›'))
            .unwrap()
            .contains("repo050")
    );
    // 反向滚动属于新的明确意图，应立即恢复响应。
    for _ in 0..30 {
        wheel(&mut s, ScrollUp, area.x + 2, area.y + 3);
    }
    key(&mut s, KeyCode::Down);
    assert_eq!(s.selected(), 1);
    for _ in 0..10 {
        assert!(!wheel(&mut s, ScrollUp, area.x + 2, area.y + 3));
    }
    assert_eq!(s.selected(), 1);
    assert!(
        render(&mut s, 90, 24)
            .lines()
            .find(|line| line.contains('›'))
            .unwrap()
            .contains("repo001")
    );
}

struct FetchGate {
    started: PathBuf,
    release: PathBuf,
}
impl FetchGate {
    fn install(fixture: &support::Fixture) -> Self {
        use std::os::unix::fs::PermissionsExt;
        let wrapper = fixture.temp.path().join("delayed-upload-pack");
        support::write(
            &wrapper,
            "#!/bin/sh\ntouch \"$0.started\"\nfor i in $(seq 1 1000); do\n if [ -e \"$0.release\" ]; then exec /usr/bin/git-upload-pack \"$@\"; fi\n sleep 0.01\ndone\nexit 98\n",
        );
        std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o700)).unwrap();
        support::git(
            &fixture.repo,
            &[
                "config",
                "remote.origin.uploadpack",
                wrapper.to_str().unwrap(),
            ],
        );
        Self {
            started: wrapper.with_file_name("delayed-upload-pack.started"),
            release: wrapper.with_file_name("delayed-upload-pack.release"),
        }
    }
    fn release(&self) {
        std::fs::write(&self.release, b"release").unwrap();
    }
}
impl Drop for FetchGate {
    fn drop(&mut self) {
        let _ = std::fs::write(&self.release, b"release");
    }
}
fn poll_background_until(s: &mut Screen, ready: impl Fn(&Screen) -> bool) {
    let start = Instant::now();
    while !ready(s) {
        assert!(
            start.elapsed() < Duration::from_secs(8),
            "background check did not reach the expected state"
        );
        s.poll_check();
        s.poll(&Cancel::default());
        thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn background_batch_keeps_old_results_navigates_animates_updates_and_cancels() {
    let _language = i18n::using(i18n::Language::English);
    let first = support::Fixture::create();
    let second = support::Fixture::create();
    support::write(first.repo.join("untracked"), "old");
    support::write(second.repo.join("untracked"), "old");
    let (_home, mut s) = fixture(vec![first.repo.clone(), second.repo.clone()]);
    s.config.max_concurrency = 2;
    s.service.store.save_config(&s.config).unwrap();
    s.service.run(&Cancel::default(), |_| {}).unwrap();
    s.reload(&Cancel::default()).unwrap();
    let batch = std::fs::read(s.service.store.state_dir().join("latest.json")).unwrap();
    std::fs::remove_file(first.repo.join("untracked")).unwrap();
    std::fs::remove_file(second.repo.join("untracked")).unwrap();
    let gate = FetchGate::install(&second);
    command(&mut s, "check all");
    assert!(matches!(s.mode, Mode::Browsing));
    assert!(s.check_task.is_some());
    assert!(matches!(
        s.result_for(&first.repo),
        Some(RepositoryResult::Attention { .. })
    ));
    poll_background_until(&mut s, |s| {
        gate.started.exists()
            && matches!(
                s.result_for(&first.repo),
                Some(RepositoryResult::Clean { .. })
            )
    });
    assert!(matches!(
        s.result_for(&second.repo),
        Some(RepositoryResult::Attention { .. })
    ));
    assert!(s.check_task.as_ref().unwrap().checking(&second.repo));
    let frame = s.check_task.as_ref().unwrap().spinner();
    assert!(render(&mut s, 140, 30).contains(frame));
    thread::sleep(Duration::from_millis(110));
    assert!(s.animate_check());
    let animated = s.check_task.as_ref().unwrap().spinner();
    assert!(render(&mut s, 140, 30).contains(animated));
    for c in "no-match".chars() {
        key(&mut s, KeyCode::Char(c));
    }
    assert!(s.rows().is_empty());
    assert!(s.check_task.is_some());
    key(&mut s, KeyCode::Esc);
    s.focus(Some(second.repo.clone()));
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.page, Page::Detail);
    key(&mut s, KeyCode::Esc);
    command(&mut s, "help");
    assert_eq!(s.page, Page::Help);
    key(&mut s, KeyCode::Esc);
    command(&mut s, "check");
    assert!(s.message.contains("already running"));
    assert!(matches!(s.mode, Mode::Browsing));
    command(&mut s, "cancel");
    assert!(s.check_task.is_some());
    assert!(matches!(s.mode, Mode::Browsing));
    poll_background_until(&mut s, |s| s.check_task.is_none());
    assert!(matches!(
        s.result_for(&first.repo),
        Some(RepositoryResult::Clean { .. })
    ));
    assert!(matches!(
        s.result_for(&second.repo),
        Some(RepositoryResult::Attention { .. })
    ));
    assert_eq!(
        std::fs::read(s.service.store.state_dir().join("latest.json")).unwrap(),
        batch
    );
    assert!(s.message.contains("cancelled"));
}
#[test]
fn background_completion_preserves_the_current_detail_page_and_search() {
    let first = support::Fixture::create();
    let second = support::Fixture::create();
    let (_home, mut s) = fixture(vec![first.repo.clone(), second.repo.clone()]);
    let gate = FetchGate::install(&second);
    command(&mut s, "check all");
    poll_background_until(&mut s, |_| gate.started.exists());
    for c in "repo".chars() {
        key(&mut s, KeyCode::Char(c));
    }
    s.focus(Some(first.repo.clone()));
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.page, Page::Detail);
    gate.release();
    poll_background_until(&mut s, |s| s.check_task.is_none());
    assert_eq!(s.page, Page::Detail);
    assert_eq!(s.query, "repo");
    assert_eq!(s.bar.editor().text, "repo");
    assert_eq!(s.selected_path(), Some(first.repo.clone()));
    assert!(matches!(s.mode, Mode::Browsing));
    assert!(matches!(s.snapshot.report, CachedReport::Available { .. }));
    assert!(s.service.store.report().unwrap().is_some());
}
#[test]
fn single_check_freezes_scope_updates_one_row_and_survives_reopening_without_a_batch() {
    let first = support::Fixture::create();
    let second = support::Fixture::create();
    support::write(first.repo.join("untracked"), "new");
    let (_home, mut s) = fixture(vec![first.repo.clone(), second.repo.clone()]);
    s.focus(Some(first.repo.clone()));
    key(&mut s, KeyCode::Char('/'));
    s.paste("check");
    s.focus(Some(second.repo.clone()));
    key(&mut s, KeyCode::Enter);
    assert!(s.check_task.is_some());
    assert!(matches!(s.mode, Mode::Browsing));
    poll_background_until(&mut s, |s| s.check_task.is_none());
    assert!(matches!(
        s.result_for(&first.repo),
        Some(RepositoryResult::Attention { .. })
    ));
    assert!(s.result_for(&second.repo).is_none());
    assert!(s.service.store.report().unwrap().is_none());
    let mut reopened = Screen::new((*s.service).clone(), &Cancel::default()).unwrap();
    assert!(matches!(
        reopened.result_for(&first.repo),
        Some(RepositoryResult::Attention { .. })
    ));
    assert!(reopened.result_for(&second.repo).is_none());
    assert!(render(&mut reopened, 140, 30).contains(i18n::tr!(CheckUpdatedAt)));
    // 新一轮完整检查应覆盖更早的单仓库缓存，而不是让旧改动一直留在列表中。
    std::fs::remove_file(first.repo.join("untracked")).unwrap();
    reopened.service.run(&Cancel::default(), |_| {}).unwrap();
    reopened.reload(&Cancel::default()).unwrap();
    assert!(matches!(
        reopened.result_for(&first.repo),
        Some(RepositoryResult::Clean { .. })
    ));
}

#[test]
fn checks_started_from_files_restore_the_panel_and_block_only_writes() {
    let _language = i18n::using(i18n::Language::English);
    let f = support::Fixture::create();
    support::write(f.repo.join("new-file"), "contents");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "stage");
    wait(&mut s);
    assert!(matches!(s.mode, Mode::Files(_)));
    let gate = FetchGate::install(&f);
    command(&mut s, "check");
    assert!(matches!(s.mode, Mode::Files(_)));
    poll_background_until(&mut s, |_| gate.started.exists());
    command(&mut s, "stage");
    assert!(matches!(s.mode, Mode::Files(_)));
    assert!(s.message.contains("unavailable during a check"));
    assert!(support::git(&f.repo, &["diff", "--cached", "--name-only"]).is_empty());
    gate.release();
    poll_background_until(&mut s, |s| s.check_task.is_none());
    assert!(matches!(s.mode, Mode::Files(_)));
    command(&mut s, "stage");
    wait(&mut s);
    assert_eq!(
        support::git(&f.repo, &["diff", "--cached", "--name-only"]),
        "new-file"
    );
}

#[test]
fn manual_check_ignores_scheduled_push_policy_and_cancels_without_a_push_warning() {
    let _language = i18n::using(i18n::Language::English);
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    s.config.push_policy = PushPolicy::FastForward;
    s.service.store.save_config(&s.config).unwrap();
    let gate = FetchGate::install(&f);
    command(&mut s, "check");
    poll_background_until(&mut s, |_| gate.started.exists());
    command(&mut s, "cancel");
    poll_background_until(&mut s, |s| s.check_task.is_none());
    assert!(s.message.contains("cancelled"));
    assert!(!s.message.contains("writes"));
    assert!(matches!(s.mode, Mode::Browsing));
    assert!(s.service.store.report().unwrap().is_none());
}

#[test]
fn configuration_confirmation_returns_to_editing_and_saves_only_on_accept() {
    let (_home, mut s) = fixture(vec![]);
    command(&mut s, "concurrency");
    s.mode = Mode::Input {
        action: Action::Concurrency,
        editor: Editor::new("7".into()),
    };
    key(&mut s, KeyCode::Enter);
    key(&mut s, KeyCode::Right);
    key(&mut s, KeyCode::Enter);
    assert!(matches!(&s.mode, Mode::Input { editor, .. } if editor.text == "7"));
    assert_eq!(s.service.store.config().unwrap().max_concurrency, 4);
    key(&mut s, KeyCode::Enter);
    key(&mut s, KeyCode::Esc);
    assert!(matches!(&s.mode, Mode::Input { editor, .. } if editor.text == "7"));
    key(&mut s, KeyCode::Enter);
    key(&mut s, KeyCode::Char('y'));
    key(&mut s, KeyCode::Char('n'));
    assert!(matches!(s.mode, Mode::Confirm { .. }));
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(s.service.store.config().unwrap().max_concurrency, 7);
}

#[test]
fn horizontal_confirmation_focus_has_distinct_styles_and_keeps_layout_stable() {
    use ratatui::{
        Terminal,
        backend::TestBackend,
        style::{Color, Modifier},
    };
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        for (width, height) in [(60, 18), (80, 24), (150, 32)] {
            let (_home, mut s) = fixture(vec!["/example/repository".into()]);
            render(&mut s, width, height);
            command(&mut s, "remove");
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
            let body = s.body_area;
            assert!(!terminal.backend().cursor_visible());
            let focused_positions = |terminal: &Terminal<TestBackend>| {
                let buffer = terminal.backend().buffer();
                (0..width)
                    .filter(|x| {
                        let cell = &buffer[(*x, height - 3)];
                        cell.bg == Color::Rgb(180, 145, 220)
                            && cell.modifier.contains(Modifier::BOLD)
                    })
                    .collect::<Vec<_>>()
            };
            let execute = focused_positions(&terminal);
            assert!(!execute.is_empty());
            key(&mut s, KeyCode::Right);
            terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
            assert_eq!(s.body_area, body);
            let back = focused_positions(&terminal);
            assert!(!back.is_empty());
            assert!(execute.last().unwrap() < back.first().unwrap());
            key(&mut s, KeyCode::Enter);
            assert!(matches!(s.mode, Mode::Browsing));
            assert_eq!(s.config.repositories.len(), 1);
            command(&mut s, "apply");
            key(&mut s, KeyCode::Esc);
            assert!(matches!(s.mode, Mode::Browsing));
        }
    }
}
