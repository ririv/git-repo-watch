use super::browser::{ready, select, session};
use super::*;
use crate::{
    browser::opening::{Presentation, TextMode},
    loading::{FEEDBACK_DELAY, Feedback},
};

fn choose_without_poll(s: &mut Screen, path: &str) {
    let browser = session(s);
    let index = browser
        .rows()
        .iter()
        .position(|row| row.entry.path == std::path::Path::new(path))
        .unwrap();
    browser.select(index);
}
fn since<T>(feedback: &Feedback<T>) -> Instant {
    match feedback {
        Feedback::Quiet { since, .. } => *since,
        _ => panic!("quiet preview transition expected"),
    }
}
fn poll_browser_at(s: &mut Screen, now: Instant) {
    let service = s.service.clone();
    let browser = session(s);
    let root = browser.root.clone();
    browser.panes.poll_at(&service, &root, now);
}

#[test]
fn browser_keeps_the_old_title_and_content_without_loading_flash_and_only_publishes_latest() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        for (name, text) in [
            ("a.txt", "ALPHA_TOKEN https://example.invalid/old\n"),
            ("b.txt", "BETA_TOKEN\n"),
            ("c.txt", "GAMMA_TOKEN\n"),
        ] {
            f.commit(name, text);
        }
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        let config = std::fs::read(s.service.store.config_path()).unwrap();
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        command(&mut s, "repo files");
        ready(&mut s);
        select(&mut s, "a.txt");
        render(&mut s, 150, 32);
        choose_without_poll(&mut s, "b.txt");
        let start = since(&session(&mut s).panes.feedback);
        poll_browser_at(&mut s, start);
        choose_without_poll(&mut s, "c.txt");
        let now = since(&session(&mut s).panes.feedback) + Duration::from_millis(50);
        let retained = render(&mut s, 150, 32);
        assert!(
            retained.contains("┌a.txt") && retained.contains("ALPHA_TOKEN"),
            "{retained}"
        );
        assert!(!retained.contains(i18n::tr!(BrowserReading)));
        assert!(!retained.contains("BETA_TOKEN") && !retained.contains("GAMMA_TOKEN"));
        assert!(s.pointer.links.cells.is_empty());
        let deadline = Instant::now();
        loop {
            poll_browser_at(&mut s, now);
            let text = render(&mut s, 150, 32);
            assert!(!text.contains(i18n::tr!(BrowserReading)), "{text}");
            assert!(!text.contains("BETA_TOKEN"), "{text}");
            if !session(&mut s).panes.pending() {
                assert!(
                    text.contains("┌c.txt") && text.contains("GAMMA_TOKEN"),
                    "{text}"
                );
                assert!(!text.contains("ALPHA_TOKEN"));
                break;
            }
            assert!(deadline.elapsed() < Duration::from_secs(8));
            thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(
            std::fs::read(s.service.store.config_path()).unwrap(),
            config
        );
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        s.shutdown();
    }
}

#[test]
fn browser_shows_loading_only_after_the_deadline_and_errors_are_not_delayed() {
    let f = support::Fixture::create();
    f.commit("a.txt", "ALPHA_TOKEN\n");
    f.commit("b.svg", "invalid SVG document\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo files");
    ready(&mut s);
    select(&mut s, "a.txt");
    render(&mut s, 150, 32);
    choose_without_poll(&mut s, "b.svg");
    let target = session(&mut s).open_target().unwrap();
    session(&mut s).present(&target, Presentation::Single(TextMode::Preview));
    let start = since(&session(&mut s).panes.feedback);
    session(&mut s)
        .panes
        .feedback
        .tick(start + FEEDBACK_DELAY - Duration::from_millis(1));
    assert!(render(&mut s, 150, 32).contains("ALPHA_TOKEN"));
    assert!(session(&mut s).panes.feedback.tick(start + FEEDBACK_DELAY));
    let text = render(&mut s, 150, 32);
    assert!(
        text.contains(i18n::tr!(BrowserReading)) && text.contains("┌b.svg"),
        "{text}"
    );
    assert!(!text.contains("ALPHA_TOKEN"));
    let now = start + FEEDBACK_DELAY;
    let deadline = Instant::now();
    while session(&mut s).panes.pending() {
        poll_browser_at(&mut s, now);
        assert!(deadline.elapsed() < Duration::from_secs(8));
        thread::sleep(Duration::from_millis(2));
    }
    assert!(!session(&mut s).panes.feedback.quiet());
    assert!(matches!(
        session(&mut s).pane().preview,
        crate::browser::Preview::Failed { .. }
    ));
    assert!(!render(&mut s, 150, 32).contains(i18n::tr!(BrowserReading)));
    s.shutdown();
}

#[test]
fn source_and_rendered_panes_switch_as_a_complete_pair() {
    let f = support::Fixture::create();
    f.commit("a.md", "# ALPHA_TOKEN\n\n**old**\n");
    f.commit("b.md", "# BETA_TOKEN\n\n**new**\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo files");
    ready(&mut s);
    select(&mut s, "a.md");
    let target = session(&mut s).open_target().unwrap();
    session(&mut s).present(&target, Presentation::Split(TextMode::Preview));
    ready(&mut s);
    render(&mut s, 180, 36);
    ready(&mut s);
    choose_without_poll(&mut s, "b.md");
    let target = session(&mut s).open_target().unwrap();
    session(&mut s).present(&target, Presentation::Split(TextMode::Preview));
    render(&mut s, 180, 36);
    let now = since(&session(&mut s).panes.feedback) + Duration::from_millis(50);
    let deadline = Instant::now();
    loop {
        let quiet = session(&mut s).panes.feedback.quiet();
        let text = render(&mut s, 180, 36);
        if quiet {
            assert!(
                text.contains("┌a.md") && text.matches("ALPHA_TOKEN").count() == 2,
                "{text}"
            );
            assert!(!text.contains("BETA_TOKEN"));
        } else {
            assert!(
                text.contains("┌b.md") && text.matches("BETA_TOKEN").count() == 2,
                "{text}"
            );
            assert!(!text.contains("ALPHA_TOKEN"));
            break;
        }
        poll_browser_at(&mut s, now);
        assert!(deadline.elapsed() < Duration::from_secs(8));
        thread::sleep(Duration::from_millis(2));
    }
    s.shutdown();
}

#[test]
fn changes_replace_fast_diff_previews_directly_without_exposing_old_content_to_copy() {
    let f = support::Fixture::create();
    f.commit("a.rs", "fn a() {}\n");
    f.commit("b.rs", "fn b() {}\n");
    support::write(f.repo.join("a.rs"), "fn a() {} // ALPHA_TOKEN\n");
    support::write(f.repo.join("b.rs"), "fn b() {} // BETA_TOKEN\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    command(&mut s, "repo diff");
    wait(&mut s);
    let service = s.service.clone();
    let Mode::Changes(changes) = &mut s.mode else {
        panic!()
    };
    changes.files.cursor = changes
        .files
        .rows()
        .iter()
        .position(|change| change.location.name() == std::path::Path::new("b.rs"))
        .map(crate::files::Cursor::File)
        .unwrap();
    let now = Instant::now();
    changes.poll_at(&service, now);
    changes.focus = crate::changes::Focus::Preview;
    if matches!(changes.preview, crate::changes::Preview::Loading(_)) {
        let text = render(&mut s, 150, 32);
        assert!(
            text.contains("ALPHA_TOKEN") && !text.contains("BETA_TOKEN"),
            "{text}"
        );
        assert!(!text.contains(i18n::tr!(ReadFileDiff)));
        let result = s
            .copy_selection()
            .payload(crate::clipboard::Scope::Diff, None);
        assert!(
            result
                .err()
                .unwrap()
                .to_string()
                .contains(i18n::tr!(CopySelectionLoading))
        );
    }
    let deadline = Instant::now();
    loop {
        let Mode::Changes(changes) = &mut s.mode else {
            panic!()
        };
        changes.poll_at(&service, now);
        let pending = changes.pending();
        let text = render(&mut s, 150, 32);
        assert!(!text.contains(i18n::tr!(ReadFileDiff)), "{text}");
        if !pending {
            assert!(
                text.contains("BETA_TOKEN") && !text.contains("ALPHA_TOKEN"),
                "{text}"
            );
            break;
        }
        assert!(deadline.elapsed() < Duration::from_secs(8));
        thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    s.shutdown();
}
