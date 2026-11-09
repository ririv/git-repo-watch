use super::*;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use engine::operations::replay::{Control, Kind};

fn reason(s: &Screen, action: Action) -> Option<String> {
    let bar::InputBar::Command {
        path, selection, ..
    } = &s.bar
    else {
        panic!("command draft expected")
    };
    s.command_availability(action, path.as_deref(), selection)
        .reason()
        .map(str::to_owned)
}
fn names(s: &Screen) -> Vec<&'static str> {
    s.search_commands()
        .unwrap()
        .into_iter()
        .map(|command| command.name)
        .collect()
}
fn click(s: &mut Screen, action: Action) {
    render(s, 160, 32);
    let commands = s.search_commands().unwrap();
    let (area, _) = s
        .command_items
        .iter()
        .find(|(_, index)| commands[*index].action == action)
        .unwrap();
    let point = *area;
    assert!(
        s.mouse(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: point.x,
                row: point.y,
                modifiers: KeyModifiers::NONE,
            },
            &Cancel::default()
        )
        .unwrap()
    );
}

#[test]
fn empty_history_keeps_its_commands_disabled_then_enables_them_after_a_commit() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let root = tempfile::tempdir().unwrap();
        let repo = root.path().join("empty");
        support::init(&repo);
        let repo = repo.canonicalize().unwrap();
        let (_home, mut s) = fixture(vec![repo.clone()]);
        command(&mut s, "repo log");
        wait(&mut s);
        s.paste("/");
        let before = names(&s);
        for name in ["repo log cherry-pick", "repo log revert", "repo log select"] {
            assert!(before.contains(&name), "{before:?}");
        }
        let action = Action::Replay(Kind::CherryPick);
        assert_eq!(
            reason(&s, action).as_deref(),
            Some(i18n::tr!(ReplaySelectCommit))
        );
        let draft = s.bar.text();
        let rendered = render(&mut s, 160, 32);
        assert!(
            rendered.contains(i18n::tr!(ReplaySelectCommit)),
            "{rendered}"
        );
        click(&mut s, action);
        assert_eq!(s.bar.text(), draft);
        key(&mut s, KeyCode::Enter);
        assert_eq!(s.bar.text(), draft);
        assert!(s.repository_tasks.is_empty());
        assert!(!repo.join(".git/index").exists());
        key(&mut s, KeyCode::Esc);
        support::git(&repo, &["commit", "--allow-empty", "-m", "first"]);
        command(&mut s, "refresh");
        wait(&mut s);
        s.paste("/");
        assert_eq!(names(&s), before);
        assert!(reason(&s, action).is_none());
        assert!(reason(&s, Action::Copy).is_none());
        s.shutdown();
    }
}

#[test]
fn repository_commands_require_a_target_from_the_current_view() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    s.paste("//repo fetch");
    assert!(reason(&s, Action::Fetch).is_none());
    key(&mut s, KeyCode::Esc);
    command(&mut s, "repo");
    wait(&mut s);
    s.paste("//repo fetch");
    assert!(reason(&s, Action::Fetch).is_none());
    key(&mut s, KeyCode::Esc);

    command(&mut s, "settings");
    command(&mut s, "repo fetch");
    assert_eq!(s.message, i18n::tr!(SelectARepositoryFirst));
    assert_eq!(s.page, Page::Configuration(Configuration::General));
    assert!(s.repository_tasks.is_empty());
    s.shutdown();
}

#[test]
fn idle_repository_recovery_commands_stay_visible_and_never_start_a_task() {
    let f = support::Fixture::create();
    let head = support::git(&f.repo, &["rev-parse", "HEAD"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    for (control, name) in [
        (Control::Continue, "continue"),
        (Control::Skip, "skip"),
        (Control::Abort, "abort"),
    ] {
        s.paste(&format!("//repo {name}"));
        assert_eq!(names(&s), [format!("repo {name}")]);
        let action = Action::ReplayControl(control);
        assert_eq!(
            reason(&s, action).as_deref(),
            Some(i18n::tr!(ReplayNotActive))
        );
        let draft = s.bar.text();
        let mut terminal = Terminal::new(TestBackend::new(140, 28)).unwrap();
        terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
        let first = s.command_items[0].0;
        assert_eq!(
            terminal.backend().buffer()[(first.x, first.y)].fg,
            view::MUTED
        );
        key(&mut s, KeyCode::Enter);
        assert_eq!(s.bar.text(), draft);
        click(&mut s, action);
        assert_eq!(s.bar.text(), draft);
        assert!(s.repository_tasks.is_empty());
        key(&mut s, KeyCode::Esc);
    }
    assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), head);
    s.shutdown();
}

#[test]
fn binary_selection_options_remain_visible_and_cannot_change_the_index() {
    let f = support::Fixture::create();
    f.commit("binary.bin", "\0old");
    f.commit("editable.txt", "old\n");
    support::write(f.repo.join("binary.bin"), "\0new");
    support::write(f.repo.join("editable.txt"), "new\n");
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    s.paste("binary");
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    s.paste("//repo diff select ");
    let draft = s.bar.text();
    assert!(reason(&s, Action::PartialSelectMenu).is_some());
    let mut terminal = Terminal::new(TestBackend::new(160, 32)).unwrap();
    terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
    assert_eq!(s.command_items.len(), 2);
    let first = s.command_items[0].0;
    assert_eq!(
        terminal.backend().buffer()[(first.x, first.y)].fg,
        view::MUTED
    );
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.bar.text(), draft);
    assert!(matches!(
        s.bar,
        bar::InputBar::Command {
            menu: command::completion::Menu::Options(_),
            ..
        }
    ));
    s.mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: first.x,
            row: first.y,
            modifiers: KeyModifiers::NONE,
        },
        &Cancel::default(),
    )
    .unwrap();
    assert_eq!(s.bar.text(), draft);
    assert!(s.repository_tasks.is_empty());
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    for _ in 0..3 {
        key(&mut s, KeyCode::Esc);
    }
    s.key(
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
        &Cancel::default(),
    )
    .unwrap();
    s.paste("editable");
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    s.paste("//repo diff select ");
    assert!(reason(&s, Action::PartialSelectMenu).is_none());
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    s.shutdown();
}

#[test]
fn source_and_preview_share_one_parameter_menu_and_disable_only_the_active_mode() {
    use super::browser::{ready, select};
    use crate::{
        browser::opening::TextMode,
        command::{completion::Menu, options::Value},
    };
    let f = support::Fixture::create();
    f.commit("README.md", "# Rendered\n");
    f.commit("plain.txt", "plain\n");
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo files");
    ready(&mut s);
    select(&mut s, "README.md");
    s.paste("/");
    let before = names(&s);
    assert!(before.contains(&"repo files mode"));
    assert!(
        !command::COMMANDS
            .iter()
            .any(|entry| ["repo files source", "repo files preview"].contains(&entry.name))
    );
    assert_eq!(command::matches("repo files mode").len(), 1);
    assert_eq!(
        command::matches("repo files mode")[0].usage(),
        "repo files mode [source|preview]"
    );
    command(&mut s, "repo files mode");
    let bar::InputBar::Command {
        menu: Menu::Options(values),
        ..
    } = &s.bar
    else {
        panic!("mode options")
    };
    assert_eq!(
        values
            .options
            .iter()
            .map(|option| option.token)
            .collect::<Vec<_>>(),
        ["source", "preview"]
    );
    assert_eq!(values.current, Some(Value::FileMode(TextMode::Source)));
    assert_eq!(
        reason(&s, Action::BrowserMode(TextMode::Source)).as_deref(),
        Some(i18n::tr!(CommandSourceCurrent))
    );
    assert!(reason(&s, Action::BrowserMode(TextMode::Preview)).is_none());
    let rendered = render(&mut s, 160, 32);
    assert!(rendered.contains(i18n::CommandSummary::BrowserMode.text()));
    let first = s.command_items[0].0;
    let draft = s.bar.text();
    s.mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: first.x,
            row: first.y,
            modifiers: KeyModifiers::NONE,
        },
        &Cancel::default(),
    )
    .unwrap();
    assert_eq!(s.bar.text(), draft);
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Tab);
    assert_eq!(s.bar.text(), "/repo files mode preview");
    assert!(reason(&s, Action::BrowserMode(TextMode::Preview)).is_none());
    assert!(matches!(
        &s.copy_selection().opener().unwrap().presentation,
        crate::browser::opening::Presentation::Single(TextMode::Source)
    ));
    key(&mut s, KeyCode::Enter);
    ready(&mut s);
    s.paste("/");
    assert_eq!(names(&s), before);
    s.paste("mode ");
    let bar::InputBar::Command {
        menu: Menu::Options(values),
        ..
    } = &s.bar
    else {
        panic!("mode options")
    };
    assert_eq!(values.current, Some(Value::FileMode(TextMode::Preview)));
    assert!(reason(&s, Action::BrowserMode(TextMode::Source)).is_none());
    assert_eq!(
        reason(&s, Action::BrowserMode(TextMode::Preview)).as_deref(),
        Some(i18n::tr!(CommandPreviewCurrent))
    );
    while !s.bar.is_plain() {
        key(&mut s, KeyCode::Esc);
    }
    select(&mut s, "plain.txt");
    s.paste("/");
    assert!(
        !names(&s)
            .iter()
            .any(|name| ["repo files mode", "repo files layout"].contains(name))
    );
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    s.shutdown();
}
