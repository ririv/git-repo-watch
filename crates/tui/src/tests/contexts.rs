use super::*;
use crate::command::{Context, Input, NamespaceEvent, context::DiffContext};
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
    assert_eq!(input.text(), "/repo diff layout 中文");
    input
        .editor_mut()
        .key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
    input
        .editor_mut()
        .key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
    assert_eq!(
        input.namespace_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)),
        NamespaceEvent::Handled
    );
    assert!(input.prefix_selected());
    assert_eq!(
        input.namespace_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        NamespaceEvent::Handled
    );
    assert!(!input.prefix_selected());
    assert_eq!(
        input.namespace_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)),
        NamespaceEvent::Handled
    );
    assert_eq!(
        input.namespace_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)),
        NamespaceEvent::Handled
    );
    assert_eq!(input.text(), "/repo ");
    assert_eq!(
        input.namespace_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)),
        NamespaceEvent::Handled
    );
    assert_eq!(
        input.namespace_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)),
        NamespaceEvent::Close
    );
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
        command(&mut s, "repo diff");
        wait(&mut s);
        s.paste("source");
        key(&mut s, KeyCode::Home);
        key(&mut s, KeyCode::Char('/'));
        assert_eq!(context(&s), Some(Context::Diff(DiffContext::Worktree)));
        assert!(
            s.search_commands()
                .unwrap()
                .iter()
                .all(|c| c.name.starts_with("repo diff "))
        );
        for (w, h) in [(60, 18), (100, 28), (180, 36)] {
            let text = render(&mut s, w, h);
            assert!(text.contains("/repo diff"), "{text}");
            assert!(!text.contains("/repo diff stage"), "{text}");
            assert!(
                text.contains(i18n::tr!(ContextCommandPlaceholder)),
                "{text}"
            );
        }
        s.paste("repo terminal");
        key(&mut s, KeyCode::Enter);
        assert!(s.search_commands().unwrap().is_empty());
        assert!(s.repository_tasks.is_empty());
        control(&mut s, 'u');
        s.paste("布局");
        assert!(
            s.search_commands()
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
        key(&mut s, KeyCode::Backspace);
        key(&mut s, KeyCode::Left);
        key(&mut s, KeyCode::Backspace);
        assert_eq!(context(&s), None);
        assert!(matches!(s.mode, Mode::Changes(_)));
        assert!(matches!(s.bar, bar::InputBar::Search(_)));
        assert_eq!(s.bar.text(), "source");
        s.paste("//settings");
        assert_eq!(s.search_commands().unwrap()[0].action, Action::Settings);
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
    command(&mut s, "repo diff");
    wait(&mut s);
    key(&mut s, KeyCode::Char('/'));
    s.paste("sta");
    key(&mut s, KeyCode::Tab);
    assert_eq!(s.bar.editor().text, "stage");
    assert_eq!(s.bar.text(), "/repo diff stage");
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
    command(&mut s, "repo log");
    wait(&mut s);
    key(&mut s, KeyCode::Char('/'));
    assert_eq!(context(&s), Some(Context::History));
    assert!(
        s.search_commands()
            .unwrap()
            .iter()
            .all(|c| c.name.starts_with("repo log "))
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
    assert!(s.search_commands().unwrap().iter().all(|c| matches!(
        c.action,
        Action::DiffLayoutMenu
            | Action::DiffHeader
            | Action::DiffLayout(_)
            | Action::Copy(crate::clipboard::Scope::Diff)
    )));
    s.paste("stage");
    key(&mut s, KeyCode::Enter);
    assert!(s.search_commands().unwrap().is_empty());
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    key(&mut s, KeyCode::Esc);
    command(&mut s, "repo branch");
    wait(&mut s);
    key(&mut s, KeyCode::Char('/'));
    assert_eq!(context(&s), Some(Context::Branches));
    assert_eq!(s.search_commands().unwrap()[0].name, "repo branch refresh");
}

#[test]
fn configuration_scope_preserves_tilde_completion_unicode_and_confirmation_cursor() {
    let (_home, mut s) = fixture(vec![]);
    std::fs::create_dir(s.service.scheduler.user_home().join("中文 目录")).unwrap();
    let config = std::fs::read(s.service.store.config_path()).unwrap();
    command(&mut s, "repos manage");
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
    command(&mut s, "repo diff");
    wait(&mut s);
    key(&mut s, KeyCode::Char('/'));
    s.focus(Some(other.repo.clone()));
    assert_eq!(context(&s), Some(Context::Diff(DiffContext::Worktree)));
    key(&mut s, KeyCode::Char('/'));
    s.paste("repos remove");
    key(&mut s, KeyCode::Enter);
    assert!(
        matches!(&s.mode,Mode::Confirm { operation: Confirmation::Remove { path, .. },.. } if path==&f.repo)
    );
    key(&mut s, KeyCode::Esc);
}

#[test]
fn repository_home_scopes_list_and_management_commands_and_reopens_after_global_escape() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec!["/example/alpha".into()]);
        let mut config = s.service.store.config().unwrap();
        key(&mut s, KeyCode::Char('/'));
        assert_eq!(context(&s), None);
        assert_eq!(s.bar.text(), "/");
        let commands = s.search_commands().unwrap();
        assert!(commands.iter().all(|c| !c.name.contains(' ')));
        assert!(commands.iter().any(|c| c.action == Action::OpenRepository));
        assert!(commands.iter().any(|c| c.action == Action::Repositories));
        for (w, h) in [(60, 18), (100, 28), (180, 36)] {
            let text = render(&mut s, w, h);
            assert!(text.contains("/repos"), "{text}");
        }
        s.paste("repos sort name");
        key(&mut s, KeyCode::Enter);
        assert_eq!(s.order, Order::Name);
        command(&mut s, "repos manage");
        assert_eq!(s.page, Page::Configuration(Configuration::Repositories));
        key(&mut s, KeyCode::Char('/'));
        assert_eq!(context(&s), Some(Context::Repositories));
        assert!(s.search_commands().unwrap().iter().all(|c| !matches!(
            c.action,
            Action::FilterMenu
                | Action::Filter(_)
                | Action::SortMenu
                | Action::Sort(_)
                | Action::RepositorySettings
                | Action::RepositoryRefresh
        )));
        key(&mut s, KeyCode::Esc);
        command(&mut s, "repos");
        assert_eq!(s.page, Page::Repositories);
        assert!(s.history.is_empty());
        assert_eq!(s.order, Order::Name);
        wait(&mut s);
        config.view.repository_order = engine::state::view::RepositoryOrder::Name;
        assert_eq!(s.service.store.config().unwrap(), config);
        assert!(s.repository_tasks.is_empty() && s.check_task.is_none());
    }
}

#[test]
fn repository_command_returns_from_a_repository_to_the_same_filtered_searched_list() {
    let f = support::Fixture::create();
    let other = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone(), other.repo.clone()]);
    s.order = Order::Path;
    s.query = f.repo.to_string_lossy().to_string();
    s.sync_bar();
    s.focus(Some(f.repo.clone()));
    key(&mut s, KeyCode::Enter);
    wait_workspace(&mut s);
    assert_eq!(s.page, Page::Detail);
    command(&mut s, "repos");
    assert_eq!(s.page, Page::Repositories);
    assert!(matches!(s.mode, Mode::Browsing));
    assert_eq!(s.selected_path(), Some(f.repo));
    assert_eq!(s.order, Order::Path);
    assert_eq!(s.bar.editor().text, s.query);
    assert!(s.history.is_empty());
    s.shutdown();
}

#[test]
fn repository_refresh_only_reloads_cached_reports_and_keeps_other_views_explicit() {
    let f = support::Fixture::create();
    support::write(f.repo.join("untracked"), "content\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    command(&mut s, "repos refresh");
    assert!(matches!(s.mode, Mode::Changes(_)));
    assert_eq!(s.message, i18n::tr!(RepositoryListOnly));
    assert!(s.repository_tasks.is_empty());
    command(&mut s, "repos");
    command(&mut s, "repos refresh");
    assert_eq!(s.page, Page::Repositories);
    assert!(matches!(s.mode, Mode::Browsing));
    assert!(s.repository_tasks.is_empty() && s.check_task.is_none());
    assert!(s.service.store.report().unwrap().is_none());
    assert!(s.service.store.repository_reports().unwrap().is_empty());
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert_eq!(
        std::fs::read_to_string(f.repo.join("untracked")).unwrap(),
        "content\n"
    );
    s.shutdown();
}

#[test]
fn repository_entry_freezes_its_target_and_nested_views_keep_full_context_paths() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let first = support::Fixture::create();
        let second = support::Fixture::create();
        support::write(first.repo.join("中文 file.ts"), "export const value = 1;\n");
        let (_home, mut s) = fixture(vec![first.repo.clone(), second.repo.clone()]);
        let mut config = s.service.store.config().unwrap();
        let index = std::fs::read(first.repo.join(".git/index")).unwrap();
        s.focus(Some(first.repo.clone()));
        key(&mut s, KeyCode::Char('/'));
        assert_eq!(s.bar.text(), "/");
        s.paste("repo");
        s.focus(Some(second.repo.clone()));
        key(&mut s, KeyCode::Enter);
        wait_workspace(&mut s);
        assert_eq!(s.page, Page::Detail);
        assert_eq!(s.workspace.path(), Some(first.repo.as_path()));
        key(&mut s, KeyCode::Char('/'));
        assert_eq!(s.bar.text(), "/repo ");
        assert_eq!(context(&s), Some(Context::Repository));
        assert!(
            s.search_commands()
                .unwrap()
                .iter()
                .all(|c| c.name.starts_with("repo "))
        );
        s.paste("diff");
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        key(&mut s, KeyCode::Char('/'));
        assert_eq!(s.bar.text(), "/repo diff ");
        s.paste("layout split");
        key(&mut s, KeyCode::Enter);
        assert_eq!(s.diff_preference, crate::diff::Preference::Split);
        command(&mut s, "repo");
        wait_workspace(&mut s);
        assert_eq!(s.command_context(), Some(Context::Repository));
        command(&mut s, "repos");
        assert_eq!(s.page, Page::Repositories);
        assert_eq!(s.command_context(), None);
        assert_eq!(s.selected_path(), Some(first.repo.clone()));
        wait(&mut s);
        config.view.diff_layout = engine::state::view::DiffLayout::Split;
        assert_eq!(s.service.store.config().unwrap(), config);
        assert_eq!(std::fs::read(first.repo.join(".git/index")).unwrap(), index);
        s.shutdown();
    }
}

#[test]
fn repository_entry_requires_a_selection() {
    let (_home, mut s) = fixture(vec![]);
    command(&mut s, "repo");
    assert_eq!(s.page, Page::Repositories);
    assert_eq!(s.message, i18n::tr!(SelectARepositoryFirst));
    assert!(s.workspace.session().is_none());
}
