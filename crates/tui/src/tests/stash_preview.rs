use super::*;
use crate::{
    inspection::{Focus, Source},
    stashes::View,
};
use crossterm::event::MouseEventKind::ScrollDown;
use engine::local::stash::preview::Scope;

fn click(s: &mut Screen, x: u16, y: u16) {
    s.mouse(
        crossterm::event::MouseEvent {
            kind: crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        },
        &Cancel::default(),
    )
    .unwrap();
}

fn prepared() -> support::Fixture {
    let f = support::Fixture::create();
    f.commit("tracked.ts", "export const value = 'base';\n");
    support::write(
        f.repo.join("tracked.ts"),
        "export const value = 'staged';\n",
    );
    support::git(&f.repo, &["add", "tracked.ts"]);
    support::write(
        f.repo.join("tracked.ts"),
        "export const value = 'saved-worktree';\n",
    );
    support::write(
        f.repo.join("中文 新文件.md"),
        &"# saved-untracked\n".repeat(60),
    );
    support::git(&f.repo, &["stash", "push", "-u", "-m", "预览演练"]);
    support::write(f.repo.join("tracked.ts"), "current-worktree\n");
    f
}
fn open(s: &mut Screen) {
    command(s, "repo stash");
    wait(s);
    key(s, KeyCode::Enter);
    wait(s);
    assert!(s.mode.inspection().is_some(), "{:?}", s.mode);
}
fn loaded(s: &mut Screen) {
    let started = Instant::now();
    loop {
        render(s, 160, 40);
        s.poll_inspection();
        if s.mode.inspection().unwrap().review.loaded() {
            break;
        }
        assert!(started.elapsed() < Duration::from_secs(8));
        thread::sleep(Duration::from_millis(5));
    }
    render(s, 160, 40);
}
fn scope(s: &Screen) -> Scope {
    let Source::Stash(detail) = &s.mode.inspection().unwrap().detail else {
        panic!("stash")
    };
    detail.scope
}
#[test]
fn stash_files_and_continuous_diffs_share_saved_objects_search_and_return_anchors() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = prepared();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        let refs = support::git(&f.repo, &["show-ref"]);
        open(&mut s);
        loaded(&mut s);
        let text = render(&mut s, 160, 40);
        for expected in [
            "预览演练",
            "tracked.ts",
            "中文 新文件.md",
            "saved-worktree",
            "saved-untracked",
        ] {
            assert!(text.contains(expected), "{expected}: {text}");
        }
        assert!(!text.contains("current-worktree"));
        let inspect = s.mode.inspection().unwrap();
        assert_eq!(inspect.review.layout.sections.len(), 2);
        assert_eq!(inspect.rows().len(), 2);
        key(&mut s, KeyCode::Down);
        render(&mut s, 160, 40);
        assert_eq!(
            s.mode.inspection().unwrap().review.layout.sections[1].start,
            s.mode.inspection().unwrap().review.layout.offset
        );
        s.paste("中文");
        assert_eq!(s.mode.inspection().unwrap().rows().len(), 1);
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        let Mode::Stashes(view) = &s.mode else {
            panic!()
        };
        let View::Diff { content, .. } = view.as_ref() else {
            panic!("{:?}", view)
        };
        assert_eq!(
            content.labels(),
            (
                i18n::tr!(StashAbsent).into(),
                i18n::tr!(StashSavedUntracked).into()
            )
        );
        assert!(render(&mut s, 160, 40).contains("saved-untracked"));
        assert_eq!(
            s.command_context(),
            Some(crate::command::context::Context::Diff(
                crate::command::context::DiffContext::History
            ))
        );
        assert!(
            s.breadcrumbs()
                .iter()
                .any(|crumb| crumb.label == "stash@{0}" && crumb.destination.is_some())
        );
        s.follow_breadcrumb(crate::navigation::breadcrumbs::Destination::Current(
            crate::navigation::breadcrumbs::Level::StashEntry,
        ));
        assert_eq!(s.mode.inspection().unwrap().query, "中文");
        assert_eq!(s.mode.inspection().unwrap().rows().len(), 1);
        key(&mut s, KeyCode::Esc);
        key(&mut s, KeyCode::Esc);
        assert!(matches!(&s.mode, Mode::Stashes(view) if matches!(view.as_ref(), View::List(_))));
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        assert_eq!(support::git(&f.repo, &["show-ref"]), refs);
        assert_eq!(
            std::fs::read_to_string(f.repo.join("tracked.ts")).unwrap(),
            "current-worktree\n"
        );
        assert!(s.service.store.report().unwrap().is_none());
        s.shutdown();
    }
}
#[test]
fn scope_options_show_current_value_and_compare_original_staging_without_affecting_restore() {
    let f = prepared();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo stash");
    wait(&mut s);
    let a = Action::StashPreviewMenu;
    assert!(
        s.command_availability(a, Some(&f.repo), &s.copy_selection())
            .reason()
            .is_some()
    );
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    loaded(&mut s);
    s.paste("//repo stash scope ");
    let bar::InputBar::Command {
        menu: command::completion::Menu::Options(options),
        ..
    } = &s.bar
    else {
        panic!("options")
    };
    assert_eq!(options.options.len(), 3);
    assert_eq!(
        options.current,
        Some(command::options::Value::StashPreview(Scope::All))
    );
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Tab);
    assert_eq!(scope(&s), Scope::All);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    loaded(&mut s);
    assert_eq!(scope(&s), Scope::Staged);
    assert_eq!(s.mode.inspection().unwrap().rows().len(), 1);
    let text = render(&mut s, 160, 40);
    assert!(
        text.contains("'base'") && text.contains("'staged'"),
        "{text}"
    );
    assert!(!text.contains("saved-worktree") && !text.contains("saved-untracked"));
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    let Mode::Stashes(view) = &s.mode else {
        panic!()
    };
    let View::Diff { content, .. } = view.as_ref() else {
        panic!()
    };
    assert_eq!(
        content.labels(),
        (
            i18n::tr!(StashBase).into(),
            i18n::tr!(StashSavedIndex).into()
        )
    );
    key(&mut s, KeyCode::Esc);
    command(&mut s, "repo stash scope unstaged");
    wait(&mut s);
    loaded(&mut s);
    let text = render(&mut s, 160, 40);
    assert!(
        text.contains("'staged'")
            && text.contains("saved-worktree")
            && text.contains("saved-untracked"),
        "{text}"
    );
    assert!(!text.contains("'base'"));
    command(&mut s, "repo stash scope all");
    wait(&mut s);
    assert_eq!(scope(&s), Scope::All);
    s.shutdown();
}
#[test]
fn narrow_inspection_defers_inline_loading_and_mouse_focus_and_full_file_navigation_work() {
    let f = prepared();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    open(&mut s);
    render(&mut s, 60, 18);
    assert!(!s.poll_inspection());
    assert_eq!(
        view::inspection::areas(s.body_area.unwrap(), s.mode.inspection().unwrap()).len(),
        2
    );
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(render(&mut s, 60, 18).contains("saved-untracked"));
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.mode.inspection().unwrap().files.selected(), Some(1));
    loaded(&mut s);
    let areas = view::inspection::areas(s.body_area.unwrap(), s.mode.inspection().unwrap());
    let preview = areas
        .iter()
        .find(|(_, focus)| *focus == Focus::Preview)
        .unwrap()
        .0;
    let files = areas
        .iter()
        .find(|(_, focus)| *focus == Focus::Files)
        .unwrap()
        .0;
    s.scroll_input = Default::default();
    assert!(wheel(&mut s, ScrollDown, preview.x + 2, preview.y + 2));
    assert_eq!(s.mode.inspection().unwrap().focus, Focus::Preview);
    s.cycle_pane(false);
    s.cycle_pane(false);
    assert_eq!(s.mode.inspection().unwrap().focus, Focus::Files);
    click(&mut s, files.x + 2, files.y + 1);
    assert_eq!(s.mode.inspection().unwrap().files.selected(), Some(0));
    click(&mut s, files.x + 2, files.y + 1);
    wait(&mut s);
    assert!(matches!(&s.mode, Mode::Stashes(view) if matches!(view.as_ref(), View::Diff { .. })));
    key(&mut s, KeyCode::Esc);
    command(&mut s, "zoom");
    assert_eq!(
        view::inspection::areas(s.body_area.unwrap(), s.mode.inspection().unwrap()).len(),
        1
    );
    key(&mut s, KeyCode::Esc);
    for (w, h) in [(160, 40), (60, 18), (24, 8), (1, 1), (0, 0), (160, 40)] {
        render(&mut s, w, h);
    }
    assert_eq!(s.mode.inspection().unwrap().files.selected(), Some(0));
    s.shutdown();
}
#[test]
fn stash_image_diff_decodes_saved_pixels_and_uses_saved_snapshot_labels() {
    use image::{ImageFormat, Rgba, RgbaImage};
    let bytes = |color| {
        let mut output = std::io::Cursor::new(Vec::new());
        RgbaImage::from_pixel(8, 4, Rgba(color))
            .write_to(&mut output, ImageFormat::Png)
            .unwrap();
        output.into_inner()
    };
    let f = support::Fixture::create();
    std::fs::write(f.repo.join("image.png"), bytes([255, 0, 0, 255])).unwrap();
    support::git(&f.repo, &["add", "."]);
    support::git(&f.repo, &["commit", "-m", "image base"]);
    std::fs::write(f.repo.join("image.png"), bytes([0, 255, 0, 255])).unwrap();
    support::git(&f.repo, &["stash", "push", "-m", "image stash"]);
    std::fs::write(f.repo.join("image.png"), bytes([0, 0, 255, 255])).unwrap();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    open(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    let Mode::Stashes(view) = &s.mode else {
        panic!()
    };
    let View::Diff { content, .. } = view.as_ref() else {
        panic!()
    };
    let document::View::Images(images) = content.as_ref() else {
        panic!()
    };
    let preview::Side::Image { pixels: before, .. } = &images.pairs[0].before else {
        panic!()
    };
    let preview::Side::Image { pixels: after, .. } = &images.pairs[0].after else {
        panic!()
    };
    assert_eq!(before.get_pixel(0, 0).0, [255, 0, 0, 255]);
    assert_eq!(after.get_pixel(0, 0).0, [0, 255, 0, 255]);
    let text = render(&mut s, 160, 40);
    assert!(text.contains('\u{10eeee}'), "{text}");
    s.shutdown();
}

#[test]
fn restoring_from_stash_details_preserves_its_file_filter_and_dropping_returns_to_the_list() {
    let f = prepared();
    support::git(&f.repo, &["restore", "--worktree", "--", "tracked.ts"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    open(&mut s);
    s.paste("tracked.ts");
    let oid = s.mode.inspection().unwrap().detail.oid();
    command(&mut s, &format!("repo stash apply {oid}"));
    wait(&mut s);
    assert!(matches!(s.mode, Mode::Confirm { .. }));
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    let inspect = s.mode.inspection().expect("apply retains content view");
    assert_eq!(inspect.query, "tracked.ts");
    assert_eq!(inspect.rows().len(), 1);
    assert!(
        std::fs::read_to_string(f.repo.join("tracked.ts"))
            .unwrap()
            .contains("saved-worktree")
    );
    command(&mut s, &format!("repo stash drop {oid}"));
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(
        matches!(&s.mode, Mode::Stashes(view) if matches!(view.as_ref(), View::List(_)) && view.session().snapshot.entries.is_empty())
    );
    assert!(
        std::fs::read_to_string(f.repo.join("tracked.ts"))
            .unwrap()
            .contains("saved-worktree")
    );
    s.shutdown();
}
