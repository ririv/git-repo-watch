use super::*;
use crate::appearance::Background;
use ratatui::{buffer::Buffer, style::Color};

fn draw(s: &mut Screen, width: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, 40)).unwrap();
    terminal.draw(|frame| view::render(frame, s)).unwrap();
    terminal.backend().buffer().clone()
}
fn surfaces(buffer: &Buffer, background: Background, removed: bool) {
    let (added, deleted) = match background {
        Background::Light => (Color::Rgb(230, 244, 232), Color::Rgb(251, 231, 233)),
        _ => (crate::diff::ADDED, crate::diff::REMOVED),
    };
    assert!(buffer.content.iter().any(|cell| cell.bg == added));
    if removed {
        assert!(buffer.content.iter().any(|cell| cell.bg == deleted));
    }
    if background == Background::Light {
        assert!(
            !buffer
                .content
                .iter()
                .any(|cell| matches!(cell.bg, crate::diff::ADDED | crate::diff::REMOVED))
        );
    }
}

#[test]
fn worktree_diff_adapts_source_and_surfaces_across_layouts_and_theme_changes() {
    let f = support::Fixture::create();
    f.commit(
        "light.ts",
        "export const message = \"before\";\n// stable\n",
    );
    support::write(
        f.repo.join("light.ts"),
        "export const message = \"after 中文\";\n// stable\n",
    );
    support::write(
        f.repo.join("notes.txt"),
        "中文 ordinary source with a link https://example.com/\n",
    );
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    let head = support::git(&f.repo, &["rev-parse", "HEAD"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    s.background = Background::Light;
    command(&mut s, "repo diff");
    wait(&mut s);
    maximize_preview(&mut s);
    for background in [Background::Light, Background::Dark, Background::Light] {
        s.background = background;
        s.poll_changes();
        wait(&mut s);
        for layout in [
            crate::diff::Preference::Unified,
            crate::diff::Preference::Split,
        ] {
            for wrap in [false, true] {
                s.diff_preference = layout;
                s.config.text_wrap = wrap;
                for context in [crate::diff::Context::Changes, crate::diff::Context::Full] {
                    s.diff_context = context;
                    let buffer = draw(&mut s, 180);
                    surfaces(&buffer, background, true);
                    let colors = buffer
                        .content
                        .iter()
                        .filter(|cell| {
                            cell.bg
                                == if background == Background::Light {
                                    Color::Rgb(230, 244, 232)
                                } else {
                                    crate::diff::ADDED
                                }
                        })
                        .map(|cell| cell.fg)
                        .collect::<std::collections::HashSet<_>>();
                    assert!(colors.len() >= 3, "{background:?} {layout:?}");
                }
            }
        }
    }
    let Mode::Changes(session) = &mut s.mode else {
        panic!("changes")
    };
    session.files.cursor = crate::files::Cursor::File(
        session
            .files
            .rows()
            .iter()
            .position(|change| change.location.name() == std::path::Path::new("notes.txt"))
            .unwrap(),
    );
    s.poll_changes();
    wait(&mut s);
    let buffer = draw(&mut s, 104);
    surfaces(&buffer, Background::Light, false);
    assert!(
        buffer
            .content
            .iter()
            .any(|cell| cell.bg == Color::Rgb(230, 244, 232)
                && matches!(cell.fg, Color::Rgb(r, g, b) if r <= 80 && g <= 80 && b <= 80))
    );
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        std::fs::read_to_string(f.repo.join("light.ts")).unwrap(),
        "export const message = \"after 中文\";\n// stable\n"
    );
    s.shutdown();
}

#[test]
fn historical_continuous_and_full_file_diffs_use_the_light_theme() {
    let f = support::Fixture::create();
    f.commit("history.ts", "export const message = \"committed 中文\";\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    s.background = Background::Light;
    command(&mut s, "repo log");
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    let start = Instant::now();
    loop {
        draw(&mut s, 180);
        s.poll_inspection();
        if s.mode.inspection().unwrap().review.loaded() {
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(8));
        thread::sleep(Duration::from_millis(5));
    }
    surfaces(&draw(&mut s, 180), Background::Light, false);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    surfaces(&draw(&mut s, 180), Background::Light, false);
    s.shutdown();
}
