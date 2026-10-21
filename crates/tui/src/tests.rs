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
        selected: 0,
        filter: Filter::All,
        query: String::new(),
        order: Order::Priority,
        setting_selected: 0,
        scroll: 0,
        message: String::new(),
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
    let manager = Manager {
        user_home: temp.path().to_owned(),
        uid: unsafe { libc::getuid() },
    };
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
    command(&mut s, "check");
    assert!(matches!(s.bar, crate::bar::InputBar::Command { .. }));
    key(&mut s, KeyCode::Esc);
    command(&mut s, "check all");
    assert!(matches!(s.mode, Mode::Confirm(Confirmation::CheckAll)));
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Char('/'));
    s.paste("remove");
    s.selected = 1;
    key(&mut s, KeyCode::Enter);
    assert!(
        matches!(&s.mode,Mode::Confirm(Confirmation::Remove(p))if p.as_path()==std::path::Path::new("/path/to/a"))
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
        command(&mut s, "check all");
        let text = render(&mut s, size.0, size.1);
        assert!(text.contains("并发上限"));
        assert!(text.contains("y 确认执行"));
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
        for size in [(60, 18), (80, 24), (150, 32)] {
            let text = render(&mut s, size.0, size.1);
            assert!(text.contains(i18n::tr!(ConfirmCommit)));
            assert!(text.contains(if language == i18n::Language::English {
                "y commit"
            } else {
                "y 确认提交"
            }));
        }
        assert_eq!(before, support::git(&f.repo, &["rev-parse", "HEAD"]));
        key(&mut s, KeyCode::Char('y'));
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
        key(&mut s, KeyCode::Char('y'));
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
    assert_eq!(s.selected, 1);
    key(&mut s, KeyCode::Left);
    assert_eq!(s.selected, 1);
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
        assert!(matches!(s.mode, Mode::Confirm(_)));
        assert!(!terminal.backend().cursor_visible());
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
        assert!(confirm.contains("y confirm"));
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
        s.selected = 1;
        assert!(
            matches!(&s.bar, crate::bar::InputBar::Command { path: Some(path), .. } if path == &PathBuf::from("/example/unavailable-alpha"))
        );
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert!(matches!(s.mode, Mode::Error { .. }));
        let text = render(&mut s, 80, 24);
        assert!(text.contains(i18n::tr!(TerminalDirectoryMissing)));
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.selected, 1);
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
