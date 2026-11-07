mod auto_fetch;
mod branches;
mod breadcrumb_returns;
mod breadcrumbs;
mod browser;
mod browser_horizontal;
mod changes;
mod command_availability;
mod command_navigation;
mod command_polish;
mod command_trigger;
mod command_values;
mod commands;
mod commit_message;
mod completion;
mod composition;
mod concurrency;
mod conflict;
mod content_locate;
mod contexts;
mod copy;
mod daily_operations;
mod diff;
mod diff_context;
mod diff_header;
mod double_click;
mod external_editor;
mod file_icons;
mod file_path;
mod file_tree;
mod focus;
mod global_actions;
mod help;
mod history;
mod history_actions;
mod history_review;
mod horizontal;
mod images;
mod input_surface;
mod interrupt;
mod layout_picker;
mod links;
mod live;
mod mouse;
mod navigation;
mod opening;
mod options;
mod overview;
mod partial;
mod prefix_segments;
mod prefix_selection;
mod preview_loading;
mod push;
mod recovery_controls;
mod replay;
mod replay_batch;
mod repository_list;
mod responsive;
mod scan_management;
mod schedule;
mod scrollbars;
mod search;
mod settings_design;
mod startup;
mod startup_fetch;
mod stash_preview;
mod stash_save;
mod sync_progress;
mod task_center;
mod task_feedback;
mod text_selection;
mod view_preferences;
mod workflows;
mod workspace;
mod workspace_navigation;
mod worktree_folding;
mod wrapping;
use super::*;
use crate::repository_list::{Filter, Order};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use engine::{
    domain::*,
    scheduler::{Manager, Registration, Schedule},
    service::{Activity, CachedReport},
    state::Store,
};
use ratatui::backend::TestBackend;
use std::{collections::BTreeSet, time::Instant};
use test_support as support;
fn screen(service: Service) -> Screen {
    let config = service.store.config().unwrap();
    let view_preferences = config.view;
    let repository_viewport = ratatui::widgets::TableState::default()
        .with_selected((!config.repositories.is_empty()).then_some(0));
    Screen {
        snapshot: Snapshot {
            registration: Registration::Loaded,
            activity: Activity::Idle,
            configured_interval_seconds: config.interval_seconds,
            configured_anchor: config.schedule_anchor.clone(),
            next_trigger: engine::scheduler::NextTrigger::Unanchored,
            push_policy: config.scheduled_actions.push_policy(),
            scheduled_actions: config.scheduled_actions.clone(),
            repository_count: config.repositories.len(),
            schedule: Schedule::Saved {
                interval_seconds: 21600,
            },
            report: CachedReport::Missing,
            max_concurrency: config.max_concurrency,
            fetch_on_startup: config.fetch_on_startup,
        },
        service: Arc::new(service),
        config,
        page: Page::Repositories,
        history: vec![],
        mode: Mode::Browsing,
        focus_mode: Default::default(),
        interaction_save: Default::default(),
        view_save: Default::default(),
        diff_preference: view_preferences.diff_layout.into(),
        diff_header: view_preferences.diff_header,
        diff_context: view_preferences.diff_context,
        file_layout: view_preferences.file_layout.into(),
        graphics: graphics::Display::new(graphics::Support::Pixels(graphics::Backend::test())),
        bar: crate::bar::InputBar::default(),
        exit_sequence: Default::default(),
        clipboard: Default::default(),
        link_opener: Default::default(),
        external_editor: Default::default(),
        command_repository: Default::default(),
        commit_generator: Default::default(),
        commit_draft_id: 0,
        repository_viewport,
        scroll_input: input::scroll::ScrollInput::default(),
        repository_area: None,
        body_area: None,
        input_area: None,
        pointer: Default::default(),
        clicks: Default::default(),
        drag: None,
        command_items: Vec::new(),
        command_viewport: Default::default(),
        command_reader: Default::default(),
        breadcrumb_items: Vec::new(),
        filter: Filter::All,
        query: String::new(),
        search_mode: Default::default(),
        order: view_preferences.repository_order.into(),
        setting_selected: 0,
        settings_focus: None,
        input_surface: Default::default(),
        scroll: 0,
        message: String::new(),
        tasks: Default::default(),
        check_task: None,
        repository_tasks: BTreeMap::new(),
        repository_errors: BTreeMap::new(),
        view_revision: 0,
        updates: BTreeMap::new(),
        live: Default::default(),
        window_focus: crate::live::WindowFocus::Focused,
        auto_fetch: Default::default(),
        overview: Default::default(),
        replay: Default::default(),
        workspace: Default::default(),
        workspace_areas: Vec::new(),
    }
}
fn fixture(paths: Vec<PathBuf>) -> (tempfile::TempDir, Screen) {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::custom(temp.path().to_owned());
    store
        .save_config(&Config {
            interval_seconds: 21600,
            schedule_anchor: Default::default(),
            repositories: paths,
            scheduled_actions: engine::workflow::Plan::from_policy(PushPolicy::Disabled),
            max_concurrency: 4,
            fetch_on_startup: false,
            auto_fetch: Default::default(),
            chinese_command_trigger: true,
            text_wrap: false,
            file_icons: true,
            interaction_mode: Default::default(),
            view: Default::default(),
            mouse: Default::default(),
            editor: Default::default(),
            ai: Default::default(),
        })
        .unwrap();
    let manager = Manager::for_user(temp.path().to_owned(), unsafe { libc::getuid() }).unwrap();
    let service = Service::new(store, manager);
    (temp, screen(service))
}
fn key(s: &mut Screen, code: KeyCode) -> bool {
    s.key(KeyEvent::new(code, KeyModifiers::NONE), &Cancel::default())
        .unwrap()
}
fn command(s: &mut Screen, text: &str) {
    while matches!(
        s.bar,
        bar::InputBar::Command { .. } | bar::InputBar::Layout { .. }
    ) {
        key(s, KeyCode::Esc);
    }
    // 既有工作流用显式全局命令；上下文后缀由 contexts 测试直接覆盖。
    s.paste(&format!("//{text}"));
    key(s, KeyCode::Enter);
}
fn open_files(s: &mut Screen) {
    // 直接准备共享文件面板；统一命令入口与预览交互由 changes 测试单独覆盖。
    s.read_files(s.scope_path().expect("selected repository"));
    let started = Instant::now();
    while s
        .repository_tasks
        .contains_key(&s.scope_path().expect("selected repository"))
    {
        assert!(started.elapsed() < Duration::from_secs(8));
        s.poll(&Cancel::default());
        s.poll_workspace();
        thread::sleep(Duration::from_millis(5));
    }
}
fn focus_next(s: &mut Screen) {
    s.cycle_pane(false);
    s.sync_plain_bar();
}
fn zoom(s: &mut Screen) {
    command(s, "zoom");
}
fn maximize_preview(s: &mut Screen) {
    let Mode::Changes(session) = &mut s.mode else {
        panic!("changes workspace expected")
    };
    session.focus = crate::changes::Focus::Preview;
    if session.arrangement != crate::workspace::Arrangement::Maximized {
        zoom(s);
    }
}
fn wait(s: &mut Screen) {
    s.poll_changes();
    s.poll_command_completion();
    let started = Instant::now();
    while s.interaction_save.pending()
        || s.command_reader.pending()
        || s.view_save.pending()
        || matches!(s.mode, Mode::Busy(_))
        || matches!(s.mode, Mode::Panel(section) if section != crate::workspace::Section::Overview)
        || !s.repository_tasks.is_empty()
        || matches!(&s.mode, Mode::Changes(session) if session.pending())
    {
        assert!(
            started.elapsed() < Duration::from_secs(8),
            "工作线程没有结束"
        );
        s.poll(&Cancel::default());
        s.poll_interaction();
        s.poll_view_preferences();
        s.poll_workspace();
        s.poll_changes();
        s.poll_command_completion();
        thread::sleep(Duration::from_millis(5));
    }
}
fn complete_path(s: &mut Screen) {
    key(s, KeyCode::Tab);
    wait(s);
}
fn wait_workspace(s: &mut Screen) {
    let started = Instant::now();
    loop {
        s.poll_workspace();
        if !s.workspace.pending() {
            break;
        }
        assert!(
            started.elapsed() < Duration::from_secs(8),
            "仓库面板读取没有结束"
        );
        thread::sleep(Duration::from_millis(5));
    }
}
fn render(s: &mut Screen, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| view::render(frame, s)).unwrap();
    s.graphics.settle();
    terminal.draw(|frame| view::render(frame, s)).unwrap();
    s.graphics.finish_frame(&mut Vec::new()).unwrap();
    if width == 0 {
        return String::new();
    }
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
                let symbol = String::from_utf8(strip_ansi_escapes::strip(cell.symbol())).unwrap();
                text.push_str(&symbol);
                skip = unicode_width::UnicodeWidthStr::width(symbol.as_str()).saturating_sub(1);
            }
            text
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn highlighted_repository(s: &mut Screen, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| view::render(frame, s)).unwrap();
    let area = s.repository_area.unwrap();
    let buffer = terminal.backend().buffer();
    let rows = (area.y + 2..area.bottom() - 1)
        .filter(|y| buffer[(area.x + 2, *y)].bg == ratatui::style::Color::Rgb(25, 65, 95))
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1, "exactly one repository row is highlighted");
    let text: String = (area.x + 2..area.right() - 1)
        .map(|x| buffer[(x, rows[0])].symbol())
        .collect();
    assert!(!text.contains('›'));
    assert_eq!(buffer[(area.x + 1, rows[0])].symbol(), " ");
    assert_eq!(buffer[(area.x + 1, area.y + 1)].symbol(), " ");
    assert!(
        !text.starts_with(' '),
        "content starts after the single-column inset"
    );
    text
}

#[test]
fn command_completion_unknown_commands_and_frozen_scope() {
    let _language = i18n::using(i18n::Language::Chinese);
    let (_temp, mut s) = fixture(vec!["/path/to/a".into(), "/path/to/b".into()]);
    key(&mut s, KeyCode::Char('/'));
    s.paste("/repo diff sta");
    key(&mut s, KeyCode::Tab);
    assert!(
        matches!(&s.bar,crate::bar::InputBar::Command{input,..}if input.text()=="/repo diff stage")
    );
    key(&mut s, KeyCode::Esc);
    command(&mut s, "repo fetch nope");
    assert!(matches!(s.bar, crate::bar::InputBar::Command { .. }));
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Char('/'));
    s.paste("/repos remove");
    s.select(1);
    key(&mut s, KeyCode::Enter);
    assert!(
        matches!(&s.mode,Mode::Confirm{operation:Confirmation::Remove { path: p, .. },..}if p.as_path()==std::path::Path::new("/path/to/a"))
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
        assert!(text.contains("过滤仓库"));
        assert!(!text.contains("1 / 1"));
        key(&mut s, KeyCode::Char('/'));
        let text = render(&mut s, size.0, size.1);
        assert!(text.contains("命令"));
        assert!(text.contains("Tab 补全"));
        key(&mut s, KeyCode::Esc);
        command(&mut s, "repos remove");
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
        open_files(&mut s);
        wait(&mut s);
        assert!(matches!(s.mode, Mode::Files(_)));
        key(&mut s, KeyCode::Char(' '));
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert!(matches!(&s.mode, Mode::Changes(_)));
        assert!(render(&mut s, 100, 28).contains("+one"));
        key(&mut s, KeyCode::Esc);
        key(&mut s, KeyCode::Char('/'));
        key(&mut s, KeyCode::Esc);
        assert!(matches!(&s.mode,Mode::Files(session)if session.selected.len()==1));
        command(&mut s, "repo diff stage");
        wait(&mut s);
        assert_eq!(
            support::git(&f.repo, &["diff", "--cached", "--name-only"]),
            "one.txt"
        );
        command(&mut s, "repo diff unstage");
        wait(&mut s);
        assert!(support::git(&f.repo, &["diff", "--cached"]).is_empty());
        key(&mut s, KeyCode::Tab);
        command(&mut s, "repo diff stage");
        wait(&mut s);
        command(&mut s, "repo commit");
        wait(&mut s);
        assert!(matches!(s.mode, Mode::CommitMessage { .. }));
        s.paste("通过命令栏提交");
        let before = support::git(&f.repo, &["rev-parse", "HEAD"]);
        key(&mut s, KeyCode::BackTab);
        key(&mut s, KeyCode::Tab);
        key(&mut s, KeyCode::Right);
        assert!(
            matches!(&s.mode, Mode::CommitMessage { editor, .. } if editor.text() == "通过命令栏提交")
        );
        key(&mut s, KeyCode::Left);
        for size in [(60, 18), (80, 24), (150, 32)] {
            let text = render(&mut s, size.0, size.1);
            assert!(text.contains(i18n::tr!(CommitMessageAlt).trim()));
            assert!(text.contains(i18n::tr!(ActionCommit)));
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
        command(&mut s, "repo push");
        wait(&mut s);
        assert!(!matches!(s.mode, Mode::PushConfirm { .. }));
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
    open_files(&mut s);
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
    assert_eq!(s.query, "alpha");
    assert_eq!(s.page, Page::Repositories);
    assert_eq!(s.selected(), 1);
    key(&mut s, KeyCode::Esc);
    assert!(s.query.is_empty());
    assert_eq!(s.rows().len(), 3);
    for c in "quit".chars() {
        assert!(!key(&mut s, KeyCode::Char(c)));
    }
    assert_eq!(s.query, "quit");
    assert!(matches!(s.mode, Mode::Browsing));
    key(&mut s, KeyCode::Esc);
    assert!(!key(&mut s, KeyCode::Char('/')));
    s.paste("/quit");
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
    s.paste("//settings");
    assert_eq!(s.page, Page::Repositories);
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.page, Page::Configuration(Configuration::General));
    s.paste("//repo push");
    key(&mut s, KeyCode::Home);
    key(&mut s, KeyCode::Delete);
    assert!(matches!(s.bar, crate::bar::InputBar::CommandsOnly));
    assert!(s.query.is_empty());
    assert!(s.bar.text().is_empty());
    assert_eq!(s.page, Page::Configuration(Configuration::General));
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
            .position(|line| line.contains("仓库 · 1"))
            .unwrap();
        let input_line = before_lines
            .iter()
            .position(|line| line.contains("输入关键词搜索"))
            .unwrap();
        key(&mut s, KeyCode::Char('/'));
        let during = render(&mut s, w, h);
        let during_lines = during.lines().collect::<Vec<_>>();
        assert!(during_lines[list_line].contains("仓库 · 1"));
        assert!(during_lines[input_line].contains("/") && during_lines[input_line].contains("›"));
        let top = s.command_items[0].0.y as usize - 1;
        assert!(during_lines[top].starts_with('╭'));
        assert!(during_lines[top].ends_with('╮'));
        assert_eq!(
            unicode_width::UnicodeWidthStr::width(during_lines[top]),
            w as usize
        );
        assert!(during_lines[top + 1].starts_with('│'));
        assert!(during_lines[top + 1].ends_with(['│', '┃']));
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
    open_files(&mut s);
    wait(&mut s);
    s.paste("two");
    assert!(matches!(&s.mode,Mode::Files(session)if session.rows().len()==1));
    key(&mut s, KeyCode::Insert);
    key(&mut s, KeyCode::Esc);
    assert!(
        matches!(&s.mode,Mode::Files(session)if session.rows().len()==2&&session.selected.len()==1)
    );
    command(&mut s, "repo diff stage");
    wait(&mut s);
    assert_eq!(
        support::git(&f.repo, &["diff", "--cached", "--name-only"]),
        "two.txt"
    );
}

#[test]
fn numeric_search_and_list_commands_do_not_conflict() {
    let _language = i18n::using(i18n::Language::Chinese);
    let (_temp, mut s) = fixture(vec!["/path/to/1234".into()]);
    for c in "1234".chars() {
        key(&mut s, KeyCode::Char(c));
    }
    assert_eq!(s.query, "1234");
    assert_eq!(s.filter, Filter::All);
    assert_eq!(s.rows().len(), 1);
    for (name, expected) in [
        ("repos filter problems", Filter::Problems),
        ("repos filter unchecked", Filter::Unchecked),
        ("repos filter all", Filter::All),
    ] {
        key(&mut s, KeyCode::Home);
        key(&mut s, KeyCode::Char('/'));
        s.paste(&format!("/{name}"));
        key(&mut s, KeyCode::Enter);
        assert_eq!(s.filter, expected);
        assert_eq!(s.query, "1234");
        assert_eq!(s.bar.editor().text, "1234");
    }
    key(&mut s, KeyCode::Esc);
    s.paste("//");
    s.key(
        KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
        &Cancel::default(),
    )
    .unwrap();
    assert_eq!(s.filter, Filter::All);
    assert_eq!(s.bar.text(), "/");
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
        assert_eq!(cursor, (3, height - 4).into());
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(0, height - 5)].symbol(), "╭");
        assert_eq!(buffer[(width - 1, height - 3)].symbol(), "╯");
        let placeholder = &buffer[(3, height - 4)];
        assert_eq!(placeholder.fg, Color::DarkGray);
        assert!(placeholder.modifier.contains(Modifier::ITALIC));
        s.paste("中文1234");
        terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
        assert_eq!(
            terminal.backend().cursor_position(),
            (11, height - 4).into()
        );
        let text = &terminal.backend().buffer()[(3, height - 4)];
        assert_eq!(text.fg, Color::Reset);
        assert!(!text.modifier.contains(Modifier::ITALIC));
        key(&mut s, KeyCode::Esc);
        command(&mut s, "settings concurrency 4");
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
        assert!(text.contains("Filter repositories"));
        assert!(text.contains("Type to search"));
        assert!(text.contains("Filter: All · Sort: Priority"));
        assert!(!text.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c)));
        s.paste("//");
        let candidates = render(&mut s, width, height);
        assert!(!candidates.contains("Current repo"));
        assert!(candidates.contains("Open repository"));
        assert!(candidates.contains("Tab complete"));
        assert!(candidates.contains("/repo"));
        key(&mut s, KeyCode::Esc);
        command(&mut s, "settings concurrency");
        let input = render(&mut s, width, height);
        assert!(input.contains("<count>"));
        s.paste("4");
        key(&mut s, KeyCode::Enter);
        let confirm = render(&mut s, width, height);
        assert!(confirm.contains("Enter confirm"));
        key(&mut s, KeyCode::Esc);
        assert!(matches!(s.mode, Mode::Browsing));
        assert_eq!(s.bar.text(), "/settings concurrency 4");
        key(&mut s, KeyCode::Esc);
    }
    key(&mut s, KeyCode::Enter);
    let detail = render(&mut s, 80, 24);
    assert!(detail.contains("Path: /example/alpha"));
    assert!(detail.contains(i18n::tr!(OverviewLoading)));
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
    open_files(&mut s);
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
        s.paste("//te");
        key(&mut s, KeyCode::Tab);
        assert_eq!(s.bar.text(), "/terminal");
        assert_eq!(
            command::matches("terminal")
                .first()
                .map(|command| command.action),
            Some(Action::Terminal)
        );
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
    let mut files = FileSession::new(s.service.files(&f.repo, &Cancel::default()).unwrap());
    files.search("filter".into());
    let origin = Mode::Files(files);
    s.finish(
        Outcome::TerminalOpened(f.repo.clone()),
        origin,
        &Cancel::default(),
    );
    assert!(matches!(&s.mode, Mode::Files(session) if session.query == "filter"));
    assert!(s.message.starts_with("Opened terminal:"));
    assert_eq!(s.bar.editor().text, "filter");
    assert!(s.service.store.report().unwrap().is_none());
}

#[test]
fn command_panel_shows_eight_candidates_and_scrolls_to_remaining_commands() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_temp, mut s) = fixture(vec!["/example/alpha".into()]);
        for (width, height) in [(60, 24), (100, 28), (150, 32)] {
            s.paste("/repo");
            let text = render(&mut s, width, height);
            assert_eq!(s.command_items.len(), 8, "{text}");
            assert!(s.command_items.iter().all(|(area, _)| area.height == 1));
            for item in s.search_commands().unwrap().into_iter().take(8) {
                assert!(text.contains(&format!("/{}", item.name)));
            }
            for _ in 0..command::COMMANDS.len() {
                key(&mut s, KeyCode::Down);
            }
            let last = render(&mut s, width, height);
            assert_eq!(s.command_items.len(), 8);
            assert!(last.contains(s.search_commands().unwrap().last().unwrap().name));
            key(&mut s, KeyCode::Esc);
        }
    }
}

#[test]
fn short_command_viewports_keep_whole_entries_and_the_last_command_clickable() {
    let (_temp, mut s) = fixture(vec!["/example/alpha".into()]);
    for height in [12, 17, 18, 21, 24] {
        s.paste("/repo");
        render(&mut s, 60, height);
        let count = s.command_items.len();
        assert!((1..=8).contains(&count));
        for _ in 0..command::COMMANDS.len() {
            key(&mut s, KeyCode::Down);
        }
        let text = render(&mut s, 60, height);
        assert_eq!(s.command_items.len(), count, "{text}");
        assert!(s.command_items.iter().all(|(area, _)| area.height == 1));
        let (_, index) = s.command_items.last().unwrap();
        let commands = s.search_commands().unwrap();
        assert_eq!(*index, commands.len() - 1, "{text}");
        assert!(text.contains(commands.last().unwrap().name), "{text}");
        key(&mut s, KeyCode::Esc);
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
    let compact = s.repository_area.unwrap();
    assert!(wheel(&mut s, ScrollDown, compact.x + 2, compact.y + 3));
    assert_eq!(s.selected(), 3);
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
    command(&mut s, "repos remove");
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
        let highlighted = highlighted_repository(&mut s, width, height);
        assert_eq!(s.selected(), 50);
        assert_eq!(s.repository_viewport.selected(), Some(50));
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
        let highlighted = highlighted_repository(&mut s, width, height);
        assert_eq!(s.selected(), 1);
        assert_eq!(s.repository_viewport.selected(), Some(1));
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
    let highlighted = highlighted_repository(&mut s, 90, 24);
    assert!(highlighted.contains("repo050"), "{highlighted}");
    key(&mut s, KeyCode::Up);
    let highlighted = highlighted_repository(&mut s, 90, 24);
    assert_eq!(s.selected_path(), Some(PathBuf::from("/example/repo049")));
    assert!(highlighted.contains("repo049"));
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
    assert!(highlighted_repository(&mut s, 90, 24).contains("repo050"));
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
    assert!(highlighted_repository(&mut s, 90, 24).contains("repo001"));
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
            "background check did not reach the expected state: {}; errors: {:?}; running: {:?}",
            s.message,
            s.repository_errors,
            s.repository_tasks.keys().collect::<Vec<_>>()
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
    let batch = serde_json::to_vec(&s.service.store.report().unwrap()).unwrap();
    std::fs::remove_file(first.repo.join("untracked")).unwrap();
    std::fs::remove_file(second.repo.join("untracked")).unwrap();
    let gate = FetchGate::install(&second);
    command(&mut s, "repos fetch");
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
    command(&mut s, "repo fetch");
    assert!(s.repository_tasks.contains_key(&second.repo));
    assert!(matches!(s.mode, Mode::Browsing));
    command(&mut s, "cancel");
    poll_background_until(&mut s, |s| s.repository_tasks.is_empty());
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
        serde_json::to_vec(&s.service.store.report().unwrap()).unwrap(),
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
    command(&mut s, "repos fetch");
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
    assert!(s.bar.editor().text.is_empty());
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
    s.paste("/repo fetch");
    s.focus(Some(second.repo.clone()));
    key(&mut s, KeyCode::Enter);
    assert!(!s.repository_tasks.is_empty());
    assert!(matches!(s.mode, Mode::Browsing));
    poll_background_until(&mut s, |s| s.repository_tasks.is_empty());
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
    reopened.focus(Some(first.repo.clone()));
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
fn checks_started_from_files_leave_the_panel_interactive_and_serialize_writes() {
    let _language = i18n::using(i18n::Language::English);
    let f = support::Fixture::create();
    support::write(f.repo.join("new-file"), "contents");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    open_files(&mut s);
    wait(&mut s);
    assert!(matches!(s.mode, Mode::Files(_)));
    let gate = FetchGate::install(&f);
    command(&mut s, "repo fetch");
    assert!(matches!(s.mode, Mode::Files(_)));
    poll_background_until(&mut s, |_| gate.started.exists());
    command(&mut s, "repo diff stage");
    assert!(matches!(s.mode, Mode::Files(_)));
    assert!(s.message.contains("already running"));
    assert!(support::git(&f.repo, &["diff", "--cached", "--name-only"]).is_empty());
    gate.release();
    poll_background_until(&mut s, |s| s.repository_tasks.is_empty());
    assert!(matches!(s.mode, Mode::Files(_)));
    command(&mut s, "repo diff stage");
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
    s.config.scheduled_actions = engine::workflow::Plan::from_policy(PushPolicy::FastForward);
    s.service.store.save_config(&s.config).unwrap();
    let gate = FetchGate::install(&f);
    command(&mut s, "repo fetch");
    poll_background_until(&mut s, |_| gate.started.exists());
    command(&mut s, "cancel");
    poll_background_until(&mut s, |s| s.repository_tasks.is_empty());
    assert!(s.message.contains("cancelled"));
    assert!(!s.message.contains("writes"));
    assert!(matches!(s.mode, Mode::Browsing));
    assert!(s.service.store.report().unwrap().is_none());
}

#[test]
fn configuration_confirmation_returns_to_editing_and_saves_only_on_accept() {
    let (_home, mut s) = fixture(vec![]);
    command(&mut s, "settings concurrency 7");
    key(&mut s, KeyCode::Right);
    key(&mut s, KeyCode::Enter);
    assert!(matches!(s.mode, Mode::Browsing));
    assert_eq!(s.bar.text(), "/settings concurrency 7");
    assert_eq!(s.service.store.config().unwrap().max_concurrency, 4);
    key(&mut s, KeyCode::Enter);
    key(&mut s, KeyCode::Esc);
    assert!(matches!(s.mode, Mode::Browsing));
    assert_eq!(s.bar.text(), "/settings concurrency 7");
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
            command(&mut s, "repos remove");
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
            let body = s.body_area;
            assert!(!terminal.backend().cursor_visible());
            let focused_positions = |terminal: &Terminal<TestBackend>| {
                let buffer = terminal.backend().buffer();
                (0..width)
                    .filter(|x| {
                        let cell = &buffer[(*x, height - 4)];
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
            command(&mut s, "schedule apply");
            key(&mut s, KeyCode::Esc);
            assert!(matches!(s.mode, Mode::Browsing));
        }
    }
}

#[test]
fn root_commands_hide_children_and_group_prefixes_keep_their_scope() {
    let root = command::matches("");
    assert!(root.iter().all(|command| !command.name.contains(' ')));
    assert!(root.iter().any(|command| command.name == "repo"));
    assert!(root.iter().any(|command| command.name == "repos"));
    for removed in [
        "interval",
        "autopush",
        "concurrency",
        "apply",
        "add",
        "remove",
        "exclude",
        "scan",
        "import",
    ] {
        assert!(
            command::COMMANDS
                .iter()
                .all(|command| command.name != removed)
        );
    }
    assert_eq!(
        command::matches("schedule ")
            .iter()
            .map(|c| c.name)
            .collect::<Vec<_>>(),
        [
            "schedule interval",
            "schedule anchor",
            "schedule autopush",
            "schedule actions",
            "schedule apply",
            "schedule enabled"
        ]
    );
    assert_eq!(
        command::matches("settings ")
            .iter()
            .map(|c| c.name)
            .collect::<Vec<_>>(),
        [
            "settings concurrency",
            "settings startup-fetch",
            "settings auto-fetch",
            "settings auto-fetch-interval",
            "settings wrap",
            "settings interaction",
            "settings ai",
            "settings editor",
            "settings ai agent",
            "settings ai model",
            "settings ai prompt",
        ]
    );
    assert_eq!(
        command::matches("repos fetch")
            .iter()
            .map(|c| c.name)
            .collect::<Vec<_>>(),
        ["repos fetch"]
    );
    assert_eq!(command::matches("repos ").len(), 7);
    assert!(command::matches("schedule unknown").is_empty());
    assert_eq!(
        command::matches("repo fetch")
            .first()
            .map(|command| command.action),
        Some(Action::Fetch)
    );
    assert_eq!(
        command::matches("repos fetch")
            .first()
            .map(|command| command.action),
        Some(Action::FetchAll)
    );
}

#[test]
fn schedule_settings_are_separate_from_general_and_repository_management() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec!["/example/alpha".into()]);
        command(&mut s, "schedule");
        assert_eq!(s.page, Page::Configuration(Configuration::Schedule));
        for (width, height) in [(60, 18), (100, 28), (150, 32)] {
            let text = render(&mut s, width, height);
            assert!(text.contains(i18n::tr!(ScheduleSettings)));
            assert!(text.contains(i18n::tr!(WorkflowScheduledActions)));
            assert!(!text.contains("/example/excluded"));
            assert!(!text.contains(command::description(Action::Concurrency)));
        }
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Enter);
        assert!(matches!(s.mode, Mode::Workflow(_)));
        key(&mut s, KeyCode::Esc);
        command(&mut s, "settings");
        assert_eq!(s.page, Page::Configuration(Configuration::General));
        let text = render(&mut s, 100, 28);
        assert!(text.contains(i18n::tr!(SettingsConcurrency)));
        assert!(!text.contains(command::description(Action::Interval)));
        assert!(!text.contains(i18n::tr!(SystemSchedule).split('{').next().unwrap()));
        assert_eq!(s.setting_selected, 0);
        key(&mut s, KeyCode::Enter);
        assert!(matches!(s.mode, Mode::Browsing));
        assert!(s.bar.text().starts_with("/settings concurrency "));
        key(&mut s, KeyCode::Esc);
        command(&mut s, "repos manage");
        assert_eq!(s.page, Page::Configuration(Configuration::Repositories));
        let text = render(&mut s, 100, 28);
        assert!(text.contains("/example/alpha"));
        assert!(!text.contains(command::description(Action::Interval)));
    }
}

#[test]
fn grouped_commands_keep_frozen_repository_scope_and_preserve_the_eight_row_menu() {
    let (_home, mut s) = fixture(vec!["/example/alpha".into(), "/example/beta".into()]);
    key(&mut s, KeyCode::Char('/'));
    s.paste("/repos");
    key(&mut s, KeyCode::Tab);
    assert_eq!(s.bar.text(), "/repos");
    s.select(1);
    s.paste(" rem");
    key(&mut s, KeyCode::Tab);
    assert_eq!(s.bar.text(), "/repos remove");
    key(&mut s, KeyCode::Enter);
    assert!(
        matches!(&s.mode, Mode::Confirm { operation: Confirmation::Remove { path, .. }, .. } if path == &PathBuf::from("/example/alpha"))
    );
    key(&mut s, KeyCode::Esc);
    s.paste("//schedule ");
    s.paste("int");
    key(&mut s, KeyCode::Tab);
    assert_eq!(s.bar.text(), "/schedule interval ");
    key(&mut s, KeyCode::Enter);
    assert!(matches!(s.mode, Mode::Browsing));
    assert!(s.bar.text().starts_with("/schedule interval "));
}

#[test]
fn schedule_portal_completion_and_page_navigation_do_not_execute_or_install_tasks() {
    let (_home, mut s) = fixture(vec![]);
    let before = s.service.store.config().unwrap();
    s.paste("//sche");
    key(&mut s, KeyCode::Tab);
    assert_eq!(s.bar.text(), "/schedule");
    assert_eq!(s.search_commands().unwrap().len(), 7);
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.page, Page::Configuration(Configuration::Schedule));
    assert_eq!(s.setting_selected, 0);
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Enter);
    assert!(matches!(s.mode, Mode::Browsing));
    assert!(s.bar.text().starts_with("/schedule interval "));
    key(&mut s, KeyCode::Esc);
    for (name, page) in [
        ("schedule", Page::Configuration(Configuration::Schedule)),
        ("repos", Page::Repositories),
        ("settings", Page::Configuration(Configuration::General)),
        (
            "repos manage",
            Page::Configuration(Configuration::Repositories),
        ),
        ("help", Page::Help),
    ] {
        command(&mut s, name);
        assert_eq!(s.page, page);
        let depth = s.history.len();
        for code in [KeyCode::Tab, KeyCode::BackTab] {
            key(&mut s, code);
            assert_eq!(s.page, page);
            assert_eq!(s.history.len(), depth);
            assert!(if name == "repos manage" {
                matches!(s.mode, Mode::Management(_))
            } else {
                matches!(s.mode, Mode::Browsing)
            });
        }
    }
    assert_eq!(
        s.service.store.config().unwrap().interval_seconds,
        before.interval_seconds
    );
    assert_eq!(
        s.service
            .store
            .config()
            .unwrap()
            .scheduled_actions
            .push_policy(),
        before.scheduled_actions.push_policy()
    );
    assert!(s.service.store.report().unwrap().is_none());
}

#[test]
fn selecting_schedule_in_root_candidates_opens_management_with_enter() {
    let (_home, mut s) = fixture(vec![]);
    s.paste("//");
    let index = s
        .search_commands()
        .unwrap()
        .iter()
        .position(|command| command.action == Action::Schedule)
        .unwrap();
    for _ in 0..index {
        key(&mut s, KeyCode::Down);
    }
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.page, Page::Configuration(Configuration::Schedule));
    assert!(matches!(s.mode, Mode::Browsing));
    assert!(s.bar.editor().text.is_empty());
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.page, Page::Repositories);
    s.paste("//sche");
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.page, Page::Configuration(Configuration::Schedule));
    assert!(s.service.store.report().unwrap().is_none());
}

#[test]
fn schedule_dashboard_shows_state_pending_interval_and_the_correct_toggle_action() {
    use engine::scheduler::Control;
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        for (registration, schedule, expected_status, expected_control) in [
            (
                Registration::Loaded,
                Schedule::Saved {
                    interval_seconds: 21600,
                },
                i18n::tr!(TaskEnabled),
                Control::Disable,
            ),
            (
                Registration::NotLoaded,
                Schedule::Saved {
                    interval_seconds: 21600,
                },
                i18n::tr!(TaskDisabled),
                Control::Enable,
            ),
            (
                Registration::NotLoaded,
                Schedule::Missing,
                i18n::tr!(TaskMissing),
                Control::Enable,
            ),
        ] {
            let (_home, mut s) = fixture(vec![]);
            s.snapshot.registration = registration;
            s.snapshot.schedule = schedule;
            command(&mut s, "schedule");
            s.config.interval_seconds = 43200;
            for (width, height) in [(60, 18), (100, 28), (150, 32)] {
                let text = render(&mut s, width, height);
                assert!(text.contains(expected_status), "{text}");
                assert!(text.contains(i18n::tr!(IntervalLabel)));
                let application = if matches!(s.snapshot.schedule, Schedule::Missing) {
                    i18n::tr!(IntervalUninstalled).to_owned()
                } else {
                    i18n::tf!(IntervalPending, "6h")
                };
                assert!(text.contains(&application));
            }
            let text = render(&mut s, 60, 18);
            assert!(text.contains(match expected_control {
                Control::Enable => i18n::tr!(ScheduleEnableButton),
                Control::Disable => i18n::tr!(ScheduleDisableButton),
                _ => unreachable!(),
            }));
            key(&mut s, KeyCode::Enter);
            assert!(
                matches!(&s.mode, Mode::Confirm { operation: Confirmation::Schedule(control), .. } if *control == expected_control)
            );
            key(&mut s, KeyCode::Esc);
            assert_eq!(s.page, Page::Configuration(Configuration::Schedule));
            assert!(!s.service.scheduler.definition().exists());
        }
    }
}

#[test]
fn parameter_candidates_show_usage_and_complete_inline_without_changing_the_page() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        command(&mut s, "repos manage");
        s.paste("//repos a");
        for (width, height) in [(60, 18), (100, 28), (150, 32)] {
            let text = render(&mut s, width, height);
            assert!(text.contains("/repos add <path>"), "{text}");
        }
        key(&mut s, KeyCode::Enter);
        assert_eq!(s.bar.text(), "/repos add ");
        assert!(matches!(s.mode, Mode::Management(_)));
        assert_eq!(s.page, Page::Configuration(Configuration::Repositories));
        key(&mut s, KeyCode::Enter);
        assert!(!s.message.is_empty());
        assert!(matches!(s.mode, Mode::Management(_)));
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.page, Page::Configuration(Configuration::Repositories));
        assert!(s.bar.editor().text.is_empty());
    }
}

#[test]
fn inline_add_treats_quoted_unicode_spaces_and_metacharacters_as_one_literal_path() {
    let mut f = support::Fixture::create();
    let renamed = f.temp.path().join("仓库 [x] $(literal) with spaces");
    std::fs::rename(&f.repo, &renamed).unwrap();
    f.repo = std::fs::canonicalize(renamed).unwrap();
    let (_home, mut s) = fixture(vec![]);
    command(&mut s, "repos manage");
    let input = format!("repos add \"{}\"", f.repo.display());
    command(&mut s, &input);
    assert!(
        matches!(&s.mode, Mode::Confirm { operation: Confirmation::Configure { command: command::ConfigArgument::Add, value, .. }, .. } if value == &f.repo.to_string_lossy())
    );
    assert!(s.config.repositories.is_empty());
    key(&mut s, KeyCode::Esc);
    assert!(matches!(s.mode, Mode::Management(_)));
    assert_eq!(s.bar.text(), format!("/{input}"));
    assert_eq!(s.page, Page::Configuration(Configuration::Repositories));
    key(&mut s, KeyCode::Enter);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(s.service.store.config().unwrap().repositories, vec![f.repo]);
    assert!(s.service.store.report().unwrap().is_none());
}

#[test]
fn invalid_or_missing_inline_arguments_keep_the_command_and_configuration_unchanged() {
    let (_home, mut s) = fixture(vec![]);
    for input in [
        "repos add ",
        "repos add relative",
        "repos add ~someone/Proj",
        "repos add \"/unterminated",
        "schedule interval zero",
        "schedule interval 0s",
        "schedule autopush invalid",
        "settings concurrency 0",
        "settings concurrency 33",
    ] {
        command(&mut s, input);
        assert!(matches!(s.mode, Mode::Browsing), "{input}");
        assert_eq!(s.bar.text(), format!("/{input}"));
        assert!(!s.message.is_empty(), "{input}");
    }
    let config = s.service.store.config().unwrap();
    assert_eq!(config.interval_seconds, 21600);
    assert_eq!(config.max_concurrency, 4);
    assert_eq!(config.scheduled_actions.push_policy(), PushPolicy::Disabled);
    assert!(config.repositories.is_empty());
}

#[test]
fn dashboard_editing_and_confirmation_share_the_inline_command_draft() {
    let (_home, mut s) = fixture(vec![]);
    command(&mut s, "schedule");
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.bar.text(), "/schedule interval 6h");
    assert_eq!(s.page, Page::Configuration(Configuration::Schedule));
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.page, Page::Configuration(Configuration::Schedule));
    command(&mut s, "schedule interval 12h");
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.bar.text(), "/schedule interval 12h");
    key(&mut s, KeyCode::Tab);
    assert_eq!(s.bar.text(), "/schedule interval 12h");
    key(&mut s, KeyCode::Enter);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(s.service.store.config().unwrap().interval_seconds, 43200);
    assert_eq!(s.page, Page::Configuration(Configuration::Schedule));
    assert!(s.service.store.report().unwrap().is_none());
}

#[test]
fn cancelling_an_inline_parameter_confirmation_restores_the_file_panel_and_command() {
    let f = support::Fixture::create();
    support::write(f.repo.join("new"), "content");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    open_files(&mut s);
    wait(&mut s);
    assert!(matches!(s.mode, Mode::Files(_)));
    command(&mut s, "settings concurrency 8");
    assert!(matches!(s.mode, Mode::Confirm { .. }));
    key(&mut s, KeyCode::Esc);
    assert!(matches!(s.mode, Mode::Files(_)));
    assert_eq!(s.bar.text(), "/settings concurrency 8");
    key(&mut s, KeyCode::Esc);
    assert!(matches!(s.mode, Mode::Files(_)));
    assert!(s.bar.editor().text.is_empty());
    assert_eq!(s.service.store.config().unwrap().max_concurrency, 4);
}
