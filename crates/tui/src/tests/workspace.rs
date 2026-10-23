use super::*;

fn focus(s: &mut Screen, section: crate::workspace::Section) {
    while matches!(s.mode, Mode::Changes(_)) {
        key(s, KeyCode::Esc);
    }
    for _ in 0..crate::workspace::Section::ALL.len() {
        if crate::workspace::Section::focused(&s.mode) == Some(section) {
            return;
        }
        s.key(
            KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
            &Cancel::default(),
        )
        .unwrap();
        wait(s);
    }
    assert_eq!(crate::workspace::Section::of(&s.mode), section);
}
fn zoom(s: &mut Screen) {
    s.key(
        KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
        &Cancel::default(),
    )
    .unwrap();
}

#[test]
fn maximized_panes_use_the_entire_body_and_switch_without_losing_their_sessions() {
    use crate::workspace::{Arrangement, Section};
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        f.commit("tracked", "tracked\n");
        support::write(f.repo.join("pending"), "pending\n");
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        key(&mut s, KeyCode::Enter);
        wait_workspace(&mut s);
        super::overview::wait_overview(&mut s);
        focus(&mut s, crate::workspace::Section::Worktree);
        key(&mut s, KeyCode::Char(' '));
        focus(&mut s, crate::workspace::Section::History);
        key(&mut s, KeyCode::Down);
        let (source, selection) = match &s.mode {
            Mode::History(view) => (view.session().source, view.session().state.selected()),
            _ => unreachable!(),
        };
        for (width, height) in [(60, 18), (140, 32)] {
            zoom(&mut s);
            for section in [
                Section::History,
                Section::Overview,
                Section::Branches,
                Section::Worktree,
            ] {
                let text = render(&mut s, width, height);
                assert_eq!(Section::of(&s.mode), section);
                assert_eq!(
                    s.workspace.session().unwrap().arrangement,
                    Arrangement::Maximized
                );
                assert_eq!(s.workspace_areas, vec![(s.body_area.unwrap(), section)]);
                if width >= 140 {
                    assert!(text.contains(i18n::tr!(WorkspaceRestoreHint)), "{text}");
                }
                s.key(
                    KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
                    &Cancel::default(),
                )
                .unwrap();
                wait(&mut s);
            }
            zoom(&mut s);
            render(&mut s, width, height);
            assert_eq!(s.workspace_areas.len(), 4);
            assert_eq!(
                s.workspace.session().unwrap().arrangement,
                Arrangement::Tiled
            );
        }
        assert!(
            matches!(&s.mode, Mode::History(view) if view.session().source==source && view.session().state.selected()==selection)
        );
        focus(&mut s, crate::workspace::Section::Worktree);
        assert!(matches!(&s.mode, Mode::Files(files) if files.selected.len()==1));
    }
}

#[test]
fn maximized_files_keep_diff_back_navigation_and_escape_clears_input_before_restoring() {
    use crate::workspace::Arrangement;
    let f = support::Fixture::create();
    support::write(f.repo.join("pending.ts"), "export const pending = true;\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    key(&mut s, KeyCode::Enter);
    focus(&mut s, crate::workspace::Section::Worktree);
    zoom(&mut s);
    s.paste("pending");
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(matches!(s.mode, Mode::Changes(_)));
    assert!(render(&mut s, 100, 28).contains("export const pending"));
    key(&mut s, KeyCode::Esc);
    assert!(matches!(&s.mode, Mode::Changes(session) if session.files.query.is_empty()));
    key(&mut s, KeyCode::Esc);
    assert!(matches!(s.mode, Mode::Files(_)));
    assert_eq!(
        s.workspace.session().unwrap().arrangement,
        Arrangement::Maximized
    );
    s.paste("pending");
    key(&mut s, KeyCode::Esc);
    assert!(s.bar.editor().text.is_empty());
    assert_eq!(
        s.workspace.session().unwrap().arrangement,
        Arrangement::Maximized
    );
    s.paste("//comm");
    zoom(&mut s);
    assert_eq!(s.bar.text(), "/comm");
    assert_eq!(
        s.workspace.session().unwrap().arrangement,
        Arrangement::Maximized
    );
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Esc);
    assert!(matches!(s.mode, Mode::Browsing));
    assert_eq!(s.page, Page::Detail);
    assert_eq!(
        s.workspace.session().unwrap().arrangement,
        Arrangement::Tiled
    );
    render(&mut s, 100, 28);
    assert_eq!(s.workspace_areas.len(), 4);
    key(&mut s, KeyCode::Left);
    assert_eq!(s.page, Page::Repositories);
    key(&mut s, KeyCode::Enter);
    // 根层没有面板焦点，放大键不会隐式进入概览。
    zoom(&mut s);
    assert_eq!(s.breadcrumbs().len(), 2);
    assert_eq!(
        s.workspace.session().unwrap().arrangement,
        Arrangement::Tiled
    );
    key(&mut s, KeyCode::Right);
    assert!(matches!(
        s.mode,
        Mode::Panel(crate::workspace::Section::Overview)
    ));
    zoom(&mut s);
    assert_eq!(s.breadcrumbs().len(), 3);
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.page, Page::Detail);
    assert!(matches!(s.mode, Mode::Browsing));
    assert_eq!(
        s.workspace.session().unwrap().arrangement,
        Arrangement::Tiled
    );
}

#[test]
fn overview_files_and_history_are_visible_together_at_every_focus_and_supported_size() {
    use crate::workspace::Section;
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        support::write(f.repo.join("pending.ts"), "export const pending = true;\n");
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        key(&mut s, KeyCode::Enter);
        wait_workspace(&mut s);
        super::overview::wait_overview(&mut s);
        for (width, height) in [(60, 18), (100, 28), (140, 32), (200, 50)] {
            for section in Section::ALL {
                s.switch_repository_section(section);
                let text = render(&mut s, width, height);
                for label in [
                    Section::Overview.title(),
                    Section::Worktree.title(),
                    Section::History.title(),
                    "pending.ts",
                ] {
                    assert!(text.contains(label), "{label}: {text}");
                }
                assert_eq!(s.workspace_areas.len(), 4);
                let pane = |section| {
                    s.workspace_areas
                        .iter()
                        .find(|(_, value)| *value == section)
                        .unwrap()
                        .0
                };
                let (overview, branches, files, history) = (
                    pane(Section::Overview),
                    pane(Section::Branches),
                    pane(Section::Worktree),
                    pane(Section::History),
                );
                assert_eq!(overview.y, branches.y);
                assert_eq!(files.y, history.y);
                assert_eq!(overview.right(), files.right());
                assert_eq!(branches.x, history.x);
                assert!(files.height >= overview.height);
                for (index, (area, _)) in s.workspace_areas.iter().enumerate() {
                    assert!(area.width > 0 && area.height > 0);
                    for (other, _) in &s.workspace_areas[index + 1..] {
                        assert!(area.intersection(*other).is_empty());
                    }
                }
                assert!(
                    matches!(&s.workspace.session().unwrap().history, crate::workspace::Panel::Ready(graph) if graph.records.len()==1)
                );
            }
        }
    }
}

#[test]
fn initial_background_read_keeps_command_drafts_and_active_file_selections() {
    let f = support::Fixture::create();
    support::write(f.repo.join("pending"), "pending\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    key(&mut s, KeyCode::Enter);
    s.paste("//comm");
    wait_workspace(&mut s);
    assert_eq!(s.bar.text(), "/comm");
    key(&mut s, KeyCode::Esc);
    open_files(&mut s);
    wait(&mut s);
    key(&mut s, KeyCode::Char(' '));
    s.refresh_workspace();
    wait_workspace(&mut s);
    assert!(matches!(&s.mode, Mode::Files(files) if files.selected.len()==1));
    focus(&mut s, crate::workspace::Section::History);
    focus(&mut s, crate::workspace::Section::Worktree);
    assert!(matches!(&s.mode, Mode::Files(files) if files.selected.len()==1));
}

#[test]
fn repository_sections_reuse_file_diff_and_history_views_and_preserve_their_sessions() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        f.commit("tracked.ts", "export const tracked = true;\n");
        support::write(f.repo.join("pending.ts"), "export const pending = true;\n");
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        key(&mut s, KeyCode::Enter);
        focus(&mut s, crate::workspace::Section::Worktree);
        assert!(matches!(s.mode, Mode::Files(_)));
        key(&mut s, KeyCode::Char(' '));
        key(&mut s, KeyCode::Right);
        wait(&mut s);
        assert!(matches!(s.mode, Mode::Changes(_)));
        assert!(render(&mut s, 120, 30).contains("export const pending"));
        focus(&mut s, crate::workspace::Section::History);
        assert!(matches!(s.mode, Mode::History(_)));
        key(&mut s, KeyCode::Down);
        let (source, selected) = match &s.mode {
            Mode::History(view) => (view.session().source, view.session().state.selected()),
            _ => unreachable!(),
        };
        assert!(render(&mut s, 140, 32).contains(i18n::tr!(WorkspaceHistory)));
        focus(&mut s, crate::workspace::Section::Worktree);
        assert!(
            matches!(&s.mode, Mode::Files(files) if files.selected.len()==1 && files.snapshot.path==f.repo)
        );
        key(&mut s, KeyCode::Tab);
        assert!(
            matches!(&s.mode, Mode::Files(files) if files.rows().iter().all(|change| change.area != Area::Staged))
        );
        focus(&mut s, crate::workspace::Section::Overview);
        assert!(matches!(
            s.mode,
            Mode::Panel(crate::workspace::Section::Overview)
        ));
        assert_eq!(s.page, Page::Detail);
        focus(&mut s, crate::workspace::Section::History);
        match &s.mode {
            Mode::History(view) => assert_eq!(
                (view.session().source, view.session().state.selected()),
                (source, selected)
            ),
            other => panic!("{other:?}"),
        }
        assert!(s.service.store.report().unwrap().is_none());
        assert!(support::git(&f.repo, &["diff", "--cached"]).is_empty());
    }
}

#[test]
fn section_clicks_preserve_command_drafts_and_another_repository_gets_its_own_session() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let first = support::Fixture::create();
    let second = support::Fixture::create();
    support::write(first.repo.join("first-pending"), "first\n");
    let (_home, mut s) = fixture(vec![first.repo.clone(), second.repo.clone()]);
    s.focus(Some(first.repo.clone()));
    key(&mut s, KeyCode::Enter);
    render(&mut s, 100, 28);
    let area = s
        .workspace_areas
        .iter()
        .find(|(_, section)| *section == crate::workspace::Section::Worktree)
        .unwrap()
        .0;
    let click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: area.x + 1,
        row: area.y,
        modifiers: KeyModifiers::NONE,
    };
    assert!(s.mouse(click, &Cancel::default()).unwrap());
    wait(&mut s);
    assert!(matches!(&s.mode, Mode::Files(files) if files.snapshot.path==first.repo));
    s.paste("//comm");
    let draft = s.bar.editor().text.clone();
    render(&mut s, 100, 28);
    s.mouse(click, &Cancel::default()).unwrap();
    assert_eq!(s.bar.editor().text, draft);
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Left);
    key(&mut s, KeyCode::Left);
    s.focus(Some(second.repo.clone()));
    key(&mut s, KeyCode::Enter);
    focus(&mut s, crate::workspace::Section::Worktree);
    assert!(
        matches!(&s.mode, Mode::Files(files) if files.snapshot.path==second.repo && files.rows().is_empty())
    );
}

#[test]
fn successful_commit_refreshes_inactive_file_and_history_snapshots() {
    let f = support::Fixture::create();
    support::write(f.repo.join("to-commit"), "content\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    key(&mut s, KeyCode::Enter);
    focus(&mut s, crate::workspace::Section::Worktree);
    key(&mut s, KeyCode::Char(' '));
    command(&mut s, "repo diff stage");
    wait(&mut s);
    focus(&mut s, crate::workspace::Section::History);
    assert!(matches!(&s.mode, Mode::History(view) if view.session().records.len()==1));
    focus(&mut s, crate::workspace::Section::Overview);
    command(&mut s, "repo commit");
    wait(&mut s);
    assert!(matches!(s.mode, Mode::CommitMessage { .. }));
    s.paste("workspace commit");
    key(&mut s, KeyCode::Enter);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(
        support::git(&f.repo, &["log", "-1", "--format=%s"]),
        "workspace commit"
    );
    wait_workspace(&mut s);
    focus(&mut s, crate::workspace::Section::History);
    assert!(matches!(&s.mode, Mode::History(view) if view.session().records.len()==2));
    focus(&mut s, crate::workspace::Section::Worktree);
    assert!(matches!(&s.mode, Mode::Files(files) if files.snapshot.changes.is_empty()));
}

#[test]
fn explicit_refresh_updates_inactive_views_after_external_repository_changes() {
    let f = support::Fixture::create();
    support::write(f.repo.join("pending"), "pending\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    key(&mut s, KeyCode::Enter);
    focus(&mut s, crate::workspace::Section::Worktree);
    assert!(matches!(&s.mode, Mode::Files(files) if files.snapshot.changes.len()==1));
    focus(&mut s, crate::workspace::Section::History);
    assert!(matches!(&s.mode, Mode::History(view) if view.session().records.len()==1));
    focus(&mut s, crate::workspace::Section::Overview);
    f.commit("pending", "committed\n");
    s.key(
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
        &Cancel::default(),
    )
    .unwrap();
    wait_workspace(&mut s);
    focus(&mut s, crate::workspace::Section::History);
    assert!(matches!(&s.mode, Mode::History(view) if view.session().records.len()==2));
    focus(&mut s, crate::workspace::Section::Worktree);
    assert!(matches!(&s.mode, Mode::Files(files) if files.snapshot.changes.is_empty()));
}
