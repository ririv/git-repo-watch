use super::*;
use crate::workspace::{Panel, Section};

#[test]
fn repository_root_arrows_select_equal_panes_without_operating_their_content() {
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        support::write(f.repo.join("a.ts"), "export const a = 1;\n");
        support::write(f.repo.join("b.ts"), "export const b = 2;\n");
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        key(&mut s, KeyCode::Enter);
        assert!(matches!(s.mode, Mode::Browsing));
        // 加载完成也只更新内容，不把尚未聚焦的面板变成活动内容。
        key(&mut s, KeyCode::Right);
        wait_workspace(&mut s);
        assert!(matches!(s.mode, Mode::Browsing));
        let session = s.workspace.session().unwrap();
        assert_eq!(session.selection, Section::Branches);
        let Panel::Ready(files) = &session.files else {
            panic!("files loaded")
        };
        let cursor = files.cursor.clone();
        for (width, height) in [(160, 36), (60, 18), (24, 12)] {
            for (code, selected) in [
                (KeyCode::Left, Section::Overview),
                (KeyCode::Up, Section::Overview),
                (KeyCode::Down, Section::Worktree),
                (KeyCode::Left, Section::Worktree),
                (KeyCode::Right, Section::History),
                (KeyCode::Down, Section::History),
                (KeyCode::Up, Section::Branches),
                (KeyCode::Right, Section::Branches),
            ] {
                key(&mut s, code);
                assert!(matches!(s.mode, Mode::Browsing));
                assert_eq!(s.workspace.session().unwrap().selection, selected);
                assert_eq!(s.breadcrumbs().len(), 2);
                assert_eq!(s.scroll, 0);
                let Panel::Ready(files) = &s.workspace.session().unwrap().files else {
                    panic!("files loaded")
                };
                assert_eq!(files.cursor, cursor);
                render(&mut s, width, height);
                if width < 60 {
                    assert_eq!(s.workspace_areas, vec![(s.body_area.unwrap(), selected)]);
                } else {
                    assert_eq!(s.workspace_areas.len(), 4);
                }
            }
        }
        assert!(render(&mut s, 160, 36).contains(i18n::tr!(RepositoryPanesHint)));
        key(&mut s, KeyCode::Tab);
        assert_eq!(s.workspace.session().unwrap().selection, Section::Worktree);
        key(&mut s, KeyCode::BackTab);
        assert_eq!(s.workspace.session().unwrap().selection, Section::Branches);
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.page, Page::Repositories);
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        s.shutdown();
    }
}

#[test]
fn entering_and_leaving_each_pane_preserves_root_selection_and_content_keys() {
    let f = support::Fixture::create();
    support::write(f.repo.join("a.ts"), "export const a = 1;\n");
    support::write(f.repo.join("b.ts"), "export const b = 2;\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    key(&mut s, KeyCode::Enter);
    wait_workspace(&mut s);
    for section in Section::ALL {
        s.workspace.session_mut().unwrap().selection = section;
        key(&mut s, KeyCode::Enter);
        assert_eq!(Section::focused(&s.mode), Some(section));
        assert_eq!(s.breadcrumbs().len(), 3);
        if section == Section::Worktree {
            let before = s.mode.file_session().unwrap().cursor.clone();
            key(&mut s, KeyCode::Down);
            assert_ne!(s.mode.file_session().unwrap().cursor, before);
        }
        key(&mut s, KeyCode::Esc);
        assert!(matches!(s.mode, Mode::Browsing));
        assert_eq!(s.page, Page::Detail);
        assert_eq!(s.breadcrumbs().len(), 2);
        assert_eq!(s.workspace.session().unwrap().selection, section);
        command(&mut s, "settings");
        key(&mut s, KeyCode::Esc);
        assert!(matches!(s.mode, Mode::Browsing));
        assert_eq!(s.workspace.session().unwrap().selection, section);
    }
    s.shutdown();
}
