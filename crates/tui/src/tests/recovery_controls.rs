use super::*;
use crate::replay::controls::Focus;
use engine::operations::{
    mutation,
    replay::{Control, Plan},
};

fn conflict() -> support::Fixture {
    let f = support::Fixture::create();
    f.commit("conflict.txt", "base\n");
    support::git(&f.repo, &["checkout", "-b", "source"]);
    f.commit("conflict.txt", "incoming\n");
    let source = support::git(&f.repo, &["rev-parse", "HEAD"]);
    support::git(&f.repo, &["checkout", "main"]);
    f.commit("conflict.txt", "current\n");
    let result = std::process::Command::new("/usr/bin/git")
        .arg("-C")
        .arg(&f.repo)
        .args(["cherry-pick", &source])
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert!(!support::git(&f.repo, &["ls-files", "-u"]).is_empty());
    f
}
fn focused(s: &Screen) -> Option<Control> {
    s.focused_replay_control()
}
fn backtab(s: &mut Screen) {
    key(s, KeyCode::BackTab);
}
fn reviewed(s: &Screen) -> Control {
    let Mode::Confirm {
        operation: Confirmation::Mutation { plan, .. },
        ..
    } = &s.mode
    else {
        panic!("review: {:?}", s.mode);
    };
    let mutation::Plan::Replay(Plan::Control(plan)) = plan.as_ref() else {
        panic!("recovery review");
    };
    plan.control
}
#[test]
fn keyboard_cycles_content_and_actions_without_scrolling_or_changing_files() {
    let f = conflict();
    let before = support::git(&f.repo, &["rev-parse", "HEAD"]);
    let contents = std::fs::read(f.repo.join("conflict.txt")).unwrap();
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    assert_eq!(focused(&s), None);
    render(&mut s, 140, 38);
    let geometry = (s.body_area, s.input_area);
    key(&mut s, KeyCode::Tab);
    assert_eq!(focused(&s), None);
    s.cycle_pane(false);
    backtab(&mut s);
    assert_eq!(focused(&s), Some(Control::Continue));
    key(&mut s, KeyCode::Tab);
    assert_eq!(focused(&s), Some(Control::Continue));
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.message, i18n::tr!(ReplayResolveFirst));
    key(&mut s, KeyCode::Char(' '));
    assert_eq!(focused(&s), Some(Control::Continue));
    assert!(s.mode.file_session().unwrap().selected.is_empty());
    assert!(s.repository_tasks.is_empty());
    for (width, height) in [(1, 1), (30, 8), (60, 18), (140, 38)] {
        render(&mut s, width, height);
    }
    key(&mut s, KeyCode::Right);
    assert_eq!(focused(&s), Some(Control::Skip));
    key(&mut s, KeyCode::Right);
    assert_eq!(focused(&s), Some(Control::Abort));
    key(&mut s, KeyCode::Right);
    assert_eq!(focused(&s), Some(Control::Continue));
    key(&mut s, KeyCode::Left);
    assert_eq!(focused(&s), Some(Control::Abort));
    let text = render(&mut s, 140, 38);
    let action_row = text
        .lines()
        .position(|line| line.contains(i18n::tr!(ReplayAbortShort)))
        .unwrap();
    assert!(
        action_row
            > text
                .lines()
                .position(|line| line.contains("<<<<<<<"))
                .unwrap(),
        "{text}"
    );
    let input = s.input_area.unwrap();
    assert_eq!(action_row, view::input::content_area(input).y as usize);
    assert_eq!((s.body_area, s.input_area), geometry);
    key(&mut s, KeyCode::Esc);
    assert_eq!(focused(&s), None);
    assert!(
        matches!(&s.mode, Mode::Changes(session) if session.focus == crate::changes::Focus::Preview)
    );
    backtab(&mut s); // 不必先切到文件面板。
    assert_eq!(focused(&s), Some(Control::Continue));
    backtab(&mut s);
    assert_eq!(focused(&s), None);
    assert!(
        matches!(&s.mode, Mode::Changes(session) if session.focus == crate::changes::Focus::Preview)
    );
    assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), before);
    assert_eq!(
        std::fs::read(f.repo.join("conflict.txt")).unwrap(),
        contents
    );
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    s.shutdown();
}
#[test]
fn keyboard_reviews_abort_restores_focus_on_cancel_and_confirms_explicitly() {
    let f = conflict();
    let before = support::git(&f.repo, &["rev-parse", "HEAD"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    backtab(&mut s);
    key(&mut s, KeyCode::Left);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(reviewed(&s), Control::Abort);
    assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), before);
    key(&mut s, KeyCode::Esc);
    assert!(s.bar.is_plain());
    assert_eq!(focused(&s), Some(Control::Abort));
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(reviewed(&s), Control::Abort);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(focused(&s), None);
    assert!(s.replay_controls().is_none());
    assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), before);
    assert_eq!(
        std::fs::read_to_string(f.repo.join("conflict.txt")).unwrap(),
        "current\n"
    );
    s.shutdown();
}
#[test]
fn resolving_and_staging_enables_keyboard_continue_and_removes_the_toolbar() {
    let f = conflict();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    backtab(&mut s);
    s.paste("//editor");
    assert_eq!(focused(&s), None);
    assert_eq!(s.bar.text(), "/editor");
    key(&mut s, KeyCode::Esc);
    support::write(f.repo.join("conflict.txt"), "resolved\n");
    command(&mut s, "repo diff stage");
    wait(&mut s);
    backtab(&mut s);
    assert_eq!(focused(&s), Some(Control::Continue));
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(reviewed(&s), Control::Continue);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(s.replay_controls().is_none());
    assert_eq!(
        support::git(&f.repo, &["show", "HEAD:conflict.txt"]),
        "resolved"
    );
    assert!(support::git(&f.repo, &["status", "--porcelain"]).is_empty());
    s.shutdown();
}
#[test]
fn worktree_pane_has_keyboard_actions_and_keeps_them_out_of_the_file_list() {
    let f = conflict();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo");
    wait_workspace(&mut s);
    s.switch_repository_section(crate::workspace::Section::Worktree);
    let text = render(&mut s, 140, 38);
    let pane = s
        .workspace_areas
        .iter()
        .find(|(_, section)| *section == crate::workspace::Section::Worktree)
        .unwrap()
        .0;
    assert!(
        !text
            .lines()
            .any(|line| line.contains(i18n::tr!(ReplayAbortShort)))
    );
    let geometry = (s.body_area, s.input_area);
    backtab(&mut s);
    assert_eq!(focused(&s), Some(Control::Continue));
    let text = render(&mut s, 140, 38);
    let action_row = text
        .lines()
        .position(|line| line.contains(i18n::tr!(ReplayAbortShort)))
        .unwrap();
    assert_eq!(
        action_row,
        view::input::content_area(s.input_area.unwrap()).y as usize
    );
    assert!(action_row >= pane.bottom() as usize, "{text}");
    assert_eq!((s.body_area, s.input_area), geometry);
    key(&mut s, KeyCode::Right);
    assert_eq!(focused(&s), Some(Control::Skip));
    backtab(&mut s);
    assert_eq!(focused(&s), None);
    let Mode::Files(files) = &s.mode else {
        panic!("worktree files");
    };
    assert!(matches!(files.recovery, Focus::Content));
    s.shutdown();
}

#[test]
fn actions_restore_search_drafts_and_typing_from_preview_resumes_file_search() {
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let f = conflict();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        command(&mut s, "repo diff");
        wait(&mut s);
        s.paste("conf");
        key(&mut s, KeyCode::Left);
        key(&mut s, KeyCode::Left);
        let original = (s.bar.editor().text.clone(), s.bar.editor().cursor);
        backtab(&mut s);
        key(&mut s, KeyCode::Right);
        render(&mut s, 100, 30);
        key(&mut s, KeyCode::Esc);
        assert_eq!(
            (s.bar.editor().text.clone(), s.bar.editor().cursor),
            original
        );
        for trigger in if language == i18n::Language::Chinese {
            vec!['/', '、']
        } else {
            vec!['/']
        } {
            backtab(&mut s);
            key(&mut s, KeyCode::Char(trigger));
            assert!(matches!(s.bar, bar::InputBar::Command { .. }));
            assert_eq!(s.mode.file_session().unwrap().query, "conf");
            key(&mut s, KeyCode::Esc);
        }
        backtab(&mut s);
        s.paste("/editor");
        assert_eq!(s.bar.text(), "/editor");
        assert_eq!(s.mode.file_session().unwrap().query, "conf");
        key(&mut s, KeyCode::Esc);
        key(&mut s, KeyCode::Esc); // 清空搜索。
        s.cycle_pane(false);
        backtab(&mut s);
        assert_eq!(focused(&s), Some(Control::Continue));
        key(&mut s, KeyCode::Char('c'));
        assert_eq!(focused(&s), None);
        assert_eq!(s.bar.editor().text, "c");
        assert!(
            matches!(&s.mode, Mode::Changes(session) if session.focus == crate::changes::Focus::Files)
        );
        key(&mut s, KeyCode::Esc);
        s.cycle_pane(false);
        backtab(&mut s);
        s.paste("中文");
        assert_eq!(s.bar.editor().text, "中文");
        assert_eq!(focused(&s), None);
        key(&mut s, KeyCode::Esc);
        s.cycle_pane(false);
        backtab(&mut s);
        key(&mut s, KeyCode::Char('/'));
        assert!(matches!(s.bar, bar::InputBar::Command { .. }));
        assert!(
            matches!(&s.mode, Mode::Changes(session) if session.focus == crate::changes::Focus::Preview)
        );
        assert_eq!(focused(&s), None);
        s.shutdown();
    }
}
#[test]
fn recovery_input_selection_repaints_gaps_and_disappears_without_stale_color() {
    use ratatui::{backend::Backend, style::Color};
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let f = conflict();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        command(&mut s, "repo diff");
        wait(&mut s);
        backtab(&mut s);
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
        let inner = view::input::content_area(s.input_area.unwrap());
        let gap = (
            inner.x
                + unicode_width::UnicodeWidthStr::width(i18n::tr!(ReplayContinueShort)) as u16
                + 2,
            inner.y,
        );
        assert_eq!(terminal.backend().buffer()[gap].symbol(), " ");
        terminal
            .backend_mut()
            .draw(std::iter::once((
                gap.0,
                gap.1,
                &ratatui::buffer::Cell::default().set_bg(Color::Red).clone(),
            )))
            .unwrap();
        key(&mut s, KeyCode::Right);
        terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
        assert_eq!(terminal.backend().buffer()[gap].bg, Color::Reset);
        key(&mut s, KeyCode::Esc);
        let expected = terminal
            .draw(|frame| view::render(frame, &mut s))
            .unwrap()
            .buffer
            .clone();
        let mut x = inner.x;
        while x < inner.right() {
            let cell = &expected[(x, inner.y)];
            assert_eq!(terminal.backend().buffer()[(x, inner.y)].bg, Color::Reset);
            // TestBackend 不单独绘制宽字形续格；实际终端由字形首格覆盖它们。
            x += unicode_width::UnicodeWidthStr::width(cell.symbol()).max(1) as u16;
        }
        s.shutdown();
    }
}

#[test]
fn shift_tab_retains_command_drafts_cursors_and_targets_then_tab_still_completes() {
    let f = conflict();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    s.paste("//repo ab");
    key(&mut s, KeyCode::Left);
    let original = (s.bar.text(), s.bar.editor().cursor);
    let toggle = KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT);
    s.key(toggle, &Cancel::default()).unwrap();
    assert_eq!(focused(&s), Some(Control::Continue));
    key(&mut s, KeyCode::Right);
    key(&mut s, KeyCode::Tab);
    assert_eq!(focused(&s), Some(Control::Skip));
    s.key(toggle, &Cancel::default()).unwrap();
    assert_eq!((s.bar.text(), s.bar.editor().cursor), original);
    assert!(matches!(&s.bar, bar::InputBar::Command { path: Some(path), .. } if path == &f.repo));
    key(&mut s, KeyCode::Right);
    key(&mut s, KeyCode::Tab);
    assert_eq!(s.bar.text(), "/repo abort");
    assert!(s.repository_tasks.is_empty());
    s.shutdown();
}

#[test]
fn views_without_recovery_options_do_not_toggle_or_show_a_mode_hint() {
    let f = support::Fixture::create();
    support::write(f.repo.join("draft.txt"), "draft\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    s.paste("draft");
    let input = s.bar.text();
    for code in [KeyCode::Tab, KeyCode::BackTab] {
        key(&mut s, code);
        assert_eq!(s.bar.text(), input);
        assert!(
            matches!(&s.mode, Mode::Changes(session) if session.focus == crate::changes::Focus::Files)
        );
    }
    assert!(!render(&mut s, 140, 38).contains(i18n::tr!(ReplayControlsEntryHint)));
    assert_eq!(focused(&s), None);
    s.shutdown();
}
