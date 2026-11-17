use super::*;
use crate::command::completion::Menu;

fn history(s: &Screen) -> &command::history::Session {
    let bar::InputBar::Command {
        menu: Menu::History(history),
        ..
    } = &s.bar
    else {
        panic!("history menu expected");
    };
    history
}
fn open(s: &mut Screen) {
    while matches!(s.bar, bar::InputBar::Command { .. }) {
        key(s, KeyCode::Esc);
    }
    s.paste("//");
    key(s, KeyCode::Up);
}

#[test]
fn history_entry_preserves_plain_list_navigation_and_only_opens_at_the_empty_command_first_row() {
    let (_home, mut s) = fixture(vec!["/example/alpha".into(), "/example/beta".into()]);
    key(&mut s, KeyCode::Down);
    let selected = s.repository_viewport.selected();
    key(&mut s, KeyCode::Up);
    assert_ne!(s.repository_viewport.selected(), selected);
    assert!(s.bar.is_plain());
    s.paste("/");
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Up);
    assert!(matches!(
        s.bar,
        bar::InputBar::Command {
            menu: Menu::Commands,
            selected: 0,
            ..
        }
    ));
    key(&mut s, KeyCode::Up);
    assert!(history(&s).entries.is_empty());
    let text = render(&mut s, 80, 24);
    assert!(text.contains(i18n::tr!(CommandHistoryEmpty)), "{text}");
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.bar.text(), "/");
    key(&mut s, KeyCode::Char('r'));
    key(&mut s, KeyCode::Up);
    assert!(matches!(
        s.bar,
        bar::InputBar::Command {
            menu: Menu::Commands,
            ..
        }
    ));
    s.shutdown();
}

#[test]
fn history_records_submissions_with_options_and_fills_without_executing() {
    let (home, mut s) = fixture(vec![]);
    s.paste("/repos filter");
    key(&mut s, KeyCode::Tab);
    assert!(s.service.store.command_history().unwrap().is_empty());
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Esc);
    command(&mut s, "repos filter problems");
    assert_eq!(s.filter, Filter::Problems);
    command(&mut s, "repos filter all");
    assert_eq!(s.filter, Filter::All);
    open(&mut s);
    assert_eq!(
        history(&s).entries,
        ["/repos filter problems", "/repos filter all"]
    );
    assert_eq!(history(&s).selected, 1);
    let text = render(&mut s, 100, 28);
    let latest = s
        .command_items
        .iter()
        .find(|(_, index)| *index == 1)
        .unwrap()
        .0;
    let older = s
        .command_items
        .iter()
        .find(|(_, index)| *index == 0)
        .unwrap()
        .0;
    assert_eq!(latest.y, older.y + 1);
    assert_eq!(latest.bottom(), s.input_area.unwrap().y);
    assert!(
        text.lines()
            .nth(latest.y as usize)
            .unwrap()
            .contains("/repos filter all")
    );
    key(&mut s, KeyCode::Up);
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.filter, Filter::All);
    assert_eq!(s.bar.text(), "/repos filter problems");
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.filter, Filter::Problems);
    assert_eq!(
        Store::custom(home.path().into()).command_history().unwrap(),
        ["/repos filter problems", "/repos filter all"]
    );
    open(&mut s);
    key(&mut s, KeyCode::Up);
    key(&mut s, KeyCode::Tab);
    assert_eq!(s.bar.text(), "/repos filter all");
    assert_eq!(s.filter, Filter::Problems);
    key(&mut s, KeyCode::Esc);
    open(&mut s);
    render(&mut s, 100, 28);
    let (area, _) = s
        .command_items
        .iter()
        .find(|(_, index)| *index == 0)
        .copied()
        .unwrap();
    s.mouse(
        crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: area.x,
            row: area.y,
            modifiers: KeyModifiers::NONE,
        },
        &Cancel::default(),
    )
    .unwrap();
    assert_eq!(s.bar.text(), "/repos filter all");
    assert_eq!(s.filter, Filter::Problems);
    s.shutdown();
}

#[test]
fn history_is_scrollable_resizable_and_does_not_replace_the_current_draft_on_escape_or_typing() {
    let (_home, mut s) = fixture(vec![]);
    for name in [
        "/repos",
        "/settings",
        "/tasks",
        "/help",
        "/repos filter problems",
        "/repos filter all",
        "/repos sort name",
        "/repos sort status",
        "/settings ai",
        "/settings startup-fetch true",
    ] {
        s.service.store.record_command(name).unwrap();
    }
    open(&mut s);
    render(&mut s, 100, 28);
    assert_eq!(history(&s).selected, 9);
    let page = s.command_items.len();
    key(&mut s, KeyCode::PageUp);
    assert_eq!(history(&s).selected, 9 - page);
    render(&mut s, 100, 28);
    key(&mut s, KeyCode::PageDown);
    assert_eq!(history(&s).selected, 9);
    key(&mut s, KeyCode::Down);
    assert_eq!(history(&s).selected, 9);
    for _ in 0..20 {
        key(&mut s, KeyCode::Up);
    }
    assert_eq!(history(&s).selected, 0);
    key(&mut s, KeyCode::Up);
    assert_eq!(history(&s).selected, 0);
    for (w, h) in [(100, 28), (60, 14), (20, 8), (1, 1), (0, 0), (100, 28)] {
        render(&mut s, w, h);
        if w == 100 {
            assert!(
                s.command_items
                    .iter()
                    .any(|(_, index)| *index == history(&s).selected)
            );
        }
    }
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.bar.text(), "/");
    key(&mut s, KeyCode::Up);
    key(&mut s, KeyCode::Char('r'));
    assert_eq!(s.bar.text(), "/r");
    assert!(matches!(
        s.bar,
        bar::InputBar::Command {
            menu: Menu::Commands,
            ..
        }
    ));
    s.shutdown();
}

#[test]
fn history_retains_the_newly_captured_repository_and_hides_disabled_features() {
    let first = support::Fixture::create();
    let second = support::Fixture::create();
    first.commit("one.txt", "one\n");
    second.commit("two.txt", "two\n");
    let (_home, mut s) = fixture(vec![first.repo.clone(), second.repo.clone()]);
    s.service
        .store
        .record_command("/repo branch create history-test")
        .unwrap();
    s.service.store.record_command("/repo compose ai").unwrap();
    s.service
        .store
        .record_command("/repo commit ai generate")
        .unwrap();
    s.config.ai.enabled = false;
    s.focus(Some(first.repo.clone()));
    s.paste("/");
    s.focus(Some(second.repo.clone()));
    key(&mut s, KeyCode::Up);
    assert_eq!(history(&s).entries, ["/repo branch create history-test"]);
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.bar.text(), "/repo branch create history-test");
    assert!(
        matches!(&s.bar, bar::InputBar::Command { path: Some(path), .. } if path == &first.repo)
    );
    assert!(matches!(s.mode, Mode::Browsing));
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(
        matches!(&s.mode, Mode::Confirm { operation: Confirmation::Mutation { plan, .. }, .. } if plan.path() == first.repo)
    );
    s.shutdown();
}

#[test]
fn invalid_commands_are_not_recorded_and_corrupt_history_is_visible_without_destroying_it() {
    let (_home, mut s) = fixture(vec![]);
    command(&mut s, "not-a-command");
    assert!(s.service.store.command_history().unwrap().is_empty());
    key(&mut s, KeyCode::Esc);
    s.service.store.record_command("/repos").unwrap();
    let path = s.service.store.state_dir().join("command-history.json");
    std::fs::write(&path, "broken history").unwrap();
    s.paste("/");
    key(&mut s, KeyCode::Up);
    assert!(
        s.message.contains(
            i18n::tr!(ReadCommandHistoryFailed)
                .split('{')
                .next()
                .unwrap()
                .trim_end_matches('：')
        )
    );
    assert!(matches!(
        s.bar,
        bar::InputBar::Command {
            menu: Menu::Commands,
            ..
        }
    ));
    assert_eq!(std::fs::read_to_string(path).unwrap(), "broken history");
    s.shutdown();
}

#[test]
fn context_prefixes_keep_up_in_command_completion_and_explicit_global_slash_opens_history() {
    use command::{Context, context::DiffContext};
    let (_home, mut s) = fixture(vec![]);
    s.service.store.record_command("/repos").unwrap();
    for context in [
        Context::Repository,
        Context::Repositories,
        Context::Settings,
        Context::Ai,
        Context::Schedule,
        Context::Branches,
        Context::History,
        Context::Stash,
        Context::Files,
        Context::Composition,
        Context::Diff(DiffContext::Worktree),
        Context::Diff(DiffContext::History),
    ] {
        open(&mut s);
        key(&mut s, KeyCode::Esc);
        let bar::InputBar::Command { input, .. } = &mut s.bar else {
            panic!();
        };
        *input = command::Input::new(Some(context));
        let text = s.bar.text();
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Up);
        key(&mut s, KeyCode::Up);
        assert!(
            matches!(
                s.bar,
                bar::InputBar::Command {
                    menu: Menu::Commands,
                    selected: 0,
                    ..
                }
            ),
            "{context:?}"
        );
        assert_eq!(s.bar.text(), text);
        let rendered = render(&mut s, 120, 28);
        assert!(!rendered.contains(i18n::tr!(CommandHistory)), "{rendered}");
        key(&mut s, KeyCode::Char('/'));
        assert_eq!(s.bar.text(), "/");
        key(&mut s, KeyCode::Up);
        assert_eq!(history(&s).entries, ["/repos"]);
    }
    key(&mut s, KeyCode::Esc);
    s.paste("repo branch ");
    let text = s.bar.text();
    key(&mut s, KeyCode::Up);
    assert!(matches!(
        s.bar,
        bar::InputBar::Command {
            menu: Menu::Commands,
            ..
        }
    ));
    assert_eq!(s.bar.text(), text);
    s.shutdown();
}
