use super::*;
use ratatui::style::Color;

#[test]
fn highlighted_diff_keeps_styles_scrolling_and_navigation_after_resizing() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        f.commit("中文 source.rs", "fn old() { let s = \"旧内容\"; }\n");
        let mut source = "fn changed() { let s = \"新内容\"; }\n".to_owned();
        source.push_str(&"// many lines\n".repeat(80));
        source.push_str("fn last_line() {}\n");
        support::write(f.repo.join("中文 source.rs"), &source);
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        command(&mut s, "stage");
        wait(&mut s);
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert!(matches!(s.mode, Mode::Diff { .. }));
        for (width, height) in [(100, 28), (60, 18), (150, 32)] {
            let Mode::Diff { scroll, .. } = &mut s.mode else {
                unreachable!()
            };
            *scroll = 0;
            let mut text = render(&mut s, width, height);
            assert!(text.contains("Rust"), "{text}");
            for _ in 0..12 {
                if text.contains("+fn changed()") {
                    break;
                }
                key(&mut s, KeyCode::Down);
                text = render(&mut s, width, height);
            }
            assert!(text.contains("+fn changed()"), "{text}");
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
            let cells = &terminal.backend().buffer().content;
            assert!(cells.iter().any(|cell| cell.bg == crate::diff::ADDED));
            assert!(cells.iter().any(|cell| cell.bg == crate::diff::REMOVED));
            let source_colors = cells
                .iter()
                .filter(|cell| cell.bg == crate::diff::ADDED && matches!(cell.fg, Color::Rgb(..)))
                .map(|cell| cell.fg)
                .collect::<std::collections::HashSet<_>>();
            assert!(source_colors.len() >= 3);
            let Mode::Diff { scroll, .. } = &mut s.mode else {
                unreachable!()
            };
            *scroll = usize::MAX;
            assert!(render(&mut s, width, height).contains("last_line"));
            key(&mut s, KeyCode::PageUp);
            assert!(!render(&mut s, width, height).contains("last_line"));
        }
        key(&mut s, KeyCode::Left);
        assert!(matches!(s.mode, Mode::Files(_)));
        assert!(support::git(&f.repo, &["diff", "--cached"]).is_empty());
        assert!(s.service.store.report().unwrap().is_none());
    }
}

#[test]
fn layout_commands_complete_without_confirmation_and_reuse_the_open_diff_snapshot() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        f.commit("one.rs", "fn old() {}\n");
        support::write(f.repo.join("one.rs"), "fn cached_new() {}\n");
        support::write(f.repo.join("two.rs"), "fn second() {}\n");
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        let before = std::fs::read(s.service.store.home.join("config.json")).unwrap();
        command(&mut s, "stage");
        wait(&mut s);
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        let text = render(&mut s, 160, 32);
        assert!(
            text.contains(&i18n::tf!(DiffBefore, i18n::tr!(Index))),
            "{text}"
        );
        assert!(
            text.contains(&i18n::tf!(DiffAfter, i18n::tr!(Worktree))),
            "{text}"
        );
        assert!(text.contains(&format!(
            "{}: {}",
            i18n::tr!(DiffAuto),
            i18n::tr!(DiffSplit)
        )));
        s.paste("/diff ");
        let menu = render(&mut s, 160, 32);
        for name in ["diff auto", "diff split", "diff unified"] {
            assert!(menu.contains(name));
        }
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Tab);
        assert_eq!(s.bar.editor().text, "/diff split");
        support::write(f.repo.join("one.rs"), "fn changed_after_snapshot() {}\n");
        key(&mut s, KeyCode::Enter);
        assert!(matches!(s.mode, Mode::Diff { .. }));
        assert_eq!(s.diff_preference, crate::diff::Preference::Split);
        let narrow = render(&mut s, 80, 28);
        assert!(narrow.contains("cached_new"), "{narrow}");
        assert!(!narrow.contains("changed_after_snapshot"));
        assert!(narrow.contains(&i18n::tf!(DiffBefore, i18n::tr!(Index))));
        command(&mut s, "diff unified");
        assert!(matches!(s.mode, Mode::Diff { .. }));
        let wide = render(&mut s, 160, 32);
        assert!(wide.contains(&format!("{} → {}", i18n::tr!(Index), i18n::tr!(Worktree))));
        command(&mut s, "diff auto");
        let narrow = render(&mut s, 80, 28);
        assert!(narrow.contains(&format!(
            "{}: {}",
            i18n::tr!(DiffAuto),
            i18n::tr!(DiffUnified)
        )));
        assert!(render(&mut s, 160, 32).contains(&format!(
            "{}: {}",
            i18n::tr!(DiffAuto),
            i18n::tr!(DiffSplit)
        )));
        command(&mut s, "diff unified");
        key(&mut s, KeyCode::Left);
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        let second = render(&mut s, 160, 32);
        assert!(second.contains("second"));
        assert_eq!(s.diff_preference, crate::diff::Preference::Unified);
        assert_eq!(
            std::fs::read(s.service.store.home.join("config.json")).unwrap(),
            before
        );
        assert!(support::git(&f.repo, &["diff", "--cached"]).is_empty());
        assert!(s.service.store.report().unwrap().is_none());
    }
}

#[test]
fn staged_comparison_labels_head_and_index_and_layout_selection_is_session_only() {
    let f = support::Fixture::create();
    f.commit("stage.rs", "fn head() {}\n");
    support::write(f.repo.join("stage.rs"), "fn index() {}\n");
    support::git(&f.repo, &["add", "stage.rs"]);
    support::write(f.repo.join("stage.rs"), "fn worktree() {}\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "diff split");
    assert!(matches!(s.mode, Mode::Browsing));
    command(&mut s, "unstage");
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    let text = render(&mut s, 160, 32);
    assert!(text.contains(&i18n::tf!(DiffBefore, "HEAD")));
    assert!(text.contains(&i18n::tf!(DiffAfter, i18n::tr!(Index))));
    assert!(text.contains("fn head") && text.contains("fn index"));
    assert!(!text.contains("fn worktree"));
    assert_eq!(
        screen(Service::new(
            s.service.store.home.clone(),
            s.service.scheduler.clone()
        ))
        .diff_preference,
        crate::diff::Preference::Auto
    );
    let cached = support::git(&f.repo, &["diff", "--cached"]);
    assert!(cached.contains("+fn index"));
}

#[test]
fn narrow_forced_split_is_explicit_and_restores_the_view_when_enlarged() {
    let _language = i18n::using(i18n::Language::English);
    let f = support::Fixture::create();
    support::write(f.repo.join("narrow.rs"), "fn visible_again() {}\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "stage");
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    command(&mut s, "diff split");
    assert!(render(&mut s, 160, 32).contains("visible_again"));
    let narrow = render(&mut s, 12, 18);
    assert!(narrow.contains("resize"), "{narrow}");
    assert!(matches!(s.mode, Mode::Diff { .. }));
    assert!(render(&mut s, 160, 32).contains("visible_again"));
}

#[test]
fn common_language_diffs_have_real_token_colors_in_both_layouts() {
    use ratatui::buffer::Buffer;
    fn token_color(buffer: &Buffer, word: &str) -> Color {
        let chars = word.chars().map(|ch| ch.to_string()).collect::<Vec<_>>();
        for row in buffer.content.chunks(buffer.area.width as usize) {
            for (column, cells) in row.windows(chars.len()).enumerate() {
                if cells
                    .iter()
                    .zip(&chars)
                    .all(|(cell, ch)| cell.symbol() == ch)
                    && row[column].bg == crate::diff::ADDED
                {
                    return row[column].fg;
                }
            }
        }
        panic!("新增代码中没有找到 {word}");
    }
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        for (file, before, after, syntax, keyword) in [
            (
                "types.ts",
                "export type Mode = \"item\";\n",
                "export type Mode = \"toolbar\" | \"item\"; // comment\n",
                "TypeScript",
                "export",
            ),
            (
                "view.tsx",
                "export const View = () => <button />;\n",
                "export const View = () => <button title=\"toolbar\" />; // comment\n",
                "TypeScriptReact",
                "export",
            ),
            (
                "view.jsx",
                "export const View = () => <button />;\n",
                "export const View = () => <button title=\"toolbar\" />; // comment\n",
                "JavaScript (Babel)",
                "export",
            ),
            (
                "main.go",
                "package main\nconst message = \"item\"\n",
                "package main\nconst message = \"toolbar\" // comment\n",
                "Go",
                "const",
            ),
            (
                "main.py",
                "message = \"item\"\n",
                "if True: message = \"toolbar\" # comment\n",
                "Python",
                "if",
            ),
            (
                "config.jsonc",
                "{\"enabled\": false}\n",
                "{\"enabled\": true, \"message\": \"toolbar\"} // comment\n",
                "JSONC",
                "true",
            ),
            (
                "config.json5",
                "{enabled: false}\n",
                "{enabled: true, message: 'toolbar'} // comment\n",
                "JSON5",
                "true",
            ),
            (
                "Page.astro",
                "---\nconst message = \"item\";\n---\n<button />\n",
                "---\nconst message: string = \"toolbar\"; // comment\n---\n<button title=\"save\">{message}</button>\n",
                "Astro",
                "const",
            ),
            (
                "App.vue",
                "<script lang=\"ts\">\nconst message = \"item\";\n</script>\n",
                "<script lang=\"ts\">\nconst message: string = \"toolbar\"; // comment\n</script>\n",
                "Vue Component",
                "const",
            ),
            (
                "App.svelte",
                "<script lang=\"ts\">\nconst message = \"item\";\n</script>\n",
                "<script lang=\"ts\">\nconst message: string = \"toolbar\"; // comment\n</script>\n",
                "Svelte",
                "const",
            ),
        ] {
            let f = support::Fixture::create();
            f.commit(file, before);
            support::write(f.repo.join(file), after);
            let (_home, mut s) = fixture(vec![f.repo.clone()]);
            command(&mut s, "stage");
            wait(&mut s);
            key(&mut s, KeyCode::Enter);
            wait(&mut s);
            assert!(matches!(s.mode, Mode::Diff { .. }));
            for width in [120, 180] {
                let rendered = render(&mut s, width, 32);
                assert!(rendered.contains(syntax), "{file}: {rendered}");
                assert!(!rendered.contains(i18n::tr!(SyntaxUnknown)));
                let mut terminal = Terminal::new(TestBackend::new(width, 32)).unwrap();
                terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
                let buffer = terminal.backend().buffer();
                assert_ne!(token_color(buffer, keyword), token_color(buffer, "toolbar"));
                assert_ne!(
                    token_color(buffer, "comment"),
                    token_color(buffer, "toolbar")
                );
            }
            assert!(support::git(&f.repo, &["diff", "--cached"]).is_empty());
            assert!(s.service.store.report().unwrap().is_none());
        }
    }
}
