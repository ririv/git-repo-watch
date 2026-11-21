use super::*;
use crate::command::{availability::Availability, completion::Menu, target::Target};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

fn candidates(s: &Screen) -> Vec<Action> {
    s.search_commands()
        .unwrap()
        .iter()
        .map(|command| command.action)
        .collect()
}

fn availability(s: &Screen, action: Action) -> Availability {
    let bar::InputBar::Command {
        path, selection, ..
    } = &s.bar
    else {
        panic!("command draft expected")
    };
    s.command_availability(action, path.as_deref(), selection)
}

#[test]
fn contextual_menus_hide_unrelated_targets_but_global_search_explains_them_without_executing() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        let head = support::git(&f.repo, &["rev-parse", "HEAD"]);
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        s.paste("/");
        assert!(!candidates(&s).contains(&Action::LogSelect));
        assert!(!candidates(&s).contains(&Action::BrowserDiff));
        key(&mut s, KeyCode::Esc);
        key(&mut s, KeyCode::Enter);
        wait_workspace(&mut s);
        s.paste("/");
        let context = candidates(&s);
        for action in [
            Action::Log,
            Action::Branches,
            Action::BrowseFiles,
            Action::Diff,
        ] {
            assert!(context.contains(&action));
        }
        let unrelated = [
            ("repo branch filter", Action::BranchFilterMenu),
            ("repo log more", Action::LogMore),
            ("repo log select", Action::LogSelect),
            ("repo log select all", Action::LogSelectAll),
            ("repo log select clear", Action::LogSelectClear),
            (
                "repo log cherry-pick",
                Action::Replay(engine::operations::replay::Kind::CherryPick),
            ),
            (
                "repo log revert",
                Action::Replay(engine::operations::replay::Kind::Revert),
            ),
            ("repo log parent", Action::LogParent),
            ("repo files diff", Action::BrowserDiff),
            ("repo files mode", Action::BrowserModeMenu),
            ("repo files layout", Action::BrowserLayoutMenu),
            ("repo stash scope", Action::StashPreviewMenu),
            (
                "repo compose commit",
                Action::Compose(crate::composition::Action::Next),
            ),
            ("repo compose reorder", Action::ComposeOrderMenu),
            ("repo diff stage", Action::Stage),
            ("repo diff unstage", Action::Unstage),
            ("repo diff select", Action::PartialSelectMenu),
            ("repo diff discard", Action::Discard),
            ("repo diff discard-file", Action::DiscardFile),
            ("repo diff conflict", Action::ConflictMenu),
            ("copy path", Action::CopyPath),
        ];
        for (_, action) in unrelated {
            assert!(!context.contains(&action), "{action:?}");
        }
        key(&mut s, KeyCode::Esc);
        for (name, action) in unrelated {
            s.paste(&format!("//{name}"));
            let index = candidates(&s)
                .iter()
                .position(|candidate| *candidate == action)
                .unwrap();
            if let bar::InputBar::Command { selected, .. } = &mut s.bar {
                *selected = index;
            }
            let Availability::Target(Target::OutOfScope(reason)) = availability(&s, action) else {
                panic!("out of scope: {action:?}")
            };
            let draft = s.bar.text();
            let mut terminal = Terminal::new(TestBackend::new(160, 32)).unwrap();
            terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
            let area = s
                .command_items
                .iter()
                .find(|(_, item)| *item == index)
                .unwrap()
                .0;
            assert_eq!(
                terminal.backend().buffer()[(area.x, area.y)].fg,
                view::MUTED
            );
            key(&mut s, KeyCode::Enter);
            assert_eq!(s.message, reason.text());
            assert_eq!(s.bar.text(), draft);
            s.mouse(
                MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: area.x,
                    row: area.y,
                    modifiers: KeyModifiers::NONE,
                },
                &Cancel::default(),
            )
            .unwrap();
            assert_eq!(s.message, reason.text());
            assert_eq!(s.bar.text(), draft);
            assert_eq!(s.page, Page::Detail);
            assert!(matches!(s.mode, Mode::Browsing));
            assert!(s.repository_tasks.is_empty());
            key(&mut s, KeyCode::Esc);
        }
        assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), head);
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        s.shutdown();
    }
}

#[test]
fn globally_searching_an_out_of_scope_option_command_does_not_open_a_parameter_menu() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    for (name, action, reason) in [
        (
            "repo branch filter",
            Action::BranchFilterMenu,
            i18n::tr!(BranchFilterRequiresView),
        ),
        (
            "repo diff select",
            Action::PartialSelectMenu,
            i18n::tr!(CommandRequiresWorktreePreview),
        ),
        (
            "repo compose reorder",
            Action::ComposeOrderMenu,
            i18n::tr!(CommandRequiresComposition),
        ),
        (
            "repo stash scope",
            Action::StashPreviewMenu,
            i18n::tr!(StashPreviewRequiresDetail),
        ),
        (
            "repo files mode",
            Action::BrowserModeMenu,
            i18n::tr!(CommandRequiresPreviewableFile),
        ),
    ] {
        s.paste(&format!("//{name} "));
        assert!(candidates(&s).contains(&action));
        assert!(matches!(
            s.bar,
            bar::InputBar::Command {
                menu: Menu::Commands,
                ..
            }
        ));
        key(&mut s, KeyCode::Enter);
        assert_eq!(s.message, reason);
        assert!(matches!(s.mode, Mode::Browsing));
        assert!(s.repository_tasks.is_empty());
        key(&mut s, KeyCode::Esc);
    }
    s.shutdown();
}

#[test]
fn disabled_features_stay_hidden_from_global_search_and_cannot_execute() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    for (ai, experimental) in [(false, true), (true, false)] {
        s.config.ai.enabled = ai;
        s.config.experimental_features = experimental;
        let disabled = command::COMMANDS
            .iter()
            .filter(|command| !command.action.available(&s.config))
            .collect::<Vec<_>>();
        for command in disabled {
            s.paste(&format!("//{}", command.name));
            assert!(
                !candidates(&s).contains(&command.action),
                "{}",
                command.name
            );
            assert!(matches!(
                availability(&s, command.action),
                Availability::Hidden
            ));
            assert!(
                !s.activate(
                    command.action,
                    Some(f.repo.clone()),
                    s.copy_selection(),
                    &Cancel::default()
                )
                .unwrap()
            );
            assert!(s.repository_tasks.is_empty());
            key(&mut s, KeyCode::Esc);
        }
    }
    s.shutdown();
}

#[test]
fn related_commands_stay_visible_when_only_the_selection_or_history_limit_is_missing() {
    use super::browser::{ready, select};
    let f = support::Fixture::create();
    f.commit("clean.txt", "unchanged\n");
    f.commit("changed.txt", "before\n");
    support::write(f.repo.join("changed.txt"), "after\n");
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo files");
    ready(&mut s);
    select(&mut s, "clean.txt");
    s.paste("/");
    assert!(candidates(&s).contains(&Action::BrowserDiff));
    assert!(matches!(
        availability(&s, Action::BrowserDiff),
        Availability::Target(Target::Unavailable(_))
    ));
    key(&mut s, KeyCode::Esc);
    command(&mut s, "repo files diff");
    assert_eq!(s.message, i18n::tr!(BrowserNoDiff));
    assert!(matches!(s.mode, Mode::Browser(_)));
    assert!(s.repository_tasks.is_empty());
    key(&mut s, KeyCode::Esc);
    select(&mut s, "changed.txt");
    s.paste("/");
    assert!(matches!(
        availability(&s, Action::BrowserDiff),
        Availability::Target(Target::Ready(_))
    ));
    command(&mut s, "repo log");
    wait(&mut s);
    s.paste("/");
    assert!(candidates(&s).contains(&Action::LogMore));
    assert_eq!(
        availability(&s, Action::LogMore).reason(),
        Some(i18n::tr!(HistoryNoMore))
    );
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    s.paste("/");
    assert!(candidates(&s).contains(&Action::LogParent));
    assert_eq!(
        availability(&s, Action::LogParent).reason(),
        Some(i18n::tr!(HistoryParentRequiresMerge))
    );
    command(&mut s, "repo diff staged");
    wait(&mut s);
    s.paste("/");
    assert!(candidates(&s).contains(&Action::CopyPath));
    assert!(candidates(&s).contains(&Action::Unstage));
    assert!(matches!(
        availability(&s, Action::CopyPath),
        Availability::Target(Target::Unavailable(_))
    ));
    assert!(matches!(
        availability(&s, Action::Unstage),
        Availability::Target(Target::Unavailable(_))
    ));
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    s.shutdown();
}
