use super::*;
use crate::{file_icons::Display, settings::Item};
use std::path::Path;

fn icon(path: &str) -> ratatui::text::Span<'static> {
    Display::Enabled.file(Path::new(path)).remove(0)
}
fn frame(s: &mut Screen) -> ratatui::buffer::Buffer {
    let mut terminal = Terminal::new(TestBackend::new(120, 32)).unwrap();
    terminal.draw(|frame| view::render(frame, s)).unwrap();
    terminal.backend().buffer().clone()
}
fn has_icon(buffer: &ratatui::buffer::Buffer, icon: &ratatui::text::Span<'_>) -> bool {
    buffer
        .content
        .iter()
        .any(|cell| cell.symbol() == icon.content.as_ref() && Some(cell.fg) == icon.style.fg)
}

#[test]
fn icon_toggle_is_localized_persistent_and_browser_slots_keep_their_width() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        f.commit("中文.ts", "export const value = 1;\n");
        std::fs::create_dir(f.repo.join("目录")).unwrap();
        f.commit("目录/child.txt", "child\n");
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        assert!(s.config.file_icons);
        command(&mut s, "repo files");
        browser::ready(&mut s);
        browser::select(&mut s, "中文.ts");
        let glyph = icon("中文.ts");
        let enabled = frame(&mut s);
        assert!(has_icon(&enabled, &glyph));
        let name = enabled
            .content
            .iter()
            .position(|cell| {
                cell.symbol() == glyph.content.as_ref() && Some(cell.fg) == glyph.style.fg
            })
            .unwrap()
            + 2;
        assert_eq!(enabled.content[name].symbol(), "中");
        assert_eq!(enabled.content[name - 2].symbol(), glyph.content.as_ref());
        assert_eq!(enabled.content[name - 4].symbol(), " ");
        let folder = Display::Enabled.directory().remove(0);
        let folder_index = enabled
            .content
            .iter()
            .position(|cell| cell.symbol() == folder.content.as_ref())
            .unwrap();
        assert_eq!(enabled.content[folder_index - 2].symbol(), "▶");
        assert_eq!(folder_index % 120, (name - 2) % 120);
        assert_eq!(enabled.content[folder_index + 2].symbol(), "目");
        command(&mut s, "settings");
        let items = Configuration::General.items(s.snapshot.registration, &s.config.ai);
        s.setting_selected = items
            .iter()
            .position(|item| *item == Item::FileIcons)
            .unwrap();
        let settings = render(&mut s, 100, 36);
        assert!(settings.contains(i18n::tr!(FileIcons)));
        assert_eq!(
            settings
                .matches(&format!(" {} ", i18n::tr!(SettingsDisplay)))
                .count(),
            1
        );
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert!(!s.config.file_icons && !s.service.store.config().unwrap().file_icons);
        command(&mut s, "repo files");
        browser::ready(&mut s);
        browser::select(&mut s, "中文.ts");
        let disabled = frame(&mut s);
        assert!(!has_icon(&disabled, &glyph));
        assert_eq!(disabled.content[name].symbol(), "中");
        assert_eq!(disabled.content[name - 2].symbol(), " ");
        assert!(!has_icon(&disabled, &folder));
        assert_eq!(disabled.content[folder_index - 2].symbol(), "▶");
        assert_eq!(disabled.content[folder_index + 2].symbol(), "目");
        browser::select(&mut s, "目录");
        key(&mut s, KeyCode::Enter);
        browser::ready(&mut s);
        assert!(browser::session(&mut s).expanded(Path::new("目录")));
        assert!(
            frame(&mut s)
                .content
                .iter()
                .any(|cell| cell.symbol() == "▼")
        );
        s.shutdown();
    }
}

#[test]
fn worktree_and_commit_files_share_icons_without_overwriting_search_or_git_state() {
    let f = support::Fixture::create();
    std::fs::create_dir(f.repo.join("目录")).unwrap();
    f.commit("目录/中文.ts", "export const value = 1;\n");
    support::write(f.repo.join("目录/中文.ts"), "export const value = 2;\n");
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    open_files(&mut s);
    let glyph = icon("中文.ts");
    assert!(has_icon(&frame(&mut s), &glyph));
    s.paste("中文");
    let buffer = frame(&mut s);
    assert!(has_icon(&buffer, &glyph));
    assert!(buffer.content.iter().any(|cell| cell.symbol() == "中"
        && cell.bg == ratatui::style::Color::Rgb(124, 83, 24)));
    command(&mut s, "repo diff view flat");
    assert!(has_icon(&frame(&mut s), &glyph));
    command(&mut s, "repo log");
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(has_icon(&frame(&mut s), &glyph));
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert_eq!(
        std::fs::read_to_string(f.repo.join("目录/中文.ts")).unwrap(),
        "export const value = 2;\n"
    );
    s.shutdown();
}
