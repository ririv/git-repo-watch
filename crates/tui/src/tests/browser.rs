use super::*;
use crate::browser::{Focus, Preview, Session, preview::Document};

pub(super) fn session(s: &mut Screen) -> &mut Session {
    let Mode::Browser(session) = &mut s.mode else {
        panic!("file browser")
    };
    session
}
pub(super) fn ready(s: &mut Screen) {
    let start = Instant::now();
    loop {
        s.poll(&Cancel::default());
        if !session(s).pending() {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(8),
            "browser reader did not finish"
        );
        thread::sleep(Duration::from_millis(5));
    }
}
pub(super) fn select(s: &mut Screen, path: &str) {
    let browser = session(s);
    let index = browser
        .rows()
        .iter()
        .position(|row| row.entry.path == std::path::Path::new(path))
        .unwrap();
    browser.select(index);
    ready(s);
}
pub(super) fn text(s: &mut Screen) -> String {
    match &session(s).pane().preview {
        Preview::Ready {
            document: Document::Text { text, .. },
            ..
        } => text.to_string(),
        other => panic!("text preview: {other:?}"),
    }
}

#[test]
fn browser_lazy_tree_search_previews_and_diff_return_keep_selection_and_git_unchanged() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        f.commit("clean.txt", "unchanged\n");
        std::fs::create_dir_all(f.repo.join("src/中文 空格")).unwrap();
        f.commit("src/中文 空格/code.ts", "export const value = 1;\n");
        write(
            f.repo.join("src/中文 空格/code.ts"),
            "export const value = 2; // changed\n",
        );
        write(f.repo.join(".gitignore"), "ignored/\n");
        write(f.repo.join("ignored/hidden.ts"), "ignored\n");
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        command(&mut s, "repo files");
        assert_eq!(s.page, Page::View(crate::navigation::View::Files));
        ready(&mut s);
        assert!(
            !session(&mut s)
                .rows()
                .iter()
                .any(|row| row.entry.path == std::path::Path::new("src/中文 空格/code.ts"))
        );
        select(&mut s, "clean.txt");
        assert!(text(&mut s).contains("unchanged"));
        assert!(render(&mut s, 150, 32).contains("src/中文 空格/"));
        select(&mut s, "src/中文 空格");
        key(&mut s, KeyCode::Enter);
        ready(&mut s);
        select(&mut s, "src/中文 空格/code.ts");
        assert!(text(&mut s).contains("export const value = 2"));
        for (width, height) in [(150, 32), (48, 12), (24, 8), (1, 1), (150, 32)] {
            render(&mut s, width, height);
            assert_eq!(
                session(&mut s).selected().unwrap().path,
                std::path::Path::new("src/中文 空格/code.ts")
            );
        }
        let Preview::Ready {
            document: Document::Text {
                text: highlighted, ..
            },
            ..
        } = &session(&mut s).pane().preview
        else {
            panic!("highlight")
        };
        assert!(highlighted.lines.iter().any(|line| {
            line.spans
                .iter()
                .any(|span| matches!(span.style.fg, Some(ratatui::style::Color::Rgb(..))))
        }));
        key(&mut s, KeyCode::Char('/'));
        assert_eq!(
            s.bar.command_input().unwrap().context(),
            Some(command::Context::Files)
        );
        key(&mut s, KeyCode::Esc);
        command(&mut s, "repo files diff");
        wait(&mut s);
        assert!(
            matches!(&s.mode, Mode::Changes(changes) if changes.files.rows()[changes.files.cursor].location.name() == std::path::Path::new("src/中文 空格/code.ts"))
        );
        assert!(render(&mut s, 150, 32).contains("value = 2"));
        key(&mut s, KeyCode::Esc);
        key(&mut s, KeyCode::Esc);
        ready(&mut s);
        assert_eq!(
            session(&mut s).selected().unwrap().path,
            std::path::Path::new("src/中文 空格/code.ts")
        );
        write(
            f.repo.join("src/中文 空格/code.ts"),
            "export const refreshed = 3;\n",
        );
        command(&mut s, "repo files refresh");
        ready(&mut s);
        assert!(text(&mut s).contains("refreshed = 3"));
        s.paste("code.ts");
        ready(&mut s);
        assert_eq!(session(&mut s).rows().len(), 1);
        assert_eq!(
            session(&mut s).selected().unwrap().path,
            std::path::Path::new("src/中文 空格/code.ts")
        );
        key(&mut s, KeyCode::Esc);
        ready(&mut s);
        assert_eq!(s.page, Page::View(crate::navigation::View::Files));
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        assert!(s.service.store.report().unwrap().is_none());
        s.shutdown();
    }
}

#[test]
fn search_reaches_unopened_directories_and_pointer_selection_uses_the_rendered_tree() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let f = support::Fixture::create();
    write(
        f.repo.join("deep/not/opened/target.txt"),
        "target contents\n",
    );
    write(f.repo.join("a.txt"), "first\n");
    write(f.repo.join("b.txt"), "second\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo files");
    ready(&mut s);
    s.paste("target.txt");
    ready(&mut s);
    assert!(text(&mut s).contains("target contents"));
    key(&mut s, KeyCode::Esc);
    ready(&mut s);
    let screen = render(&mut s, 140, 32);
    let y = screen
        .lines()
        .position(|line| line.contains("b.txt"))
        .unwrap() as u16;
    s.mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 5,
            row: y,
            modifiers: KeyModifiers::NONE,
        },
        &Cancel::default(),
    )
    .unwrap();
    ready(&mut s);
    assert_eq!(
        session(&mut s).selected().unwrap().path,
        std::path::Path::new("b.txt")
    );
    assert!(text(&mut s).contains("second"));
    let body = s.body_area.unwrap();
    let preview = view::browser::areas(body, session(&mut s))
        .into_iter()
        .find(|(_, focus)| *focus == Focus::Preview)
        .unwrap()
        .0;
    assert!(
        s.mouse(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: preview.right() - 3,
                row: preview.y + 4,
                modifiers: KeyModifiers::NONE
            },
            &Cancel::default()
        )
        .unwrap()
    );
    assert_eq!(session(&mut s).focus, Focus::Preview);
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Tab);
    assert_eq!(session(&mut s).focus, Focus::Preview);
    key(&mut s, KeyCode::Esc);
    assert_eq!(session(&mut s).focus, Focus::Tree);
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.page, Page::Repositories);
    s.shutdown();
}

#[test]
fn directory_clicks_toggle_once_per_pair_and_files_keep_double_click_activation() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    use ratatui::layout::Position;

    fn click(s: &mut Screen, path: &str) {
        let index = session(s)
            .rows()
            .iter()
            .position(|row| row.entry.path == std::path::Path::new(path))
            .unwrap();
        render(s, 120, 32);
        let point = (0..32)
            .find_map(|y| {
                (0..120).find_map(|x| {
                    let point = Position::new(x, y);
                    (s.pointer.hit(point)
                        == Some(crate::pointer::Hit::Item(
                            crate::pointer::Target::Browser,
                            index,
                        )))
                    .then_some(point)
                })
            })
            .unwrap();
        for kind in [
            MouseEventKind::Down(MouseButton::Left),
            MouseEventKind::Up(MouseButton::Left),
        ] {
            s.mouse(
                MouseEvent {
                    kind,
                    column: point.x,
                    row: point.y,
                    modifiers: KeyModifiers::NONE,
                },
                &Cancel::default(),
            )
            .unwrap();
        }
    }

    let f = support::Fixture::create();
    std::fs::create_dir_all(f.repo.join("src/中文 目录")).unwrap();
    f.commit("src/中文 目录/code.ts", "export const value = 1;\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo files");
    ready(&mut s);
    let directory = std::path::Path::new("src");
    click(&mut s, "src/中文 目录");
    assert!(session(&mut s).expanded(directory));
    click(&mut s, "src/中文 目录");
    ready(&mut s);
    assert!(session(&mut s).expanded(directory));
    assert_eq!(session(&mut s).focus, Focus::Tree);
    click(&mut s, "src/中文 目录");
    assert!(!session(&mut s).expanded(directory));
    click(&mut s, "src/中文 目录");
    assert!(!session(&mut s).expanded(directory));
    s.config.mouse.precise_click = false;
    click(&mut s, "src/中文 目录");
    assert!(!session(&mut s).expanded(directory));
    s.config.mouse.precise_click = true;
    click(&mut s, "src/中文 目录");
    ready(&mut s);
    assert!(session(&mut s).expanded(directory));
    click(&mut s, "src/中文 目录/code.ts");
    assert_eq!(session(&mut s).focus, Focus::Tree);
    click(&mut s, "src/中文 目录/code.ts");
    assert_eq!(session(&mut s).focus, Focus::Preview);
    s.shutdown();
}

#[test]
fn compact_chains_stop_at_files_and_forks_and_refresh_preserves_navigation() {
    use std::path::Path;
    let f = support::Fixture::create();
    write(f.repo.join(".gitignore"), "*.skip\n");
    write(f.repo.join("output/ignored.skip"), "ignored\n");
    write(
        f.repo.join("output/中文 目录/a.ts"),
        "export const a = 1;\n",
    );
    write(
        f.repo.join("output/中文 目录/b.ts"),
        "export const b = 2;\n",
    );
    write(f.repo.join("fork/left/a.txt"), "left\n");
    write(f.repo.join("fork/right/b.txt"), "right\n");
    write(f.repo.join("mixed/info.txt"), "info\n");
    write(f.repo.join("mixed/nested/deep/c.txt"), "nested\n");
    std::fs::create_dir_all(f.repo.join("empty/one/two")).unwrap();
    std::fs::create_dir(f.repo.join("links")).unwrap();
    std::os::unix::fs::symlink(f.repo.join("output"), f.repo.join("links/alias")).unwrap();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo files");
    ready(&mut s);
    let paths = session(&mut s)
        .rows()
        .iter()
        .map(|row| row.entry.path.clone())
        .collect::<Vec<_>>();
    for path in [
        "output/中文 目录",
        "fork",
        "mixed",
        "empty/one/two",
        "links",
    ] {
        assert!(paths.contains(&PathBuf::from(path)), "{paths:?}");
    }
    let screen = render(&mut s, 150, 32);
    assert!(screen.contains("▶  output/中文 目录/"));
    assert!(!screen.contains("ignored.skip"));
    select(&mut s, "mixed");
    key(&mut s, KeyCode::Enter);
    ready(&mut s);
    assert!(render(&mut s, 150, 32).contains("nested/deep/"));
    select(&mut s, "mixed/nested/deep");
    key(&mut s, KeyCode::Left);
    assert_eq!(session(&mut s).selected().unwrap().path, Path::new("mixed"));
    select(&mut s, "output/中文 目录");
    key(&mut s, KeyCode::Enter);
    ready(&mut s);
    select(&mut s, "output/中文 目录/a.ts");
    write(f.repo.join("output/info.txt"), "new fork\n");
    command(&mut s, "repo files refresh");
    ready(&mut s);
    assert_eq!(
        session(&mut s).selected().unwrap().path,
        Path::new("output/中文 目录/a.ts")
    );
    assert!(
        session(&mut s)
            .rows()
            .iter()
            .any(|row| row.entry.path == Path::new("output/info.txt"))
    );
    select(&mut s, "output/中文 目录");
    std::fs::remove_file(f.repo.join("output/info.txt")).unwrap();
    command(&mut s, "repo files refresh");
    ready(&mut s);
    assert_eq!(
        session(&mut s).selected().unwrap().path,
        Path::new("output/中文 目录")
    );
    key(&mut s, KeyCode::Left);
    assert!(
        !session(&mut s)
            .rows()
            .iter()
            .any(|row| row.entry.path == Path::new("output/中文 目录/a.ts"))
    );
    for (width, height) in [(150, 32), (48, 12), (24, 8), (1, 1), (150, 32)] {
        render(&mut s, width, height);
        assert_eq!(
            session(&mut s).selected().unwrap().path,
            Path::new("output/中文 目录")
        );
    }
    key(&mut s, KeyCode::Left);
    assert_eq!(s.page, Page::Repositories);
    s.shutdown();
}

fn write(path: impl AsRef<std::path::Path>, text: &str) {
    let path = path.as_ref();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    support::write(path, text);
}

#[test]
fn browser_images_use_the_shared_pixel_view_and_unsupported_terminals_remain_usable() {
    let f = support::Fixture::create();
    image::RgbaImage::from_pixel(8, 4, image::Rgba([30, 90, 160, 255]))
        .save(f.repo.join("image.png"))
        .unwrap();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo files");
    ready(&mut s);
    select(&mut s, "image.png");
    let Preview::Ready {
        document: Document::Image(image),
        ..
    } = &session(&mut s).pane().preview
    else {
        panic!("image")
    };
    assert!(
        matches!(&image.pairs[0].after, crate::preview::Side::Image { pixels, .. } if pixels.dimensions() == (8,4))
    );
    let screen = render(&mut s, 140, 32);
    assert_eq!(screen.matches('┌').count(), 2, "{screen}");
    for (width, height) in [(140, 32), (48, 12), (1, 1), (140, 32)] {
        render(&mut s, width, height);
    }
    s.graphics = crate::graphics::Display::new(crate::graphics::Support::Unavailable(
        i18n::tr!(GraphicsUnsupported).into(),
    ));
    assert!(render(&mut s, 140, 32).contains(i18n::tr!(GraphicsUnsupported)));
    s.shutdown();
}
