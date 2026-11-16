use super::*;
use crate::input_pages::Page;

#[derive(Clone, Copy)]
enum Clear {
    Backspace,
    Delete,
    Interrupt,
    Escape,
}
fn clear_search(s: &mut Screen, clear: Clear) {
    match clear {
        Clear::Backspace => {
            key(s, KeyCode::Backspace);
        }
        Clear::Delete => {
            key(s, KeyCode::Home);
            key(s, KeyCode::Delete);
        }
        Clear::Interrupt => {
            s.key(
                KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
                &Cancel::default(),
            )
            .unwrap();
        }
        Clear::Escape => {
            key(s, KeyCode::Esc);
        }
    }
}
fn scan_focus(s: &Screen) -> &crate::management::Focus {
    let Mode::Management(session) = &s.mode else {
        panic!("scan review")
    };
    &session.focus
}
#[test]
fn clearing_temporary_scan_text_restores_the_exact_action_but_manual_input_stays_open() {
    for clear in [
        Clear::Backspace,
        Clear::Delete,
        Clear::Interrupt,
        Clear::Escape,
    ] {
        let f = support::Fixture::create();
        let (_home, mut s) = fixture(vec![]);
        command(&mut s, &format!("repos scan \"{}\"", f.repo.display()));
        wait(&mut s);
        let before = std::fs::read(s.service.store.config_path()).unwrap();
        key(&mut s, KeyCode::Right);
        key(&mut s, KeyCode::Char('文'));
        assert!(scan_focus(&s).is_input());
        render(&mut s, 100, 28);
        let geometry = (s.body_area, s.input_area);
        clear_search(&mut s, clear);
        let text = render(&mut s, 100, 28);
        assert_eq!(s.footer_page_number(), Some((2, 2)), "{text}");
        assert!(matches!(
            scan_focus(&s),
            crate::input_pages::Selection::Actions(crate::management::Controls {
                selected: crate::management::Action::Back,
                ..
            })
        ));
        assert_eq!((s.body_area, s.input_area), geometry);
        key(&mut s, KeyCode::BackTab);
        key(&mut s, KeyCode::Char('文'));
        key(&mut s, KeyCode::Backspace);
        render(&mut s, 100, 28);
        assert_eq!(s.footer_page_number(), Some((1, 2)));
        let registered = s.config.repositories.clone();
        if let Mode::Management(session) = &mut s.mode {
            session.refresh(&registered);
        }
        render(&mut s, 100, 28);
        assert_eq!(s.footer_page_number(), Some((1, 2)));
        assert_eq!(
            std::fs::read(s.service.store.config_path()).unwrap(),
            before
        );
        s.shutdown();
    }
}

#[test]
fn commit_and_stash_share_temporary_typing_and_restore_the_selected_action_on_empty_text() {
    for name in ["repo commit", "repo stash save tracked"] {
        let f = support::Fixture::create();
        f.commit("file.txt", "base\n");
        support::write(f.repo.join("file.txt"), "staged\n");
        support::git(&f.repo, &["add", "file.txt"]);
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        let head = support::git(&f.repo, &["rev-parse", "HEAD"]);
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        command(&mut s, name);
        wait(&mut s);
        key(&mut s, KeyCode::Right);
        s.paste("字");
        assert_eq!(s.footer_page_number(), Some((1, 2)));
        key(&mut s, KeyCode::Backspace);
        assert_eq!(s.footer_page_number(), Some((2, 2)));
        match &s.mode {
            Mode::CommitMessage { editor, .. } => assert_eq!(
                editor.selected(),
                Some(crate::commit_form::Action::Edit(
                    crate::commit_form::EditAction::Return
                ))
            ),
            Mode::StashSave(session) => assert_eq!(
                session.focus,
                crate::stash_save::Focus::Actions(confirmation::Choice::Return)
            ),
            _ => panic!("review remained open"),
        }
        key(&mut s, KeyCode::BackTab);
        s.paste("字");
        key(&mut s, KeyCode::Backspace);
        render(&mut s, 100, 28);
        assert_eq!(s.footer_page_number(), Some((1, 2)));
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), head);
        s.shutdown();
    }
}

#[test]
fn empty_conflict_search_returns_to_the_same_file_and_action_without_closing_an_empty_command_menu()
{
    let (f, _home, mut s) = super::conflict::automatic_setup();
    let original = std::fs::read(f.repo.join("src/中文 空格.ts")).unwrap();
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    let right = crate::conflict::controls::Action::Accept(
        engine::local::files::conflict::resolve::Choice::Right,
    );
    key(&mut s, KeyCode::Right);
    assert_eq!(s.focused_conflict_control(), Some(right));
    key(&mut s, KeyCode::Char('文'));
    assert_eq!(s.input_page(), Page::Input);
    key(&mut s, KeyCode::Backspace);
    render(&mut s, 140, 38);
    assert_eq!(s.input_page(), Page::Conflict);
    assert_eq!(s.focused_conflict_control(), Some(right));
    key(&mut s, KeyCode::Char('/'));
    render(&mut s, 140, 38);
    assert!(s.bar.command_input().is_some());
    assert_eq!(s.input_page(), Page::Input);
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.input_page(), Page::Conflict);
    assert_eq!(s.focused_conflict_control(), Some(right));
    while s.input_page() != Page::Input {
        key(&mut s, KeyCode::BackTab);
    }
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Char('文'));
    key(&mut s, KeyCode::Backspace);
    s.normalize_input_page();
    assert_eq!(s.input_page(), Page::Input);
    assert_eq!(
        std::fs::read(f.repo.join("src/中文 空格.ts")).unwrap(),
        original
    );
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    s.shutdown();
}
