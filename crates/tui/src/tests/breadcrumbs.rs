use super::*;
use crate::{
    navigation::breadcrumbs::{Destination, Level},
    workspace::Section,
};

fn labels(s: &Screen) -> Vec<String> {
    s.breadcrumbs()
        .into_iter()
        .map(|crumb| crumb.label)
        .collect()
}
fn click_crumb(s: &mut Screen, destination: Destination) {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    render(s, 180, 36);
    let area = s
        .breadcrumb_items
        .iter()
        .find(|(_, value)| *value == destination)
        .unwrap()
        .0;
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
}

#[test]
fn pane_focus_adds_one_level_and_sibling_switches_replace_it() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        let home = i18n::tr!(Repositories).to_owned();
        assert_eq!(labels(&s), std::slice::from_ref(&home));
        assert_eq!(render(&mut s, 120, 32).lines().next().unwrap().trim(), home);
        for section in Section::ALL {
            key(&mut s, KeyCode::Enter);
            wait_workspace(&mut s);
            let history_len = s.history.len();
            for sibling in Section::ALL {
                s.switch_repository_section(sibling);
                assert_eq!(
                    labels(&s),
                    [
                        home.clone(),
                        crate::view::name(&f.repo),
                        sibling.title().into()
                    ]
                );
                assert_eq!(s.history.len(), history_len);
            }
            s.switch_repository_section(section);
            key(&mut s, KeyCode::Esc);
            assert_eq!(s.page, Page::Detail);
            assert_eq!(labels(&s), [home.clone(), crate::view::name(&f.repo)]);
            key(&mut s, KeyCode::Esc);
            assert_eq!(s.page, Page::Repositories);
            assert_eq!(labels(&s), std::slice::from_ref(&home));
        }
        s.shutdown();
    }
}

#[test]
fn historical_details_add_one_level_and_breadcrumbs_restore_real_parent_sessions() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        support::write(f.repo.join("中文 文件.rs"), "fn example() {}\n");
        support::git(&f.repo, &["add", "."]);
        support::git(&f.repo, &["commit", "-m", "breadcrumb commit"]);
        let oid = support::git(&f.repo, &["rev-parse", "HEAD"]);
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        key(&mut s, KeyCode::Enter);
        wait_workspace(&mut s);
        command(&mut s, "repo log");
        wait(&mut s);
        let graph = labels(&s);
        assert_eq!(
            graph,
            [
                i18n::tr!(Repositories),
                &crate::view::name(&f.repo),
                i18n::tr!(HistoryTitle)
            ]
        );
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert_eq!(labels(&s).last().unwrap(), &oid[..8]);
        assert_eq!(labels(&s).len(), graph.len() + 1);
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert_eq!(labels(&s).last().unwrap(), "中文 文件.rs");
        assert_eq!(labels(&s).len(), graph.len() + 2);
        command(&mut s, "settings");
        assert_eq!(labels(&s).len(), graph.len() + 3);
        // 可直接返回已保存视图中的中间父级，而不是只返回整个文件差异。
        let target = s
            .breadcrumbs()
            .iter()
            .find(|crumb| crumb.label == oid[..8])
            .unwrap()
            .destination
            .unwrap();
        click_crumb(&mut s, target);
        assert!(
            matches!(&s.mode, Mode::History(view) if matches!(view.as_ref(), crate::history::View::Commit(_)))
        );
        assert_eq!(labels(&s).len(), graph.len() + 1);
        click_crumb(&mut s, Destination::Current(Level::History));
        assert_eq!(labels(&s), graph);
        click_crumb(
            &mut s,
            Destination::Saved {
                index: 1,
                level: Level::Page,
            },
        );
        assert_eq!(s.page, Page::Detail);
        click_crumb(&mut s, Destination::Home);
        assert_eq!(s.page, Page::Repositories);
        assert_eq!(s.selected_path(), Some(f.repo.clone()));
        assert!(s.history.is_empty());
        s.shutdown();
    }
}

#[test]
fn branch_and_stash_details_are_nested_and_single_back_restores_their_lists() {
    let f = support::Fixture::create();
    support::write(f.repo.join("stash.txt"), "change");
    support::git(&f.repo, &["stash", "push", "-u", "-m", "test stash"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    for (command_name, parent) in [
        ("repo branch", Level::Branches),
        ("repo stash", Level::Stashes),
    ] {
        command(&mut s, command_name);
        wait(&mut s);
        let list = labels(&s);
        assert_eq!(list.len(), 2);
        assert!(list[1].contains(&crate::view::name(&f.repo)));
        key(&mut s, KeyCode::Enter);
        assert_eq!(labels(&s).len(), 3);
        click_crumb(&mut s, Destination::Current(parent));
        assert_eq!(labels(&s), list);
        key(&mut s, KeyCode::Enter);
        key(&mut s, KeyCode::Esc);
        assert_eq!(labels(&s), list);
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.page, Page::Repositories);
    }
}

#[test]
fn embedded_detail_returns_to_its_focused_pane_and_commands_do_not_navigate_on_click() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    key(&mut s, KeyCode::Enter);
    wait_workspace(&mut s);
    s.switch_repository_section(Section::Branches);
    key(&mut s, KeyCode::Enter);
    assert_eq!(labels(&s).len(), 4);
    click_crumb(&mut s, Destination::Current(Level::Pane(Section::Branches)));
    assert_eq!(labels(&s).len(), 3);
    assert_eq!(Section::focused(&s.mode), Some(Section::Branches));
    key(&mut s, KeyCode::Char('/'));
    let draft = s.bar.text();
    click_crumb(&mut s, Destination::Home);
    assert_eq!(s.page, Page::Detail);
    assert_eq!(s.bar.text(), draft);
    key(&mut s, KeyCode::Esc);
    click_crumb(&mut s, Destination::Current(Level::Repository));
    assert_eq!(labels(&s).len(), 2);
    assert_eq!(Section::focused(&s.mode), None);
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.page, Page::Repositories);
    s.shutdown();
}

#[test]
fn configuration_and_help_follow_the_same_enter_and_return_path() {
    let (_home, mut s) = fixture(vec![]);
    command(&mut s, "settings");
    let settings = labels(&s);
    key(&mut s, KeyCode::Tab);
    assert_eq!(labels(&s).len(), settings.len() + 1);
    key(&mut s, KeyCode::F(1));
    assert_eq!(labels(&s).len(), settings.len() + 2);
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Esc);
    assert_eq!(labels(&s), settings);
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.page, Page::Repositories);
}

#[test]
fn narrow_headers_keep_the_current_file_visible_and_clip_only_display_labels() {
    use unicode_width::UnicodeWidthStr;
    let _language = i18n::using(i18n::Language::Chinese);
    let f = support::Fixture::create();
    let filename = format!("{}重点.rs", "中文目录".repeat(12));
    support::write(f.repo.join(&filename), "fn example() {}\n");
    support::git(&f.repo, &["add", "."]);
    support::git(&f.repo, &["commit", "-m", "long file"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    key(&mut s, KeyCode::Enter);
    wait_workspace(&mut s);
    command(&mut s, "repo log");
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    for width in [60, 100, 180] {
        let text = render(&mut s, width, 30);
        let header = text.lines().next().unwrap().trim();
        assert!(header.contains("重点.rs"), "{header}");
        assert!(header.width() <= width as usize);
        assert_eq!(labels(&s).last().unwrap(), &filename);
        for (area, _) in &s.breadcrumb_items {
            assert!(area.right() <= width);
        }
    }
    click_crumb(&mut s, Destination::Home);
    assert_eq!(s.page, Page::Repositories);
}

#[test]
fn maximized_siblings_keep_one_level_and_keyboard_and_mouse_return_to_the_same_root() {
    use crate::workspace::Arrangement;
    for back in [KeyCode::Esc, KeyCode::Left] {
        let f = support::Fixture::create();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        key(&mut s, KeyCode::Enter);
        wait_workspace(&mut s);
        let root = labels(&s);
        let visits = s.history.len();
        key(&mut s, KeyCode::Right);
        for section in Section::ALL {
            assert_eq!(Section::focused(&s.mode), Some(section));
            let focused = labels(&s);
            assert_eq!(focused.len(), root.len() + 1);
            s.key(
                KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
                &Cancel::default(),
            )
            .unwrap();
            assert_eq!(labels(&s), focused);
            assert_eq!(
                s.workspace.session().unwrap().arrangement,
                Arrangement::Maximized
            );
            s.key(
                KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
                &Cancel::default(),
            )
            .unwrap();
            assert_eq!(labels(&s).len(), focused.len());
            assert_eq!(s.history.len(), visits);
            let back = if Section::focused(&s.mode) == Some(Section::History) {
                KeyCode::Esc
            } else {
                back
            };
            key(&mut s, back);
            assert_eq!(labels(&s), root);
            assert_eq!(
                s.workspace.session().unwrap().arrangement,
                Arrangement::Tiled
            );
            s.switch_repository_section(section.advance(false));
            s.key(
                KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
                &Cancel::default(),
            )
            .unwrap();
            click_crumb(&mut s, Destination::Current(Level::Repository));
            assert_eq!(labels(&s), root);
            assert_eq!(
                s.workspace.session().unwrap().arrangement,
                Arrangement::Tiled
            );
            s.switch_repository_section(section.advance(false));
        }
        key(&mut s, back);
        key(&mut s, back);
        assert_eq!(s.page, Page::Repositories);
        s.shutdown();
    }
}

#[test]
fn loading_and_failed_panes_have_the_same_scope_and_return_without_a_late_takeover() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    key(&mut s, KeyCode::Enter);
    s.switch_repository_section(Section::Worktree);
    assert!(matches!(s.mode, Mode::Panel(Section::Worktree)));
    assert_eq!(labels(&s).len(), 3);
    key(&mut s, KeyCode::Esc);
    wait_workspace(&mut s);
    assert!(matches!(s.mode, Mode::Browsing));
    assert_eq!(labels(&s).len(), 2);
    s.switch_repository_section(Section::Worktree);
    assert!(matches!(s.mode, Mode::Files(_)));
    assert_eq!(labels(&s).len(), 3);
    s.open_repository(f.repo.join("missing-repository"));
    s.switch_repository_section(Section::History);
    wait_workspace(&mut s);
    assert!(matches!(
        s.workspace.session().unwrap().history,
        crate::workspace::Panel::Failed(_)
    ));
    assert_eq!(labels(&s).len(), 3);
    click_crumb(&mut s, Destination::Current(Level::Repository));
    assert_eq!(labels(&s).len(), 2);
    assert!(matches!(s.mode, Mode::Browsing));
    s.shutdown();
}

#[test]
fn nested_history_and_saved_ancestors_restore_pane_scope_and_selected_commit() {
    let f = support::Fixture::create();
    f.commit("example.rs", "fn example() {}\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    key(&mut s, KeyCode::Enter);
    wait_workspace(&mut s);
    s.switch_repository_section(Section::History);
    let selected = match &s.mode {
        Mode::History(view) => view.session().selected().unwrap().oid,
        _ => unreachable!(),
    };
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(labels(&s).len(), 4);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(labels(&s).len(), 5);
    command(&mut s, "settings");
    let target = s
        .breadcrumbs()
        .into_iter()
        .find(|crumb| crumb.label == Section::History.title())
        .unwrap()
        .destination
        .unwrap();
    click_crumb(&mut s, target);
    assert_eq!(labels(&s).len(), 3);
    assert!(
        matches!(&s.mode, Mode::History(view) if matches!(view.as_ref(), crate::history::View::Graph(_)) && view.session().selected().unwrap().oid == selected)
    );
    key(&mut s, KeyCode::Char('/'));
    assert_eq!(s.bar.text(), "/repo log ");
    key(&mut s, KeyCode::Esc);
    command(&mut s, "settings");
    let target = s
        .breadcrumbs()
        .into_iter()
        .find(|crumb| crumb.label == crate::view::name(&f.repo))
        .unwrap()
        .destination
        .unwrap();
    click_crumb(&mut s, target);
    assert_eq!(labels(&s).len(), 2);
    assert!(matches!(s.mode, Mode::Browsing));
    key(&mut s, KeyCode::Char('/'));
    assert_eq!(s.bar.text(), "/repo ");
    key(&mut s, KeyCode::Esc);
    s.switch_repository_section(Section::History);
    assert!(
        matches!(&s.mode, Mode::History(view) if view.session().selected().unwrap().oid == selected)
    );
    s.shutdown();
}

#[test]
fn leaving_pane_and_repository_preserves_the_home_search_without_extra_escape_steps() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    s.paste("repo");
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.page, Page::Detail);
    assert!(s.bar.editor().text.is_empty());
    wait_workspace(&mut s);
    s.switch_repository_section(Section::Branches);
    s.paste("main");
    s.paste("//settings");
    key(&mut s, KeyCode::Enter);
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.bar.editor().text, "main");
    click_crumb(&mut s, Destination::Current(Level::Repository));
    assert!(s.bar.editor().text.is_empty());
    s.switch_repository_section(Section::Branches);
    assert_eq!(s.bar.editor().text, "main");
    key(&mut s, KeyCode::Esc);
    assert_eq!(labels(&s).len(), 3);
    key(&mut s, KeyCode::Esc);
    assert_eq!(labels(&s).len(), 2);
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.page, Page::Repositories);
    assert_eq!(s.bar.editor().text, "repo");
    assert_eq!(s.query, "repo");
    assert!(s.history.is_empty());
    s.shutdown();
}
