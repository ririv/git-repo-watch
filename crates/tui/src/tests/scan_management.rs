use super::*;
use crate::management::{Selection, Source, Status};
use std::path::Path;

fn repository(root: &Path, relative: &str) -> PathBuf {
    let path = root.join(relative);
    support::init(&path);
    std::fs::canonicalize(path).unwrap()
}
fn session(s: &mut Screen) -> &mut crate::management::Session {
    let Mode::Management(session) = &mut s.mode else {
        panic!("registration view");
    };
    session
}

#[test]
fn scan_reviews_before_adding_and_exclusions_do_not_survive_the_request() {
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let projects = tempfile::tempdir().unwrap();
        let root = std::fs::canonicalize(projects.path())
            .unwrap()
            .join("项目 目录");
        let managed = repository(&root, "managed");
        let keep_a = repository(&root, "new-a");
        let keep_b = repository(&root, "new-b");
        let backup = repository(&root, "backup/old");
        let archive = repository(&root, "中文 归档/old");
        std::fs::create_dir_all(root.join("broken")).unwrap();
        support::write(
            root.join("broken/.git"),
            "gitdir: /missing/gw-discovery-test\n",
        );
        let (_home, mut s) = fixture(vec![managed.clone()]);
        let before = std::fs::read(s.service.store.config_path()).unwrap();
        command(
            &mut s,
            &format!(
                "repos scan \"{}\" --exclude backup \"中文 归档\"",
                root.display()
            ),
        );
        wait(&mut s);
        assert_eq!(s.footer_page_number(), Some((2, 2)));
        key(&mut s, KeyCode::BackTab);
        let review = session(&mut s);
        assert!(matches!(&review.source, Source::Scan { excluded, .. } if excluded.len() == 2));
        assert_eq!(review.entries.len(), 4);
        assert_eq!(review.selection().len(), 2);
        assert!(
            review
                .entries
                .iter()
                .any(|entry| matches!(entry.status, Status::Failed(_)))
        );
        let index = review
            .rows()
            .iter()
            .position(|&i| review.entries[i].path == keep_a)
            .unwrap();
        review.state.select(Some(index));
        key(&mut s, KeyCode::Char(' '));
        assert_eq!(
            session(&mut s).selection().as_slice(),
            std::slice::from_ref(&keep_b)
        );
        s.paste("new-b");
        assert_eq!(session(&mut s).rows().len(), 1);
        for count in [0, 1] {
            s.key(
                KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL),
                &Cancel::default(),
            )
            .unwrap();
            assert_eq!(session(&mut s).selection().len(), count);
        }
        assert_eq!(
            std::fs::read(s.service.store.config_path()).unwrap(),
            before
        );
        for (width, height) in [(30, 9), (60, 18), (120, 32)] {
            let text = render(&mut s, width, height);
            assert!(text.contains(i18n::tr!(ScanResults)), "{text}");
            if width >= 60 {
                assert!(text.contains("new-b"), "{text}");
            }
        }
        key(&mut s, KeyCode::BackTab);
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        let paths = s.service.store.config().unwrap().repositories;
        assert_eq!(paths, [managed.clone(), keep_b]);
        assert!(matches!(s.mode, Mode::Management(_)));
        assert!(session(&mut s).selection().is_empty());
        assert!(s.service.store.scan_report().unwrap().is_none());
        command(&mut s, &format!("repos scan \"{}\"", root.display()));
        wait(&mut s);
        assert!(session(&mut s).selection().contains(&backup));
        assert!(session(&mut s).selection().contains(&archive));
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.service.store.config().unwrap().repositories.len(), 2);
        s.shutdown();
    }
}

#[test]
fn management_selection_targets_remove_and_cancel_restores_the_list() {
    let first = support::Fixture::create();
    let second = support::Fixture::create();
    let (_home, mut s) = fixture(vec![first.repo.clone(), second.repo.clone()]);
    command(&mut s, "repos manage");
    key(&mut s, KeyCode::Down);
    assert_eq!(s.scope_path(), Some(second.repo.clone()));
    command(&mut s, "repos remove");
    assert!(
        matches!(&s.mode, Mode::Confirm { operation: Confirmation::Remove { path, .. }, .. } if path == &second.repo)
    );
    key(&mut s, KeyCode::Esc);
    assert!(matches!(s.mode, Mode::Management(_)));
    assert_eq!(s.scope_path(), Some(second.repo.clone()));
    command(&mut s, "repos remove");
    key(&mut s, KeyCode::Left);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(
        s.service.store.config().unwrap().repositories.as_slice(),
        std::slice::from_ref(&first.repo)
    );
    assert_eq!(session(&mut s).entries.len(), 1);
    assert!(second.repo.join(".git").is_dir());
    assert!(command::COMMANDS.iter().all(|command| !matches!(
        command.name,
        "repos exclude" | "repos unexclude" | "repos import"
    )));
    s.shutdown();
}

#[test]
fn scan_checkbox_and_apply_button_respect_precise_click_and_selection() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let projects = tempfile::tempdir().unwrap();
    let path = repository(projects.path(), "new");
    let (_home, mut s) = fixture(vec![]);
    command(
        &mut s,
        &format!("repos scan \"{}\"", projects.path().display()),
    );
    wait(&mut s);
    render(&mut s, 100, 26);
    let position = (0..26)
        .find_map(|y| {
            (0..100).find_map(|x| {
                let point = ratatui::layout::Position::new(x, y);
                (s.pointer.hit(point)
                    == Some(crate::pointer::Hit::Button(
                        crate::pointer::Action::ManagementToggle(0),
                    )))
                .then_some(point)
            })
        })
        .unwrap();
    for enabled in [false, true, true] {
        s.config.mouse.precise_click = enabled;
        s.mouse(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: position.x,
                row: position.y,
                modifiers: KeyModifiers::NONE,
            },
            &Cancel::default(),
        )
        .unwrap();
        if !enabled {
            assert_eq!(
                session(&mut s).selection().as_slice(),
                std::slice::from_ref(&path)
            );
        }
    }
    assert!(matches!(
        session(&mut s).entries[0].status,
        Status::New(Selection::Included)
    ));
    render(&mut s, 100, 26);
    key(&mut s, KeyCode::BackTab);
    render(&mut s, 100, 26);
    let apply = (0..26)
        .find_map(|y| {
            (0..100).find_map(|x| {
                let point = ratatui::layout::Position::new(x, y);
                (s.pointer.hit(point)
                    == Some(crate::pointer::Hit::Button(
                        crate::pointer::Action::Management(crate::management::Action::Apply),
                    )))
                .then_some(point)
            })
        })
        .unwrap();
    s.mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: apply.x,
            row: apply.y,
            modifiers: KeyModifiers::NONE,
        },
        &Cancel::default(),
    )
    .unwrap();
    wait(&mut s);
    assert_eq!(s.service.store.config().unwrap().repositories, [path]);
    s.shutdown();
}

#[test]
fn registration_refresh_preserves_checked_new_entries_and_reflects_external_removals() {
    let projects = tempfile::tempdir().unwrap();
    let first = repository(projects.path(), "first");
    let second = repository(projects.path(), "second");
    let third = repository(projects.path(), "third");
    let report = engine::discovery::discover(projects.path(), &[], &Cancel::default()).unwrap();
    let mut review = crate::management::Session::scanned(report, std::slice::from_ref(&first));
    let third_index = review
        .rows()
        .iter()
        .position(|&i| review.entries[i].path == third)
        .unwrap();
    review.toggle(third_index);
    review.refresh(&[]);
    assert!(matches!(
        review
            .entries
            .iter()
            .find(|entry| entry.path == first)
            .unwrap()
            .status,
        Status::New(Selection::Skipped)
    ));
    assert_eq!(review.selection().as_slice(), std::slice::from_ref(&second));
    review.refresh(std::slice::from_ref(&second));
    assert!(review.selection().is_empty());
    assert!(matches!(
        review
            .entries
            .iter()
            .find(|entry| entry.path == second)
            .unwrap()
            .status,
        Status::Managed
    ));
}

#[test]
fn management_uses_the_normal_command_popup_without_a_separate_menu_focus() {
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec!["/example/first".into(), "/example/second".into()]);
        let before = s.service.store.config().unwrap().repositories;
        command(&mut s, "repos manage");
        for code in [KeyCode::Tab, KeyCode::BackTab, KeyCode::Enter] {
            key(&mut s, code);
            key(&mut s, KeyCode::Down);
            assert_eq!(session(&mut s).state.selected(), Some(1));
            key(&mut s, KeyCode::Up);
            assert_eq!(session(&mut s).state.selected(), Some(0));
        }
        key(&mut s, KeyCode::Down);
        s.paste("/");
        assert!(matches!(s.bar, bar::InputBar::Command { .. }));
        key(&mut s, KeyCode::Esc);
        for name in ["repos add", "repos scan"] {
            command(&mut s, name);
            let bar::InputBar::Command { input, .. } = &s.bar else {
                panic!("command opens its argument editor");
            };
            assert_eq!(input.text(), format!("/{name} "));
            key(&mut s, KeyCode::Esc);
        }
        command(&mut s, "repos remove");
        assert!(
            matches!(&s.mode, Mode::Confirm { operation: Confirmation::Remove { path, .. }, .. } if path == &before[1])
        );
        key(&mut s, KeyCode::Esc);
        assert_eq!(session(&mut s).state.selected(), Some(1));
        for (width, height) in [(28, 9), (28, 18), (90, 18)] {
            render(&mut s, width, height);
        }
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.page, Page::Repositories);
        assert_eq!(s.service.store.config().unwrap().repositories, before);
        s.shutdown();
    }
}

#[test]
fn scan_actions_use_the_bottom_input_slot_and_keep_search_command_and_selection_drafts() {
    use crate::pointer::{Action as PointerAction, Hit};
    use ratatui::layout::Position;
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let projects = tempfile::tempdir().unwrap();
        let path = repository(projects.path(), "keep-me");
        repository(projects.path(), "other");
        let (_home, mut s) = fixture(vec![]);
        command(
            &mut s,
            &format!("repos scan \"{}\"", projects.path().display()),
        );
        wait(&mut s);
        assert_eq!(s.footer_page_number(), Some((2, 2)));
        assert!(matches!(
            session(&mut s).focus,
            crate::management::Focus::Actions(crate::management::Controls {
                selected: crate::management::Action::Apply,
                ..
            })
        ));
        key(&mut s, KeyCode::BackTab);
        let registered = s.config.repositories.clone();
        session(&mut s).refresh(&registered);
        render(&mut s, 120, 32);
        assert_eq!(s.footer_page_number(), Some((1, 2)));
        s.paste("keep");
        let selected = session(&mut s).state.selected();
        for (width, height) in [(28, 9), (60, 18), (120, 32)] {
            let text = render(&mut s, width, height);
            if width >= 60 {
                assert!(text.contains(i18n::tr!(ReplayControlsEntryHint)), "{text}");
            }
            let body = s.body_area;
            let input = s.input_area.unwrap();
            assert_eq!(s.footer_page_number(), Some((1, 2)));
            assert!((0..height).all(|y| (0..width).all(|x| !matches!(
                s.pointer.hit(Position::new(x, y)),
                Some(Hit::Button(PointerAction::Management(_)))
            ))));
            key(&mut s, KeyCode::BackTab);
            let text = render(&mut s, width, height);
            assert_eq!(s.footer_page_number(), Some((2, 2)));
            assert_eq!(s.body_area, body);
            assert_eq!(s.input_area, Some(input));
            assert!(text.contains("2/2"), "{text}");
            assert!(text.contains(i18n::tr!(ScanActionsTitle)), "{text}");
            for y in 0..height {
                for x in 0..width {
                    if matches!(
                        s.pointer.hit(Position::new(x, y)),
                        Some(Hit::Button(PointerAction::Management(_)))
                    ) {
                        assert!(input.contains(Position::new(x, y)), "{text}");
                    }
                }
            }
            let selection = session(&mut s).selection();
            key(&mut s, KeyCode::Tab);
            assert_eq!(session(&mut s).selection(), selection);
            key(&mut s, KeyCode::Esc);
            assert_eq!(s.bar.editor().text, "keep");
            assert_eq!(session(&mut s).state.selected(), selected);
        }
        s.paste("//");
        key(&mut s, KeyCode::Char('r'));
        let draft = s.bar.text();
        key(&mut s, KeyCode::BackTab);
        render(&mut s, 120, 32);
        assert!(s.bar.command_input().is_none());
        key(&mut s, KeyCode::BackTab);
        assert_eq!(s.bar.text(), draft);
        assert!(s.bar.command_input().is_some());
        key(&mut s, KeyCode::Esc);
        key(&mut s, KeyCode::BackTab);
        s.paste("-me");
        assert_eq!(s.bar.editor().text, "keep-me");
        assert_eq!(session(&mut s).selected().unwrap().path, path);
        key(&mut s, KeyCode::BackTab);
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert_eq!(s.config.repositories.len(), 2);
        s.shutdown();
    }
}
