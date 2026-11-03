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
        for name in [
            "repo log cherry-pick",
            "repo log revert",
            "repo log select",
            "repo log copy",
        ] {
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
        command(&mut s, "repo log refresh");
        wait(&mut s);
        s.paste("/");
        assert_eq!(names(&s), before);
        assert!(reason(&s, action).is_none());
        assert!(reason(&s, Action::Copy(crate::clipboard::Scope::Log)).is_none());
        s.shutdown();
    }
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
fn source_and_preview_choices_keep_their_positions_and_disable_only_the_active_mode() {
    use super::browser::{ready, select};
    let f = support::Fixture::create();
    f.commit("README.md", "# Rendered\n");
    f.commit("plain.txt", "plain\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo files");
    ready(&mut s);
    select(&mut s, "README.md");
    s.paste("/");
    let before = names(&s);
    assert!(before.contains(&"repo files source") && before.contains(&"repo files preview"));
    assert_eq!(
        reason(&s, Action::BrowserSource).as_deref(),
        Some(i18n::tr!(CommandSourceCurrent))
    );
    assert!(reason(&s, Action::BrowserPreview).is_none());
    let draft = s.bar.text();
    click(&mut s, Action::BrowserSource);
    assert_eq!(s.bar.text(), draft);
    key(&mut s, KeyCode::Esc);
    command(&mut s, "repo files preview");
    ready(&mut s);
    s.paste("/");
    assert_eq!(names(&s), before);
    assert!(reason(&s, Action::BrowserSource).is_none());
    assert_eq!(
        reason(&s, Action::BrowserPreview).as_deref(),
        Some(i18n::tr!(CommandPreviewCurrent))
    );
    key(&mut s, KeyCode::Esc);
    select(&mut s, "plain.txt");
    s.paste("/");
    assert!(!names(&s).iter().any(|name| {
        [
            "repo files source",
            "repo files preview",
            "repo files toggle",
            "repo files split",
        ]
        .contains(name)
    }));
    s.shutdown();
}
