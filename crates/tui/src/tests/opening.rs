use super::browser::{ready, select, session, text};
use super::*;
use crate::browser::{
    Focus, Preview,
    opening::{Layout, Presentation, TextMode},
    preview::Document,
};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Position;

#[test]
fn per_file_commands_preserve_independent_source_and_rendered_positions() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        let markdown = format!(
            "# 文档 Title\n\n{}\n```rust\nlet value = 42;\n```\n",
            format!(
                "{}\n\n",
                "A long paragraph with 中文 and styled **words**. ".repeat(4)
            )
            .repeat(40)
        );
        f.commit("README.md", &markdown);
        f.commit("plain.txt", "plain file\n");
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        command(&mut s, "repo files");
        ready(&mut s);
        select(&mut s, "plain.txt");
        assert!(session(&mut s).open_target().is_none());
        key(&mut s, KeyCode::Char('/'));
        assert!(!s.search_commands().unwrap().iter().any(|command| matches!(
            command.action,
            Action::BrowserLayoutMenu
                | Action::BrowserLayout(_)
                | Action::BrowserModeMenu
                | Action::BrowserMode(crate::browser::opening::TextMode::Source)
                | Action::BrowserMode(crate::browser::opening::TextMode::Preview)
        )));
        key(&mut s, KeyCode::Esc);
        select(&mut s, "README.md");
        assert!(text(&mut s).contains("# 文档 Title"));
        assert_eq!(
            session(&mut s).open_target().unwrap().presentation.active(),
            TextMode::Source
        );
        render(&mut s, 150, 32);
        session(&mut s).pane().position.scroll = 7;
        session(&mut s).pane().position.horizontal.constrain(40);
        session(&mut s).pane().position.horizontal.seek(4);
        command(&mut s, "repo files mode preview");
        ready(&mut s);
        assert_eq!(
            session(&mut s).open_target().unwrap().presentation.active(),
            TextMode::Preview
        );
        assert!(text(&mut s).contains("文档 Title"));
        assert!(!text(&mut s).contains("# 文档 Title"));
        assert!(matches!(
            session(&mut s).pane().preview.document(),
            Some(Document::Text { gutter: 0, .. })
        ));
        session(&mut s).pane().position.scroll = 3;
        command(&mut s, "repo files mode source");
        ready(&mut s);
        assert_eq!(session(&mut s).pane().position.scroll, 7);
        assert_eq!(session(&mut s).pane().position.horizontal.offset(), 4);
        select(&mut s, "plain.txt");
        select(&mut s, "README.md");
        assert_eq!(session(&mut s).pane().position.scroll, 7);
        command(&mut s, "repo files mode preview");
        ready(&mut s);
        assert_eq!(session(&mut s).pane().position.scroll, 3);
        for (width, height) in [(150, 32), (48, 12), (24, 8), (1, 1), (150, 32)] {
            render(&mut s, width, height);
            ready(&mut s);
            let Some(Document::Text { text, .. }) = session(&mut s).pane().preview.document()
            else {
                panic!("Markdown layout")
            };
            // 缩放只重排视觉行，文档仍保留完整的逻辑文本供定位与复制。
            assert!(text.to_string().contains("文档 Title"));
        }
        s.shutdown();
    }
}

#[test]
fn svg_preview_uses_pixels_and_errors_keep_source_available() {
    let f = support::Fixture::create();
    f.commit("drawing.svg", r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="10"><rect width="20" height="10" fill="#e02040"/></svg>"##);
    f.commit("broken.svg", "<svg>broken");
    f.commit("external.svg", r#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="10"><image href="outside.png" width="20" height="10"/></svg>"#);
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo files");
    ready(&mut s);
    select(&mut s, "drawing.svg");
    command(&mut s, "repo files mode preview");
    ready(&mut s);
    let Some(Document::Image(view)) = session(&mut s).pane().preview.document() else {
        panic!("SVG image")
    };
    let crate::preview::Side::Vector { tree, .. } = &view.pairs[0].after else {
        panic!("vector snapshot")
    };
    let raster = crate::preview::svg::raster(tree, 80, 80).unwrap();
    assert_eq!(raster.get_pixel(40, 40).0, [224, 32, 64, 255]);
    assert_eq!(raster.get_pixel(0, 0).0, [0, 0, 0, 0]);
    assert!(render(&mut s, 150, 32).contains('\u{10eeee}'));
    s.graphics =
        crate::graphics::Display::new(crate::graphics::Support::Unavailable("no pixels".into()));
    assert!(render(&mut s, 150, 32).contains("no pixels"));
    command(&mut s, "repo files mode source");
    ready(&mut s);
    assert!(text(&mut s).contains("<svg"));
    for (file, message) in [
        ("broken.svg", i18n::tr!(SvgInvalid)),
        ("external.svg", i18n::tr!(SvgExternal)),
    ] {
        select(&mut s, file);
        command(&mut s, "repo files mode preview");
        ready(&mut s);
        assert!(
            matches!(&session(&mut s).pane().preview, Preview::Failed {error,..} if error.contains(message))
        );
        command(&mut s, "repo files mode source");
        ready(&mut s);
        assert!(text(&mut s).contains("<svg"));
        assert_eq!(session(&mut s).focus, Focus::Preview);
    }
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    s.shutdown();
}

#[test]
fn split_keeps_two_documents_and_independent_viewports_across_focus_refresh_and_file_changes() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        f.commit(
            "README.md",
            &format!(
                "# Split document\n\n{}",
                "paragraph **bold** with 中文\n\n".repeat(80)
            ),
        );
        f.commit("other.md", "# Other document\n");
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        command(&mut s, "repo files");
        ready(&mut s);
        select(&mut s, "README.md");
        command(&mut s, "repo files layout split");
        ready(&mut s);
        let rendered = render(&mut s, 150, 32);
        ready(&mut s);
        assert!(rendered.contains(i18n::tr!(FileSource)));
        assert!(rendered.contains(i18n::tr!(FileMarkdownPreview)));
        assert!(matches!(
            session(&mut s).presentation(),
            Presentation::Split(TextMode::Preview)
        ));
        for (mode, raw) in [(TextMode::Source, true), (TextMode::Preview, false)] {
            let Some(Document::Text { text, .. }) =
                session(&mut s).panes.get(mode).preview.document()
            else {
                panic!("both sides must load")
            };
            assert_eq!(text.to_string().contains("# Split document"), raw);
        }
        session(&mut s)
            .panes
            .get_mut(TextMode::Source)
            .position
            .scroll = 5;
        session(&mut s)
            .panes
            .get_mut(TextMode::Preview)
            .position
            .scroll = 9;
        key(&mut s, KeyCode::Tab);
        assert_eq!(session(&mut s).focus, Focus::Tree);
        key(&mut s, KeyCode::Tab);
        assert_eq!(session(&mut s).presentation().active(), TextMode::Source);
        key(&mut s, KeyCode::Down);
        assert_eq!(
            session(&mut s).panes.get(TextMode::Source).position.scroll,
            6
        );
        assert_eq!(
            session(&mut s).panes.get(TextMode::Preview).position.scroll,
            9
        );
        key(&mut s, KeyCode::Tab);
        key(&mut s, KeyCode::Down);
        assert_eq!(
            session(&mut s).panes.get(TextMode::Preview).position.scroll,
            10
        );
        key(&mut s, KeyCode::BackTab);
        assert_eq!(session(&mut s).presentation().active(), TextMode::Source);
        command(&mut s, "repo files layout");
        assert_eq!(
            super::options::choices(&s).current,
            Some(command::options::Value::FileLayout(Layout::Split))
        );
        let text = render(&mut s, 150, 32);
        let current = text.lines().find(|line| line.contains("│split ")).unwrap();
        assert!(current.ends_with("✓│"), "{text}");
        key(&mut s, KeyCode::Esc);
        key(&mut s, KeyCode::Esc);
        command(&mut s, "repo files layout split");
        ready(&mut s);
        assert_eq!(
            session(&mut s).presentation(),
            Presentation::Split(TextMode::Source)
        );
        assert_eq!(session(&mut s).pane().position.scroll, 6);
        command(&mut s, "repo files layout single");
        ready(&mut s);
        assert_eq!(
            session(&mut s).presentation(),
            Presentation::Single(TextMode::Source)
        );
        assert_eq!(session(&mut s).pane().position.scroll, 6);
        command(&mut s, "repo files layout single");
        ready(&mut s);
        assert_eq!(
            session(&mut s).presentation(),
            Presentation::Single(TextMode::Source)
        );
        assert_eq!(session(&mut s).pane().position.scroll, 6);
        command(&mut s, "repo files layout split");
        ready(&mut s);
        assert_eq!(session(&mut s).pane().position.scroll, 10);
        select(&mut s, "other.md");
        assert_eq!(
            session(&mut s).presentation(),
            Presentation::Single(TextMode::Source)
        );
        select(&mut s, "README.md");
        assert_eq!(
            session(&mut s).presentation(),
            Presentation::Split(TextMode::Preview)
        );
        assert_eq!(
            session(&mut s).panes.get(TextMode::Source).position.scroll,
            6
        );
        assert_eq!(
            session(&mut s).panes.get(TextMode::Preview).position.scroll,
            10
        );
        std::fs::write(
            f.repo.join("README.md"),
            "# Refreshed document\n\nnew **content**\n",
        )
        .unwrap();
        command(&mut s, "refresh");
        ready(&mut s);
        for mode in [TextMode::Source, TextMode::Preview] {
            let Some(Document::Text { text, .. }) =
                session(&mut s).panes.get(mode).preview.document()
            else {
                panic!("refreshed split")
            };
            assert!(text.to_string().contains("Refreshed document"));
        }
        for (width, height) in [(150, 32), (48, 12), (24, 8), (1, 1), (150, 32)] {
            render(&mut s, width, height);
            ready(&mut s);
            assert!(matches!(
                session(&mut s).presentation(),
                Presentation::Split(_)
            ));
        }
        command(&mut s, "repo files mode source");
        ready(&mut s);
        assert_eq!(
            session(&mut s).presentation(),
            Presentation::Single(TextMode::Source)
        );
        s.shutdown();
    }
}

#[test]
fn split_pointer_focus_wheel_and_scrollbars_target_the_correct_side_and_respect_preferences() {
    let f = support::Fixture::create();
    f.commit(
        "README.md",
        &format!(
            "# Pointer document\n\n{}",
            "long **paragraph** 中文 content\n\n".repeat(100)
        ),
    );
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo files");
    ready(&mut s);
    select(&mut s, "README.md");
    command(&mut s, "repo files layout split");
    ready(&mut s);
    render(&mut s, 150, 32);
    ready(&mut s);
    render(&mut s, 150, 32);
    let body = s.body_area.unwrap();
    let (area, _) = view::browser::areas(body, session(&mut s))
        .into_iter()
        .find(|(_, focus)| *focus == Focus::Preview)
        .unwrap();
    let source = Position::new(area.x + 4, area.y + 5);
    let preview = Position::new(area.right() - 4, area.y + 5);
    let pointer = |s: &mut Screen, point: Position, kind: MouseEventKind| {
        s.mouse(
            MouseEvent {
                kind,
                column: point.x,
                row: point.y,
                modifiers: KeyModifiers::NONE,
            },
            &Cancel::default(),
        )
        .unwrap()
    };
    s.config.mouse.select_pane = false;
    assert!(!pointer(
        &mut s,
        source,
        MouseEventKind::Down(MouseButton::Left)
    ));
    assert_eq!(session(&mut s).presentation().active(), TextMode::Preview);
    s.config.mouse.select_pane = true;
    assert!(pointer(
        &mut s,
        source,
        MouseEventKind::Down(MouseButton::Left)
    ));
    assert_eq!(session(&mut s).presentation().active(), TextMode::Source);
    assert!(pointer(&mut s, preview, MouseEventKind::ScrollDown));
    assert_eq!(session(&mut s).presentation().active(), TextMode::Preview);
    assert_eq!(
        session(&mut s).panes.get(TextMode::Source).position.scroll,
        0
    );
    assert_eq!(
        session(&mut s).panes.get(TextMode::Preview).position.scroll,
        3
    );
    s.config.mouse.scroll = false;
    assert!(!pointer(&mut s, source, MouseEventKind::ScrollDown));
    assert_eq!(session(&mut s).presentation().active(), TextMode::Preview);
    s.config.mouse.scroll = true;
    render(&mut s, 150, 32);
    let target = crate::pointer::ScrollTarget::BrowserPreview(TextMode::Source);
    let track = (area.y + 3..area.bottom() - 1).rev().find_map(|y| {
        (area.x + 1..area.right() - 1).find_map(|x| {
            let point = Position::new(x,y);
            matches!(s.pointer.hit(point), Some(crate::pointer::Hit::Scrollbar(crate::pointer::Action::Scroll(found,offset))) if found == target && offset > 0).then_some(point)
        })
    }).expect("source scrollbar track");
    assert!(pointer(
        &mut s,
        track,
        MouseEventKind::Down(MouseButton::Left)
    ));
    assert!(session(&mut s).panes.get(TextMode::Source).position.scroll > 0);
    assert_eq!(
        session(&mut s).panes.get(TextMode::Preview).position.scroll,
        3
    );
    assert_eq!(session(&mut s).presentation().active(), TextMode::Source);
    s.config.interaction_mode = engine::state::InteractionMode::TextSelection;
    assert!(!pointer(
        &mut s,
        preview,
        MouseEventKind::Down(MouseButton::Left)
    ));
    assert_eq!(session(&mut s).presentation().active(), TextMode::Source);
    s.shutdown();
}
