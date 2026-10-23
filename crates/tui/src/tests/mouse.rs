use super::*;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Position;

fn event(s: &mut Screen, kind: MouseEventKind, x: u16, y: u16) -> bool {
    s.mouse(
        MouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        },
        &Cancel::default(),
    )
    .unwrap()
}
fn click(s: &mut Screen, x: u16, y: u16) -> bool {
    event(s, MouseEventKind::Down(MouseButton::Left), x, y)
}

#[test]
fn repository_clicks_use_visible_offsets_and_ignore_headers_blanks_and_disabled_input() {
    let paths = (0..30)
        .map(|i| PathBuf::from(format!("/example/repository-{i:02}")))
        .collect();
    let (_home, mut s) = fixture(paths);
    for (width, height) in [(60, 18), (160, 32)] {
        key(&mut s, KeyCode::End);
        render(&mut s, width, height);
        let area = s.repository_area.unwrap();
        let offset = s.repository_viewport.offset();
        click(&mut s, area.x + 3, area.y + 3);
        assert_eq!(s.selected(), offset + 1);
        assert_eq!(s.page, Page::Repositories);
        assert!(!click(&mut s, area.x + 3, area.y + 1));
        assert_eq!(s.selected(), offset + 1);
        s.config.mouse.precise_click = false;
        assert!(!click(&mut s, area.x + 3, area.y + 2));
        assert_eq!(s.selected(), offset + 1);
        s.config.mouse.precise_click = true;
        click(&mut s, area.right() - 1, area.bottom() - 2);
        assert_eq!(s.selected(), 29);
        render(&mut s, width, height);
        click(&mut s, area.right() - 1, area.y + 2);
        assert_eq!(s.selected(), 0);
        s.config.mouse.scroll = false;
        assert!(!event(
            &mut s,
            MouseEventKind::ScrollDown,
            area.x + 3,
            area.y + 3
        ));
        assert_eq!(s.selected(), 0);
        key(&mut s, KeyCode::Down);
        assert_eq!(s.selected(), 1);
        s.config.mouse.scroll = true;
    }
}

#[test]
fn grouped_files_click_the_rendered_row_without_staging_or_opening_a_new_view() {
    let f = support::Fixture::create();
    for i in 0..20 {
        support::write(f.repo.join(format!("staged-{i:02}.rs")), "fn staged() {}\n");
    }
    support::git(&f.repo, &["add", "."]);
    for i in 0..20 {
        support::write(
            f.repo.join(format!("unstaged-{i:02}.rs")),
            "fn unstaged() {}\n",
        );
    }
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    s.key(
        KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL),
        &Cancel::default(),
    )
    .unwrap();
    for (width, height) in [(60, 24), (160, 32)] {
        key(&mut s, KeyCode::End);
        let text = render(&mut s, width, height);
        let y = text
            .lines()
            .position(|line| line.contains("staged-18.rs") && !line.contains("unstaged"))
            .unwrap() as u16;
        assert!(click(&mut s, 5, y));
        let Mode::Changes(session) = &s.mode else {
            panic!("changes");
        };
        assert!(
            session.files.rows()[session.files.cursor]
                .location
                .display()
                .contains("staged-18.rs")
        );
        assert!(session.files.selected.is_empty());
        let header = text
            .lines()
            .position(|line| line.contains(i18n::tr!(ChangesStaged)))
            .unwrap() as u16;
        click(&mut s, 5, header);
        let Mode::Changes(session) = &s.mode else {
            panic!("changes");
        };
        assert!(
            session.files.rows()[session.files.cursor]
                .location
                .display()
                .contains("staged-18.rs")
        );
    }
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    s.shutdown();
}

#[test]
fn pane_selection_click_selection_and_wheel_preferences_are_independent() {
    let f = support::Fixture::create();
    f.commit("tracked", "tracked\n");
    support::write(f.repo.join("pending"), "pending\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    key(&mut s, KeyCode::Enter);
    wait_workspace(&mut s);
    render(&mut s, 150, 36);
    let files = s
        .workspace_areas
        .iter()
        .find(|(_, section)| *section == crate::workspace::Section::Worktree)
        .unwrap()
        .0;
    let branches = s
        .workspace_areas
        .iter()
        .find(|(_, section)| *section == crate::workspace::Section::Branches)
        .unwrap()
        .0;
    s.config.mouse.select_pane = false;
    assert!(!click(&mut s, files.x + 2, files.y + 2));
    assert_eq!(crate::workspace::Section::focused(&s.mode), None);
    s.config.mouse.select_pane = true;
    s.config.mouse.precise_click = false;
    click(&mut s, files.x + 2, files.y + 2);
    assert_eq!(
        crate::workspace::Section::focused(&s.mode),
        Some(crate::workspace::Section::Worktree)
    );
    s.config.mouse.scroll = false;
    assert!(!event(
        &mut s,
        MouseEventKind::ScrollDown,
        branches.x + 2,
        branches.y + 2
    ));
    assert!(!event(
        &mut s,
        MouseEventKind::Moved,
        branches.x + 2,
        branches.y + 2
    ));
    assert_eq!(
        crate::workspace::Section::focused(&s.mode),
        Some(crate::workspace::Section::Worktree)
    );
    s.config.mouse.scroll = true;
    event(
        &mut s,
        MouseEventKind::ScrollDown,
        branches.x + 2,
        branches.y + 2,
    );
    assert_eq!(
        crate::workspace::Section::focused(&s.mode),
        Some(crate::workspace::Section::Branches)
    );
    s.shutdown();
}

#[test]
fn settings_group_is_bilingual_and_each_toggle_persists_without_new_commands() {
    use engine::state::MouseSetting;
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        command(&mut s, "settings");
        for width in [60, 140] {
            let text = render(&mut s, width, 24);
            assert!(text.contains(i18n::tr!(MouseSettings)));
            assert!(text.contains(i18n::tr!(MouseScroll)));
        }
        for setting in [
            MouseSetting::PreciseClick,
            MouseSetting::Scroll,
            MouseSetting::SelectPane,
        ] {
            let index = Configuration::General
                .items(s.snapshot.registration)
                .iter()
                .position(|item| *item == crate::settings::Item::Mouse(setting))
                .unwrap();
            s.setting_selected = index;
            key(&mut s, KeyCode::Enter);
            wait(&mut s);
            assert!(!s.config.mouse.enabled(setting));
            assert!(!s.service.store.config().unwrap().mouse.enabled(setting));
            assert_eq!(s.page, Page::Configuration(Configuration::General));
            assert!(matches!(s.mode, Mode::Browsing));
        }
        assert!(command::matches("settings mouse").is_empty());
        assert!(
            std::fs::read_to_string(s.service.store.config_path())
                .unwrap()
                .contains("[mouse]")
        );
    }
}

#[test]
fn hit_map_handles_filtered_indices_and_excludes_headers_and_empty_rows() {
    use crate::pointer::{Action, Map, Target};
    use ratatui::layout::Rect;
    let mut map = Map::default();
    map.list(
        Rect::new(0, 0, 20, 10),
        Rect::new(1, 3, 18, 5),
        &[2, 7, 11],
        1,
        Target::Files,
    );
    assert_eq!(
        map.hit(Position::new(2, 3)),
        Some(Action::Select(Target::Files, 7))
    );
    assert_eq!(
        map.hit(Position::new(2, 4)),
        Some(Action::Select(Target::Files, 11))
    );
    assert_eq!(map.hit(Position::new(2, 2)), None);
    assert_eq!(map.hit(Position::new(2, 5)), None);
    assert_eq!(map.hit(Position::new(19, 3)), None);
    map.clear();
    assert_eq!(map.hit(Position::new(2, 3)), None);
}

#[test]
fn diff_tracks_seek_both_axes_and_respect_the_click_toggle() {
    let f = support::Fixture::create();
    let source = (0..80)
        .map(|i| format!("// {i} {}TAIL\n", "中文".repeat(100)))
        .collect::<String>();
    support::write(f.repo.join("long.rs"), &source);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    maximize_preview(&mut s);
    for preference in [
        crate::diff::Preference::Unified,
        crate::diff::Preference::Split,
    ] {
        s.diff_preference = preference;
        render(&mut s, 100, 24);
        let area = s.body_area.unwrap();
        click(&mut s, area.right() - 2, area.bottom() - 1);
        click(&mut s, area.right() - 1, area.bottom() - 2);
        let Mode::Changes(session) = &s.mode else {
            panic!("changes");
        };
        let crate::changes::Preview::Ready {
            content, scroll, ..
        } = &session.preview
        else {
            panic!("preview");
        };
        let crate::document::View::Text(view) = content.as_ref() else {
            panic!("text");
        };
        assert!(view.horizontal.offset() > 0);
        assert!(*scroll > 0);
        let text = render(&mut s, 100, 24);
        assert!(text.contains("TAIL"), "{text}");
        s.config.mouse.precise_click = false;
        assert!(!click(&mut s, area.right() - 1, area.y + 2));
        s.config.mouse.precise_click = true;
        click(&mut s, area.right() - 1, area.y + 2);
        let Mode::Changes(session) = &s.mode else {
            panic!("changes");
        };
        let crate::changes::Preview::Ready { scroll, .. } = &session.preview else {
            panic!("preview");
        };
        assert_eq!(*scroll, 0);
    }
    assert_eq!(
        std::fs::read_to_string(f.repo.join("long.rs")).unwrap(),
        source
    );
    s.shutdown();
}

#[test]
fn history_clicks_select_the_commit_at_the_drawn_row_after_scrolling() {
    let f = support::Fixture::create();
    for i in 0..35 {
        f.commit("tracked", &format!("{i}\n"));
    }
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    key(&mut s, KeyCode::End);
    render(&mut s, 150, 24);
    let area = s.body_area.unwrap();
    let Mode::History(view) = &s.mode else {
        panic!("history");
    };
    let crate::history::View::Graph(session) = view.as_ref() else {
        panic!("graph");
    };
    let expected = session.records[session.state.offset() + 1].entry.oid;
    click(&mut s, area.x + 10, area.y + 4);
    let Mode::History(view) = &s.mode else {
        panic!("history");
    };
    let crate::history::View::Graph(session) = view.as_ref() else {
        panic!("graph");
    };
    assert_eq!(session.selected().unwrap().oid, expected);
    assert!(!click(&mut s, area.x + 10, area.y + 1));
    s.shutdown();
}
