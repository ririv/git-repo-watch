use super::browser::{ready, select, session};
use super::*;
use crate::browser::{
    Focus,
    opening::{Presentation, TextMode},
    preview::Document,
};

fn open(s: &mut Screen, path: &str) {
    command(s, "repo files");
    ready(s);
    select(s, path);
    key(s, KeyCode::Enter);
    ready(s);
    assert_eq!(session(s).focus, Focus::Preview);
    assert_eq!(s.search_support(), crate::search::Support::Locate);
}
fn query(s: &mut Screen, value: &str) {
    s.key(
        KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
        &Cancel::default(),
    )
    .unwrap();
    s.paste(value);
}
fn frame(s: &mut Screen, width: u16, height: u16) -> ratatui::buffer::Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|frame| view::render(frame, s)).unwrap();
    terminal.backend().buffer().clone()
}

#[test]
fn content_locates_each_occurrence_cycles_and_highlights_without_mutating_source_or_git() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        let text = (0..60)
            .map(|line| match line {
                30 => "export const 中文 = \"Needle needle\";\n".to_owned(),
                55 => "// 中文 👩‍💻 NEEDLE\n".to_owned(),
                _ => "// filler\n".to_owned(),
            })
            .collect::<String>();
        f.commit("find.ts", &text);
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        open(&mut s, "find.ts");
        render(&mut s, 100, 24);
        let document = match &session(&mut s).pane().preview {
            crate::browser::Preview::Ready { document, .. } => document.clone(),
            _ => panic!(),
        };
        s.paste("needle");
        assert!(s.search_title().contains("1/3"));
        let buffer = frame(&mut s, 100, 24);
        let pane = session(&mut s).pane();
        assert!(pane.position.scroll > 0);
        assert_eq!(pane.find.current(&pane.position.find).unwrap().line, 30);
        let current = buffer
            .content
            .iter()
            .position(|cell| {
                cell.symbol() == "N" && cell.bg == ratatui::style::Color::Rgb(124, 83, 24)
            })
            .unwrap();
        let other = current + "Needle ".len();
        assert_eq!(buffer.content[other].symbol(), "n");
        assert_eq!(
            buffer.content[other].bg,
            ratatui::style::Color::Rgb(81, 65, 37)
        );
        assert_eq!(buffer.content[current].fg, buffer.content[other].fg);
        assert!(matches!(
            buffer.content[current].fg,
            ratatui::style::Color::Rgb(..)
        ));
        key(&mut s, KeyCode::Down);
        assert!(s.search_title().contains("2/3"));
        let buffer = frame(&mut s, 100, 24);
        assert_eq!(
            buffer.content[current].bg,
            ratatui::style::Color::Rgb(81, 65, 37)
        );
        assert_eq!(
            buffer.content[other].bg,
            ratatui::style::Color::Rgb(124, 83, 24)
        );
        let pane = session(&mut s).pane();
        assert_eq!(pane.find.current(&pane.position.find).unwrap().line, 30);
        key(&mut s, KeyCode::Down);
        render(&mut s, 100, 24);
        let pane = session(&mut s).pane();
        assert_eq!(pane.find.current(&pane.position.find).unwrap().line, 55);
        assert!(s.search_title().contains("3/3"));
        key(&mut s, KeyCode::Down);
        assert!(s.search_title().contains("1/3"));
        key(&mut s, KeyCode::Up);
        assert!(s.search_title().contains("3/3"));
        query(&mut s, "not found");
        assert!(s.search_title().contains("0/0"));
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Esc);
        assert_eq!(session(&mut s).focus, Focus::Preview);
        assert!(s.bar.text().is_empty());
        let current = match &session(&mut s).pane().preview {
            crate::browser::Preview::Ready { document, .. } => document,
            _ => panic!(),
        };
        assert!(Arc::ptr_eq(&document, current));
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        assert_eq!(
            std::fs::read_to_string(f.repo.join("find.ts")).unwrap(),
            text
        );
        s.shutdown();
    }
}

#[test]
fn long_unicode_lines_reveal_matches_horizontally_and_after_wrapping_and_resize() {
    let f = support::Fixture::create();
    f.commit("long.txt", &format!("{}TARGET\n", "中".repeat(1000)));
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    open(&mut s, "long.txt");
    render(&mut s, 90, 18);
    s.paste("target");
    let text = render(&mut s, 90, 18);
    assert!(text.contains("TARGET"), "{text}");
    assert!(session(&mut s).pane().position.horizontal.offset() > 0);
    s.config.text_wrap = true;
    for (width, height) in [(90, 18), (24, 14), (1, 1), (90, 18)] {
        let text = render(&mut s, width, height);
        assert_eq!(session(&mut s).pane().position.horizontal.offset(), 0);
        if width > 1 {
            assert!(text.contains("TARGET"), "{text}");
        }
        assert!(s.search_title().contains("1/1"));
    }
    assert!(session(&mut s).pane().position.scroll > 0);
    s.shutdown();
}

#[test]
fn file_filter_source_and_rendered_content_keep_independent_queries_in_split_mode() {
    let f = support::Fixture::create();
    f.commit(
        "README.md",
        &format!(
            "# Heading\n\n{} **needle phrase** tail\n",
            "words ".repeat(90)
        ),
    );
    f.commit("other.txt", "unrelated\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo files");
    ready(&mut s);
    s.paste("README.md");
    ready(&mut s);
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.search_support(), crate::search::Support::Locate);
    assert!(s.bar.text().is_empty());
    s.paste("**needle phrase**");
    assert!(s.search_title().contains("1/1"));
    command(&mut s, "repo files layout split");
    ready(&mut s);
    render(&mut s, 100, 24);
    ready(&mut s);
    assert_eq!(
        session(&mut s).presentation(),
        Presentation::Split(TextMode::Preview)
    );
    assert!(s.bar.text().is_empty());
    s.paste("needle phrase");
    render(&mut s, 100, 24);
    ready(&mut s);
    assert!(s.search_title().contains("1/1"));
    s.cycle_pane(true);
    assert_eq!(s.bar.text(), "**needle phrase**");
    s.cycle_pane(false);
    assert_eq!(s.bar.text(), "needle phrase");
    for (width, height) in [(150, 32), (58, 20), (24, 14), (150, 32)] {
        render(&mut s, width, height);
        ready(&mut s);
        render(&mut s, width, height);
        assert_eq!(s.bar.text(), "needle phrase");
        assert!(s.search_title().contains("1/1"));
    }
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Esc);
    assert_eq!(session(&mut s).focus, Focus::Tree);
    assert_eq!(s.bar.text(), "README.md");
    s.shutdown();
}

#[test]
fn gutters_and_binary_notices_do_not_become_searchable_file_contents() {
    let f = support::Fixture::create();
    f.commit("plain.txt", &"word\n".repeat(30));
    std::fs::write(f.repo.join("binary.dat"), [0, 1, 2, 0]).unwrap();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    open(&mut s, "plain.txt");
    s.paste("1");
    assert!(s.search_title().contains("0/0"));
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Esc);
    select(&mut s, "binary.dat");
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.search_support(), crate::search::Support::Unavailable);
    assert!(matches!(
        session(&mut s).pane().preview.document(),
        Some(Document::Notice(_))
    ));
    s.paste("word");
    assert!(s.bar.text().is_empty());
    s.shutdown();
}
