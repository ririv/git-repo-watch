use super::*;
use ratatui::style::Color;

#[test]
fn highlighted_diff_keeps_styles_scrolling_and_navigation_after_resizing() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        std::fs::create_dir(f.repo.join(".claude")).unwrap();
        f.commit(
            ".claude/中文 source.rs",
            "fn old() { let s = \"旧内容\"; }\n",
        );
        let mut source = "fn changed() { let s = \"新内容\"; }\n".to_owned();
        source.push_str(&"// many lines\n".repeat(80));
        source.push_str("fn last_line() {}\n");
        support::write(f.repo.join(".claude/中文 source.rs"), &source);
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        open_files(&mut s);
        wait(&mut s);
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        maximize_preview(&mut s);
        assert!(matches!(s.mode, Mode::Changes(_)));
        for (width, height) in [(100, 28), (60, 18), (150, 32)] {
            let Mode::Changes(session) = &mut s.mode else {
                unreachable!()
            };
            let crate::changes::Preview::Ready { scroll, .. } = &mut session.preview else {
                unreachable!()
            };
            *scroll = 0;
            command(&mut s, "repo diff select line");
            key(&mut s, KeyCode::Home);
            key(&mut s, KeyCode::Down);
            let text = render(&mut s, width, height);
            assert!(text.contains(i18n::tr!(FileDiff)), "{text}");
            assert!(text.contains("+fn changed()"), "{text}");
            let lines = text.lines().collect::<Vec<_>>();
            let header = lines
                .iter()
                .position(|line| line.contains(i18n::tr!(FileDiff)))
                .unwrap();
            let information = &lines[header + 1..header + 4];
            assert_eq!(
                information[0].trim_matches('│').trim(),
                ".claude/中文 source.rs",
                "{text}"
            );
            assert!(information[1].contains("Rust"), "{text}");
            assert!(information[2].starts_with('├'), "{text}");
            assert!(information[2].contains(i18n::tr!(Index)), "{text}");
            assert!(information[2].contains(i18n::tr!(Worktree)), "{text}");
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
            let buffer = terminal.backend().buffer();
            let path_y = header as u16 + 1;
            let directory = &buffer[(1, path_y)];
            assert_eq!(directory.fg, Color::White);
            assert!(!directory.modifier.contains(ratatui::style::Modifier::BOLD));
            let filename = &buffer[(1 + ".claude/".len() as u16, path_y)];
            assert_eq!(filename.fg, Color::White);
            assert!(!filename.modifier.contains(ratatui::style::Modifier::BOLD));
            let syntax_x = 3 + unicode_width::UnicodeWidthStr::width(i18n::tr!(SyntaxLabel)) as u16;
            assert_eq!(buffer[(syntax_x, path_y + 1)].fg, Color::Reset);
            let divider_y = header as u16 + 3;
            let horizontal = (1..width - 1)
                .map(|x| &buffer[(x, divider_y)])
                .filter(|cell| matches!(cell.symbol(), "─" | "┬"))
                .collect::<Vec<_>>();
            assert!(!horizontal.is_empty());
            for cell in horizontal {
                assert_eq!(cell.fg, Color::DarkGray, "标签颜色不应扩散到横线");
                assert!(!cell.modifier.contains(ratatui::style::Modifier::BOLD));
            }
            if let Some(x) = (1..width - 1).find(|x| buffer[(*x, divider_y)].symbol() == "┬") {
                let vertical = (divider_y + 1..height)
                    .map(|y| &buffer[(x, y)])
                    .find(|cell| cell.symbol() == "│")
                    .expect("双栏正文应存在中间竖线");
                assert_eq!(vertical.fg, buffer[(x, divider_y)].fg);
                assert!(!vertical.modifier.contains(ratatui::style::Modifier::BOLD));
            }
            let cells = &terminal.backend().buffer().content;
            assert!(cells.iter().any(|cell| cell.bg == view::SELECTION));
            let source_colors = cells
                .iter()
                .filter(|cell| cell.bg == crate::diff::ADDED && matches!(cell.fg, Color::Rgb(..)))
                .map(|cell| cell.fg)
                .collect::<std::collections::HashSet<_>>();
            assert!(source_colors.len() >= 3);
            let Mode::Changes(session) = &mut s.mode else {
                unreachable!()
            };
            let crate::changes::Preview::Ready { scroll, .. } = &mut session.preview else {
                unreachable!()
            };
            *scroll = usize::MAX;
            let bottom = render(&mut s, width, height);
            assert!(bottom.contains("last_line"));
            assert_eq!(
                &bottom.lines().collect::<Vec<_>>()[header + 1..header + 4],
                information,
                "滚动补丁时上方信息区应保持固定"
            );
            key(&mut s, KeyCode::PageUp);
            assert!(!render(&mut s, width, height).contains("last_line"));
        }
        key(&mut s, KeyCode::Esc);
        key(&mut s, KeyCode::Esc);
        key(&mut s, KeyCode::Esc);
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
        let before = std::fs::read(s.service.store.config_path()).unwrap();
        open_files(&mut s);
        wait(&mut s);
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        maximize_preview(&mut s);
        let text = render(&mut s, 160, 32);
        assert!(text.contains(&format!("├ {} ", i18n::tr!(Index))), "{text}");
        assert!(
            text.contains(&format!("┬ {} ", i18n::tr!(Worktree))),
            "{text}"
        );
        assert!(text.contains(&format!(
            "{}: {}",
            i18n::tr!(DiffAuto),
            i18n::tr!(DiffSplit)
        )));
        s.paste("//repo diff layout ");
        let menu = render(&mut s, 160, 32);
        for name in ["auto", "split", "unified"] {
            assert!(menu.contains(name));
        }
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Tab);
        assert_eq!(s.bar.text(), "/repo diff layout split");
        support::write(f.repo.join("one.rs"), "fn changed_after_snapshot() {}\n");
        key(&mut s, KeyCode::Enter);
        assert!(matches!(s.mode, Mode::Changes(_)));
        assert_eq!(s.diff_preference, crate::diff::Preference::Split);
        let narrow = render(&mut s, 80, 28);
        assert!(narrow.contains("cached_new"), "{narrow}");
        assert!(!narrow.contains("changed_after_snapshot"));
        assert!(narrow.contains(&format!("├ {} ", i18n::tr!(Index))));
        command(&mut s, "repo diff layout unified");
        assert!(matches!(s.mode, Mode::Changes(_)));
        let wide = render(&mut s, 160, 32);
        assert!(wide.contains(&format!("{} → {}", i18n::tr!(Index), i18n::tr!(Worktree))));
        command(&mut s, "repo diff layout auto");
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
        command(&mut s, "repo diff layout unified");
        key(&mut s, KeyCode::Esc);
        key(&mut s, KeyCode::Esc);
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        maximize_preview(&mut s);
        let second = render(&mut s, 160, 32);
        assert!(second.contains("second"));
        assert_eq!(s.diff_preference, crate::diff::Preference::Unified);
        assert_eq!(
            std::fs::read(s.service.store.config_path()).unwrap(),
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
    command(&mut s, "repo diff layout split");
    assert!(matches!(s.mode, Mode::Browsing));
    open_files(&mut s);
    key(&mut s, KeyCode::Tab);
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    maximize_preview(&mut s);
    let text = render(&mut s, 160, 32);
    assert!(text.contains("├ HEAD "));
    assert!(text.contains(&format!("┬ {} ", i18n::tr!(Index))));
    assert!(text.contains("fn head") && text.contains("fn index"));
    assert!(!text.contains("fn worktree"));
    assert_eq!(
        screen(Service::new(
            s.service.store.clone(),
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
    open_files(&mut s);
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    command(&mut s, "repo diff layout split");
    maximize_preview(&mut s);
    assert!(render(&mut s, 160, 32).contains("visible_again"));
    let narrow = render(&mut s, 12, 18);
    assert!(narrow.contains("needs"), "{narrow}");
    assert_eq!(s.diff_preference, crate::diff::Preference::Split);
    assert!(matches!(s.mode, Mode::Changes(_)));
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
            open_files(&mut s);
            wait(&mut s);
            key(&mut s, KeyCode::Enter);
            wait(&mut s);
            maximize_preview(&mut s);
            assert!(matches!(s.mode, Mode::Changes(_)));
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
