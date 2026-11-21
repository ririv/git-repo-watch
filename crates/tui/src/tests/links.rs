use super::browser::{ready, select};
use super::*;
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use engine::desktop::Target;
use ratatui::layout::Position;
use std::sync::{Arc, Mutex, mpsc};
fn click(s: &mut Screen, point: Position) -> bool {
    s.mouse(
        MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: point.x,
            row: point.y,
            modifiers: KeyModifiers::NONE,
        },
        &Cancel::default(),
    )
    .unwrap()
}
pub(super) fn opened(s: &mut Screen) {
    let start = Instant::now();
    while !s.poll_links() {
        assert!(start.elapsed() < Duration::from_secs(3));
        thread::sleep(Duration::from_millis(5));
    }
}
#[test]
fn rendered_links_open_their_full_targets_without_capturing_lists_or_selection_mode() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        let url = "https://example.com/long/path/with/many/segments?q=one%20two#part";
        f.commit("README.md",&format!("# Links\n\n[访问网址]({url})\n\n[阅读文档](docs/%E4%B8%AD%E6%96%87%20dir/guide.md)\n"));
        std::fs::create_dir_all(f.repo.join("docs/中文 dir")).unwrap();
        f.commit("docs/中文 dir/guide.md", "# Guide\n");
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        let (tx, rx) = mpsc::channel();
        let tx = Mutex::new(tx);
        s.link_opener = crate::links::Opener::with_sink(move |target, _| {
            tx.lock().unwrap().send(target.clone()).unwrap();
            Ok(())
        });
        command(&mut s, "repo files");
        ready(&mut s);
        select(&mut s, "README.md");
        command(&mut s, "repo files mode preview");
        ready(&mut s);
        render(&mut s, 150, 32);
        ready(&mut s);
        render(&mut s, 150, 32);
        let web = s
            .pointer
            .links
            .cells
            .iter()
            .find(|(_, target)| target.uri() == url)
            .map(|((y, x), _)| Position::new(*x, *y))
            .unwrap();
        s.config.mouse.precise_click = false;
        click(&mut s, web);
        assert!(rx.try_recv().is_err());
        s.config.mouse.precise_click = true;
        s.config.interaction_mode = engine::state::InteractionMode::TextSelection;
        assert!(!click(&mut s, web));
        assert!(rx.try_recv().is_err());
        s.config.interaction_mode = engine::state::InteractionMode::Application;
        assert!(click(&mut s, web));
        opened(&mut s);
        assert_eq!(rx.recv().unwrap().uri(), url);
        let local = f.repo.join("docs/中文 dir/guide.md");
        let point = s
            .pointer
            .links
            .cells
            .iter()
            .find(|(_, target)| target.label() == local.display().to_string())
            .map(|((y, x), _)| Position::new(*x, *y))
            .unwrap();
        assert!(click(&mut s, point));
        opened(&mut s);
        assert_eq!(rx.recv().unwrap().label(), local.display().to_string());
        key(&mut s, KeyCode::Char('/'));
        render(&mut s, 150, 32);
        assert!(s.pointer.links.cells.is_empty());
        key(&mut s, KeyCode::Esc);
        for (width, height) in [(48, 14), (24, 12), (1, 1), (150, 32)] {
            render(&mut s, width, height);
            ready(&mut s);
            render(&mut s, width, height);
            assert!(
                s.pointer
                    .links
                    .cells
                    .values()
                    .all(|target| target.uri() == url
                        || target.label() == local.display().to_string())
            );
        }
        command(&mut s, "repo files layout split");
        ready(&mut s);
        render(&mut s, 150, 32);
        ready(&mut s);
        render(&mut s, 150, 32);
        assert!(
            s.pointer
                .links
                .cells
                .values()
                .any(|target| target.uri() == url)
        );
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        s.shutdown();
    }
}
#[test]
fn wrapped_repository_paths_keep_the_full_literal_target_and_do_not_steal_row_selection() {
    let temp = tempfile::tempdir().unwrap();
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    render(&mut s, 150, 32);
    assert!(
        s.pointer
            .links
            .cells
            .values()
            .any(|target| target.label() == f.repo.display().to_string())
    );
    let repository_area = s.repository_area.unwrap();
    assert!(
        !s.pointer
            .links
            .cells
            .keys()
            .any(|(y, x)| repository_area.contains(Position::new(*x, *y)))
    );
    let path = temp.path().join("中文 文件夹 with spaces");
    let text = crate::links::Text::new(
        ratatui::text::Text::from(format!("Path: {}", path.display())),
        std::slice::from_ref(&path),
    );
    let mut map = crate::links::Map::default();
    map.reset(true);
    let mut terminal = Terminal::new(TestBackend::new(18, 12)).unwrap();
    terminal
        .draw(|frame| map.paragraph(frame, frame.area(), &text, 0))
        .unwrap();
    assert!(map.cells.keys().map(|(y, _)| *y).max().unwrap() > 0);
    assert!(
        map.cells
            .values()
            .all(|target| target.label() == path.display().to_string())
    );
    s.shutdown();
}
#[test]
fn markdown_tables_repeated_labels_and_native_osc_targets_survive_reflow_and_clearing() {
    let source = "| Name | Target |\n|---|---|\n| one | [同名](https://first.example/path) |\n| two | [同名](https://second.example/path) |\n";
    for width in [120, 36, 16] {
        let text = crate::markdown::linked(source, width, None);
        let mut map = crate::links::Map::default();
        map.reset(true);
        let mut terminal = Terminal::new(TestBackend::new(width, 30)).unwrap();
        terminal
            .draw(|frame| map.paragraph(frame, frame.area(), &text, 0))
            .unwrap();
        for uri in ["https://first.example/path", "https://second.example/path"] {
            assert!(map.cells.values().any(|target| target.uri() == uri));
        }
        let original = terminal.backend().buffer().clone();
        let mut native = crate::links::Native::default();
        let mut bytes = Vec::new();
        native.flush(&mut bytes, &original, &map).unwrap();
        let wire = String::from_utf8(bytes).unwrap();
        assert!(wire.contains("\x1b]8;id=link-"));
        assert!(wire.contains("https://first.example/path"));
        assert_eq!(&original, terminal.backend().buffer());
        assert!(
            original
                .content
                .iter()
                .all(|cell| !cell.symbol().contains('\x1b'))
        );
        map.reset(false);
        let mut cleared = Vec::new();
        native.flush(&mut cleared, &original, &map).unwrap();
        assert!(!String::from_utf8(cleared).unwrap().contains("\x1b]8;id="));
    }
}
#[test]
fn a_slow_opener_is_independent_and_duplicate_clicks_do_not_launch_twice() {
    let (started, rx) = mpsc::channel();
    let started = Mutex::new(started);
    let released = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let release = released.clone();
    let mut opener = crate::links::Opener::with_sink(move |_, cancel| {
        started.lock().unwrap().send(()).unwrap();
        while !release.load(std::sync::atomic::Ordering::SeqCst) {
            cancel.check()?;
            thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    });
    let target = Arc::new(Target::reference("https://example.com", None).unwrap());
    opener.submit(target.clone()).unwrap();
    rx.recv_timeout(Duration::from_secs(2)).unwrap();
    opener.submit(target).unwrap();
    assert!(rx.try_recv().is_err());
    assert!(opener.poll().is_none());
    released.store(true, std::sync::atomic::Ordering::SeqCst);
    let start = Instant::now();
    loop {
        if let Some((_, result)) = opener.poll() {
            result.unwrap();
            break;
        }
        assert!(start.elapsed() < Duration::from_secs(2));
        thread::sleep(Duration::from_millis(5));
    }
}
