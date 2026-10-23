use super::*;
use crate::command::{Context, Input, context::DiffContext};
fn control(s: &mut Screen, c: char) {
    s.key(
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL),
        &Cancel::default(),
    )
    .unwrap();
}
fn context(s: &Screen) -> Option<Context> {
    s.bar.command_input().and_then(Input::context)
}

#[test]
fn locked_prefix_selects_before_removal_and_never_executes_on_focus_return() {
    let mut input = Input::new(Some(Context::Diff(DiffContext::Worktree)));
    input.paste("layout 中文");
    input
        .editor_mut()
        .key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
    input
        .editor_mut()
        .key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
    assert_eq!(input.text(), "/diff layout 中文");
    input
        .editor_mut()
        .key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
    input
        .editor_mut()
        .key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
    assert!(input.namespace_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)));
    assert!(input.prefix_selected());
    assert!(input.namespace_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)));
    assert!(!input.prefix_selected());
    for _ in 0..2 {
        input.namespace_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
    }
    assert_eq!(input.context(), None);
    assert_eq!(input.text(), "/");
    for text in ["/settings", "//settings"] {
        let mut input = Input::new(Some(Context::Diff(DiffContext::Worktree)));
        input.paste(text);
        assert_eq!(input.context(), None);
        assert_eq!(input.text(), "/settings");
    }
}

#[test]
fn diff_scope_filters_bilingual_candidates_preserves_search_and_reopens_layout_drafts() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        support::write(f.repo.join("source.ts"), "export const value = 1;\n");
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        command(&mut s, "diff");
        wait(&mut s);
        s.paste("source");
        key(&mut s, KeyCode::Home);
        key(&mut s, KeyCode::Char('/'));
        assert_eq!(context(&s), Some(Context::Diff(DiffContext::Worktree)));
        assert!(
            s.bar
                .commands()
                .unwrap()
                .iter()
                .all(|c| c.name.starts_with("diff "))
        );
        for (w, h) in [(60, 18), (100, 28), (180, 36)] {
            let text = render(&mut s, w, h);
            assert!(text.contains("/diff"), "{text}");
            assert!(!text.contains("/diff stage"), "{text}");
            assert!(
                text.contains(i18n::tr!(ContextCommandPlaceholder)),
                "{text}"
            );
        }
        s.paste("terminal");
        key(&mut s, KeyCode::Enter);
        assert!(s.bar.commands().unwrap().is_empty());
        assert!(s.repository_tasks.is_empty());
        control(&mut s, 'u');
        s.paste("布局");
        assert!(
            s.bar
                .commands()
                .unwrap()
                .iter()
                .all(|c| matches!(c.action, Action::DiffLayoutMenu | Action::DiffLayout(_)))
        );
        control(&mut s, 'u');
        s.paste("layout");
        let cursor = s.bar.editor().cursor;
        key(&mut s, KeyCode::Enter);
        assert!(matches!(s.bar, bar::InputBar::Layout { .. }));
        key(&mut s, KeyCode::Right);
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.bar.editor().text, "layout");
        assert_eq!(s.bar.editor().cursor, cursor);
        assert_eq!(context(&s), Some(Context::Diff(DiffContext::Worktree)));
        control(&mut s, 'u');
        for _ in 0..2 {
            key(&mut s, KeyCode::Backspace);
        }
        assert_eq!(context(&s), None);
        assert!(matches!(s.mode, Mode::Changes(_)));
        s.paste("settings");
        assert_eq!(s.bar.commands().unwrap()[0].action, Action::Settings);
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.bar.editor().text, "source");
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    }
}

#[test]
fn stage_uses_local_suffix_and_preserves_the_existing_git_workflow() {
    let f = support::Fixture::create();
    support::write(f.repo.join("new-source"), "content\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "diff");
    wait(&mut s);
    key(&mut s, KeyCode::Char('/'));
    s.paste("sta");
    key(&mut s, KeyCode::Tab);
    assert_eq!(s.bar.editor().text, "stage");
    assert_eq!(s.bar.text(), "/diff stage");
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(
        support::git(&f.repo, &["diff", "--cached", "--name-only"]),
        "new-source"
    );
}

#[test]
fn history_branch_and_historical_diff_have_explicit_groups_without_worktree_writes() {
    let f = support::Fixture::create();
    support::write(f.repo.join("source.rs"), "fn before() {}\n");
    support::git(&f.repo, &["add", "source.rs"]);
    support::git(&f.repo, &["commit", "-m", "before"]);
    support::write(f.repo.join("source.rs"), "fn after() {}\n");
    support::git(&f.repo, &["commit", "-am", "after"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "log");
    wait(&mut s);
    key(&mut s, KeyCode::Char('/'));
    assert_eq!(context(&s), Some(Context::History));
    assert!(
        s.bar
            .commands()
            .unwrap()
            .iter()
            .all(|c| c.name.starts_with("log "))
    );
    key(&mut s, KeyCode::Char('/'));
    assert_eq!(context(&s), None);
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(
        matches!(&s.mode,Mode::History(view) if matches!(view.as_ref(),crate::history::View::Diff { .. }))
    );
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    key(&mut s, KeyCode::Char('/'));
    assert_eq!(context(&s), Some(Context::Diff(DiffContext::History)));
    assert!(s.bar.commands().unwrap().iter().all(|c| matches!(
        c.action,
        Action::DiffLayoutMenu
            | Action::DiffLayout(_)
            | Action::Copy(crate::clipboard::Scope::Diff)
    )));
    s.paste("stage");
    key(&mut s, KeyCode::Enter);
    assert!(s.bar.commands().unwrap().is_empty());
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    key(&mut s, KeyCode::Esc);
    command(&mut s, "branch");
    wait(&mut s);
    key(&mut s, KeyCode::Char('/'));
    assert_eq!(context(&s), Some(Context::Branches));
    assert_eq!(s.bar.commands().unwrap()[0].name, "branch refresh");
}

#[test]
fn configuration_scope_preserves_tilde_completion_unicode_and_confirmation_cursor() {
    let (_home, mut s) = fixture(vec![]);
    std::fs::create_dir(s.service.scheduler.user_home().join("中文 目录")).unwrap();
    let config = std::fs::read(s.service.store.config_path()).unwrap();
    command(&mut s, "repo");
    key(&mut s, KeyCode::Char('/'));
    assert_eq!(context(&s), Some(Context::Repositories));
    s.paste("add \"~/中文\"");
    key(&mut s, KeyCode::Left);
    key(&mut s, KeyCode::Tab);
    assert_eq!(s.bar.editor().text, "add \"~/中文 目录/\"");
    let cursor = s.bar.editor().cursor;
    key(&mut s, KeyCode::Enter);
    assert!(matches!(s.mode, Mode::Confirm { .. }));
    key(&mut s, KeyCode::Esc);
    assert_eq!(context(&s), Some(Context::Repositories));
    assert_eq!(s.bar.editor().cursor, cursor);
    assert_eq!(s.bar.editor().text, "add \"~/中文 目录/\"");
    assert_eq!(
        std::fs::read(s.service.store.config_path()).unwrap(),
        config
    );
    key(&mut s, KeyCode::Esc);
    command(&mut s, "settings");
    key(&mut s, KeyCode::Char('/'));
    assert_eq!(context(&s), Some(Context::Settings));
    key(&mut s, KeyCode::Esc);
    command(&mut s, "schedule");
    key(&mut s, KeyCode::Char('/'));
    assert_eq!(context(&s), Some(Context::Schedule));
}

#[test]
fn changing_command_namespace_keeps_the_original_repository_scope() {
    let f = support::Fixture::create();
    let other = support::Fixture::create();
    support::write(f.repo.join("draft"), "content");
    let (_home, mut s) = fixture(vec![f.repo.clone(), other.repo.clone()]);
    s.focus(Some(f.repo.clone()));
    command(&mut s, "diff");
    wait(&mut s);
    key(&mut s, KeyCode::Char('/'));
    s.focus(Some(other.repo.clone()));
    assert_eq!(context(&s), Some(Context::Diff(DiffContext::Worktree)));
    key(&mut s, KeyCode::Char('/'));
    s.paste("repo remove");
    key(&mut s, KeyCode::Enter);
    assert!(
        matches!(&s.mode,Mode::Confirm { operation: Confirmation::Remove(path),.. } if path==&f.repo)
    );
    key(&mut s, KeyCode::Esc);
}
