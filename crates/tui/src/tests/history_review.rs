use super::*;
use crate::{
    history::{Focus, View},
    inspection::{Session as Commit, review::Rows},
};
use crossterm::event::MouseEventKind::{ScrollDown, ScrollUp};

fn detail(s: &Screen) -> &Commit {
    let Mode::History(view) = &s.mode else {
        panic!("history view")
    };
    let View::Commit(commit) = view.as_ref() else {
        panic!("commit view")
    };
    &commit.inspect
}
fn detail_mut(s: &mut Screen) -> &mut Commit {
    let Mode::History(view) = &mut s.mode else {
        panic!("history view")
    };
    let View::Commit(commit) = view.as_mut() else {
        panic!("commit view")
    };
    &mut commit.inspect
}
fn open(f: &support::Fixture) -> (tempfile::TempDir, Screen) {
    let (home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    (home, s)
}
fn loaded(s: &mut Screen) {
    let start = Instant::now();
    loop {
        render(s, 160, 40);
        s.poll_inspection();
        if detail(s).review.loaded() {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(8),
            "historical previews did not complete"
        );
        thread::sleep(Duration::from_millis(5));
    }
    render(s, 160, 40);
}
fn files(count: usize, message: &str) -> support::Fixture {
    let f = support::Fixture::create();
    for file in 0..count {
        let content = (0..40)
            .map(|line| format!("export const unique_file_{file:02}_{line:02} = \"historical\";\n"))
            .collect::<String>();
        support::write(f.repo.join(format!("file-{file:02}.ts")), &content);
    }
    support::git(&f.repo, &["add", "."]);
    support::git(&f.repo, &["commit", "-m", message]);
    f
}

#[test]
fn all_files_stream_into_one_review_and_selection_scrolls_without_changing_git_state() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = files(20, "Review subject\n\nReview body");
        support::write(f.repo.join("file-01.ts"), "current-index\n");
        support::git(&f.repo, &["add", "file-01.ts"]);
        support::write(f.repo.join("file-01.ts"), "current-worktree\n");
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        let (_home, mut s) = open(&f);
        loaded(&mut s);
        let commit = detail(&s);
        assert_eq!(commit.review.layout.sections.len(), 20);
        assert!(
            commit
                .review
                .layout
                .sections
                .iter()
                .all(|section| matches!(section.rows, Rows::Ready(_)))
        );
        let areas = view::inspection::areas(s.body_area.unwrap(), commit);
        let message = areas
            .iter()
            .find(|(_, focus)| *focus == Focus::Message)
            .unwrap()
            .0;
        let files = areas
            .iter()
            .find(|(_, focus)| *focus == Focus::Files)
            .unwrap()
            .0;
        let preview = areas
            .iter()
            .find(|(_, focus)| *focus == Focus::Preview)
            .unwrap()
            .0;
        assert!(message.bottom() <= files.y && files.x < preview.x);
        let text = render(&mut s, 160, 40);
        assert!(text.contains("Review subject") && text.contains("Review body"));
        key(&mut s, KeyCode::Down);
        let text = render(&mut s, 160, 40);
        assert!(text.contains("unique_file_01_00"), "{text}");
        assert_eq!(
            detail(&s).review.layout.offset,
            detail(&s).review.layout.sections[1].start
        );
        assert!(!text.contains("current-index") && !text.contains("current-worktree"));
        key(&mut s, KeyCode::Up);
        render(&mut s, 160, 40);
        assert_eq!(detail(&s).review.layout.offset, 0);
        key(&mut s, KeyCode::Down);
        s.paste("file-01");
        let text = render(&mut s, 160, 40);
        assert_eq!(detail(&s).review.layout.sections.len(), 1);
        assert!(text.contains("unique_file_01_00") && !text.contains("unique_file_00_00"));
        let offset = detail(&s).review.layout.offset;
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert!(
            matches!(&s.mode, Mode::History(view) if matches!(view.as_ref(), View::Diff { .. }))
        );
        let text = render(&mut s, 160, 40);
        assert!(text.contains("historical") && !text.contains("current-worktree"));
        key(&mut s, KeyCode::Esc);
        assert_eq!(detail(&s).query, "file-01");
        assert_eq!(detail(&s).review.layout.offset, offset);
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        assert_eq!(
            std::fs::read_to_string(f.repo.join("file-01.ts")).unwrap(),
            "current-worktree\n"
        );
        s.shutdown();
    }
}

#[test]
fn narrow_views_skip_preview_loading_and_resize_preserves_the_file_selection() {
    let f = files(3, "Compact commit");
    let (_home, mut s) = open(&f);
    render(&mut s, 60, 18);
    let areas = view::inspection::areas(s.body_area.unwrap(), detail(&s));
    assert_eq!(areas.len(), 2);
    assert!(areas.iter().all(|(_, focus)| *focus != Focus::Preview));
    assert!(!s.poll_inspection());
    key(&mut s, KeyCode::Down);
    loaded(&mut s);
    assert_eq!(detail(&s).files.selected(), Some(1));
    for (width, height) in [(60, 18), (24, 8), (8, 4), (1, 1), (0, 0), (160, 40)] {
        render(&mut s, width, height);
    }
    assert_eq!(detail(&s).files.selected(), Some(1));
    assert!(render(&mut s, 160, 40).contains("unique_file_01_00"));
    s.shutdown();
}

#[test]
fn header_preference_applies_to_continuous_and_full_file_previews() {
    let f = files(3, "Header preference");
    let (_home, mut s) = open(&f);
    loaded(&mut s);
    assert!(!render(&mut s, 160, 40).contains("diff --git"));
    s.paste("file-00");
    assert_eq!(detail(&s).review.layout.sections.len(), 3);
    command(&mut s, "repo diff header show");
    let expanded = render(&mut s, 160, 40);
    assert!(expanded.contains("diff --git a/file-00.ts"), "{expanded}");
    assert_eq!(detail(&s).review.layout.sections.len(), 1);
    key(&mut s, KeyCode::Esc);
    render(&mut s, 160, 40);
    assert_eq!(detail(&s).review.layout.sections.len(), 3);
    key(&mut s, KeyCode::Down);
    let expanded = render(&mut s, 160, 40);
    assert!(expanded.contains("diff --git a/file-01.ts"), "{expanded}");
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    let expanded = render(&mut s, 160, 40);
    assert!(expanded.contains("diff --git a/file-01.ts"), "{expanded}");
    command(&mut s, "repo diff header hide");
    assert!(!render(&mut s, 160, 40).contains("diff --git"));
    key(&mut s, KeyCode::Esc);
    let collapsed = render(&mut s, 160, 40);
    assert!(!collapsed.contains("diff --git"));
    assert!(collapsed.contains("unique_file_01_00"), "{collapsed}");
    s.shutdown();
}

#[test]
fn wheel_focus_zoom_and_full_file_navigation_are_scoped_to_their_panels() {
    let f = files(
        3,
        &format!("Scrollable message\n\n{}", "Message body\n".repeat(30)),
    );
    let (_home, mut s) = open(&f);
    loaded(&mut s);
    let areas = view::inspection::areas(s.body_area.unwrap(), detail(&s));
    let preview = areas
        .iter()
        .find(|(_, focus)| *focus == Focus::Preview)
        .unwrap()
        .0;
    let message = areas
        .iter()
        .find(|(_, focus)| *focus == Focus::Message)
        .unwrap()
        .0;
    s.scroll_input = Default::default();
    assert!(wheel(&mut s, ScrollDown, preview.x + 2, preview.y + 2));
    assert_eq!(detail(&s).focus, Focus::Preview);
    assert_eq!(detail(&s).review.layout.offset, 3);
    assert_eq!(detail(&s).files.selected(), Some(0));
    assert_eq!(s.search_support(), crate::search::Support::Unavailable);
    s.paste("hidden query");
    assert!(detail(&s).query.is_empty());
    render(&mut s, 160, 40);
    s.scroll_input = Default::default();
    assert!(wheel(&mut s, ScrollDown, message.x + 2, message.y + 2));
    assert_eq!(detail(&s).scroll, 3);
    assert_eq!(detail(&s).focus, Focus::Message);
    command(&mut s, "focus");
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Right);
    key(&mut s, KeyCode::Enter);
    assert_eq!(detail(&s).focus, Focus::Preview);
    command(&mut s, "zoom");
    render(&mut s, 160, 40);
    assert_eq!(
        view::inspection::areas(s.body_area.unwrap(), detail(&s)).len(),
        1
    );
    key(&mut s, KeyCode::Esc);
    assert_eq!(detail(&s).arrangement, crate::workspace::Arrangement::Tiled);
    render(&mut s, 160, 40);
    s.config.mouse.select_pane = false;
    detail_mut(&mut s).focus = Focus::Files;
    s.sync_bar();
    s.scroll_input = Default::default();
    assert!(!wheel(&mut s, ScrollUp, preview.x + 2, preview.y + 2));
    assert_eq!(detail(&s).focus, Focus::Files);
    s.shutdown();
}

#[test]
fn changing_merge_parent_replaces_every_preview_with_the_selected_comparison() {
    let f = support::Fixture::create();
    f.commit("base.ts", "export const base = 1;\n");
    support::git(&f.repo, &["checkout", "-b", "feature"]);
    f.commit("feature.ts", "export const unique_feature = 1;\n");
    support::git(&f.repo, &["checkout", "main"]);
    f.commit("main.ts", "export const unique_main = 1;\n");
    support::git(
        &f.repo,
        &["merge", "--no-ff", "-m", "Merge commit", "feature"],
    );
    let (_home, mut s) = open(&f);
    loaded(&mut s);
    assert!(render(&mut s, 160, 40).contains("unique_feature"));
    key(&mut s, KeyCode::Tab);
    wait(&mut s);
    loaded(&mut s);
    let crate::inspection::Source::Commit(source) = &detail(&s).detail else {
        panic!("commit")
    };
    assert_eq!(
        source.base,
        engine::local::history::Base::Commit(source.parents[1])
    );
    let text = render(&mut s, 160, 40);
    assert!(
        text.contains("unique_main") && !text.contains("unique_feature"),
        "{text}"
    );
    s.shutdown();
}

#[test]
fn continuous_review_keeps_permission_rename_and_binary_metadata() {
    use std::os::unix::fs::PermissionsExt;
    let _language = i18n::using(i18n::Language::English);
    let f = files(1, "Original file");
    support::git(&f.repo, &["mv", "file-00.ts", "renamed.ts"]);
    std::fs::set_permissions(
        f.repo.join("renamed.ts"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    std::fs::write(f.repo.join("01-中文 二进制.dat"), [0, 1, 2, 0, 4]).unwrap();
    support::git(&f.repo, &["add", "."]);
    support::git(&f.repo, &["commit", "-m", "Rename and executable mode"]);
    let (_home, mut s) = open(&f);
    loaded(&mut s);
    let text = render(&mut s, 160, 40);
    assert!(
        text.contains("File mode") && text.contains("100644") && text.contains("100755"),
        "{text}"
    );
    assert!(
        text.contains("file-00.ts")
            && text.contains("renamed.ts")
            && text.contains("Renamed")
            && text.contains("Similarity"),
        "{text}"
    );
    assert!(!text.contains("old mode") && !text.contains("diff --git"));
    assert!(
        text.contains("Binary files") || text.contains("Binary content"),
        "{text}"
    );
    assert!(text.contains("b/01-中文 二进制.dat"), "{text}");
    s.shutdown();
}
