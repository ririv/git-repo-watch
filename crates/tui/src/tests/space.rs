use super::*;

#[test]
fn empty_search_ignores_spaces_but_text_and_command_prefixes_keep_them() {
    let (_home, mut s) = fixture(vec!["/example/a".into()]);
    let selected = s.selected_path();
    for _ in 0..3 {
        key(&mut s, KeyCode::Char(' '));
    }
    assert!(s.bar.editor().text.is_empty());
    assert_eq!(s.selected_path(), selected);
    key(&mut s, KeyCode::Char('a'));
    key(&mut s, KeyCode::Char(' '));
    assert_eq!(s.bar.text(), "a ");
    key(&mut s, KeyCode::Esc);
    s.open_command_bar();
    let before = s.bar.text();
    key(&mut s, KeyCode::Char(' '));
    assert_eq!(s.bar.text(), format!("{before} "));
    s.shutdown();
}

#[test]
fn content_checks_receive_only_empty_search_spaces_and_commands_keep_their_own_spaces() {
    let f = support::Fixture::create();
    support::write(f.repo.join("draft.txt"), "draft\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    open_files(&mut s);
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    key(&mut s, KeyCode::Char(' '));
    assert!(!s.mode.file_session().unwrap().selected.is_empty());
    assert!(s.bar.editor().text.is_empty());
    key(&mut s, KeyCode::Char(' '));
    assert!(s.mode.file_session().unwrap().selected.is_empty());
    s.paste("draft");
    key(&mut s, KeyCode::Char(' '));
    assert_eq!(s.bar.text(), "draft ");
    assert!(s.mode.file_session().unwrap().selected.is_empty());
    key(&mut s, KeyCode::Esc);
    s.open_command_bar();
    let before = s.bar.text();
    key(&mut s, KeyCode::Char(' '));
    assert_eq!(s.bar.text(), format!("{before} "));
    assert!(s.mode.file_session().unwrap().selected.is_empty());
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    s.shutdown();
}

#[test]
fn conflict_and_recovery_action_pages_ignore_space_without_writing_or_switching_pages() {
    let (f, _home, mut s) = super::conflict::automatic_setup();
    let path = f.repo.join("src/中文 空格.ts");
    let contents = std::fs::read(&path).unwrap();
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    for page in [
        crate::input_pages::Page::Conflict,
        crate::input_pages::Page::Replay,
    ] {
        s.show_input_page(page);
        let before = s.footer_page_number();
        let conflict = s.focused_conflict_control();
        let replay = s.focused_replay_control();
        key(&mut s, KeyCode::Char(' '));
        wait(&mut s);
        assert_eq!(s.footer_page_number(), before);
        assert_eq!(s.focused_conflict_control(), conflict);
        assert_eq!(s.focused_replay_control(), replay);
        assert_eq!(std::fs::read(&path).unwrap(), contents);
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    }
    s.shutdown();
}

#[test]
fn scan_actions_ignore_space_and_the_input_page_can_toggle_repositories() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![]);
    command(&mut s, &format!("repos scan \"{}\"", f.repo.display()));
    wait(&mut s);
    let config = std::fs::read(s.service.store.config_path()).unwrap();
    assert_eq!(s.footer_page_number(), Some((2, 2)));
    key(&mut s, KeyCode::Char(' '));
    assert_eq!(s.footer_page_number(), Some((2, 2)));
    assert_eq!(
        std::fs::read(s.service.store.config_path()).unwrap(),
        config
    );
    key(&mut s, KeyCode::BackTab);
    key(&mut s, KeyCode::Char(' '));
    let Mode::Management(session) = &s.mode else {
        panic!("scan results")
    };
    assert!(session.selection().is_empty());
    assert!(s.bar.editor().text.is_empty());
    key(&mut s, KeyCode::Char(' '));
    let Mode::Management(session) = &s.mode else {
        panic!("scan results")
    };
    assert_eq!(session.selection().len(), 1);
    s.shutdown();
}

#[test]
fn commit_and_stash_action_pages_ignore_space_while_text_pages_preserve_leading_spaces() {
    for name in ["repo commit", "repo stash save tracked"] {
        let f = support::Fixture::create();
        f.commit("file.txt", "base\n");
        support::write(f.repo.join("file.txt"), "staged\n");
        support::git(&f.repo, &["add", "file.txt"]);
        let head = support::git(&f.repo, &["rev-parse", "HEAD"]);
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        command(&mut s, name);
        wait(&mut s);
        assert_eq!(s.footer_page_number(), Some((2, 2)));
        key(&mut s, KeyCode::Char(' '));
        assert_eq!(s.footer_page_number(), Some((2, 2)));
        key(&mut s, KeyCode::BackTab);
        key(&mut s, KeyCode::Char(' '));
        match &s.mode {
            Mode::CommitMessage { editor, .. } => assert_eq!(editor.text(), " "),
            Mode::StashSave(session) => assert_eq!(session.message.text, " "),
            _ => panic!("text form"),
        }
        assert_eq!(s.footer_page_number(), Some((1, 2)));
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), head);
        s.shutdown();
    }
}

#[test]
fn workflow_space_toggles_the_step_but_cannot_execute_the_selected_footer_action() {
    let (_home, mut s) = fixture(vec![]);
    command(&mut s, "schedule actions");
    let before = std::fs::read(s.service.store.config_path()).unwrap();
    let Mode::Workflow(session) = &s.mode else {
        panic!("workflow editor")
    };
    let steps = session.steps();
    key(&mut s, KeyCode::Char(' '));
    let Mode::Workflow(session) = &s.mode else {
        panic!("workflow editor")
    };
    assert_ne!(session.steps(), steps);
    key(&mut s, KeyCode::BackTab);
    key(&mut s, KeyCode::Char(' '));
    key(&mut s, KeyCode::Right);
    key(&mut s, KeyCode::Char(' '));
    assert!(matches!(s.mode, Mode::Workflow(_)));
    assert_eq!(s.footer_page_number(), Some((2, 2)));
    assert_eq!(
        std::fs::read(s.service.store.config_path()).unwrap(),
        before
    );
    s.shutdown();
}
