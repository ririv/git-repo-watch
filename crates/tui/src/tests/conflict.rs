use super::*;
use crate::{
    conflict::{Body, DisplayMode, Focus},
    document::View,
};

pub(super) fn automatic_setup() -> (support::Fixture, tempfile::TempDir, Screen) {
    let f = support::Fixture::create();
    std::fs::create_dir_all(f.repo.join("src")).unwrap();
    let base = (0..35)
        .map(|n| format!("export\tconst shared_{n} = {n};\n"))
        .collect::<String>();
    let path = "src/中文 空格.ts";
    f.commit(path, &format!("{base}export const value = \"BASE\";\n"));
    support::git(&f.repo, &["checkout", "-b", "incoming"]);
    f.commit(path, &format!("{base}export const value = \"INCOMING\";\n"));
    let incoming = support::git(&f.repo, &["rev-parse", "HEAD"]);
    support::git(&f.repo, &["checkout", "main"]);
    f.commit(path, &format!("{base}export const value = \"CURRENT\";\n"));
    let output = std::process::Command::new("/usr/bin/git")
        .arg("-C")
        .arg(&f.repo)
        .args(["cherry-pick", &incoming])
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    let (home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    (f, home, s)
}
fn setup() -> (support::Fixture, tempfile::TempDir, Screen) {
    let (f, home, mut s) = automatic_setup();
    // 下列既有用例分别操作文件列表、比较布局或快捷命令，显式选择输入页。
    if let Mode::Changes(session) = &mut s.mode {
        session.focus = crate::changes::Focus::Files;
    }
    s.show_input_page(crate::input_pages::Page::Input);
    s.sync_plain_bar();
    (f, home, s)
}

fn review(s: &Screen) -> &crate::conflict::View {
    let Mode::Changes(session) = &s.mode else {
        panic!("changes")
    };
    let crate::changes::Preview::Ready { content, .. } = &session.preview else {
        panic!("ready")
    };
    let View::Conflict(view) = content.as_ref() else {
        panic!("conflict")
    };
    view
}
fn scroll(s: &mut Screen, n: usize) {
    let Mode::Changes(session) = &mut s.mode else {
        panic!()
    };
    let crate::changes::Preview::Ready { scroll, .. } = &mut session.preview else {
        panic!()
    };
    *scroll = n;
}
fn activate(s: &mut Screen, focus: Focus) {
    if s.body_area.is_none() {
        render(s, 210, 40);
    }
    if let Mode::Changes(session) = &mut s.mode {
        session.focus = crate::changes::Focus::Preview;
    }
    command(s, "focus");
    key(
        s,
        if focus == Focus::Compare {
            KeyCode::Up
        } else {
            KeyCode::Down
        },
    );
    key(s, KeyCode::Enter);
    assert_eq!(review(s).focus(), focus);
}
#[test]
fn conflict_preview_omits_partial_diff_commands() {
    let (_f, _home, mut s) = setup();
    activate(&mut s, Focus::Result);
    s.paste("//repo diff ");
    assert!(
        s.search_commands()
            .unwrap()
            .iter()
            .all(|command| !matches!(command.action, Action::Discard | Action::PartialSelectMenu))
    );
    assert!(matches!(&s.mode, Mode::Changes(session)
        if session.focus == crate::changes::Focus::Preview));
    s.shutdown();
}

#[test]
fn inputs_and_actual_source_result_remain_visible_with_an_optional_base_at_every_size() {
    let (f, _home, mut s) = setup();
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    let working = std::fs::read(f.repo.join("src/中文 空格.ts")).unwrap();
    assert_eq!(review(&s).focus(), Focus::Result);
    assert_eq!(review(&s).mode, DisplayMode::Overview);
    for (n, expected) in ["BASE", "CURRENT", "INCOMING"].into_iter().enumerate() {
        let Body::Text { lines, .. } = &review(&s).columns[n].body else {
            panic!()
        };
        assert!(lines.last().unwrap().to_string().contains(expected));
        assert!(
            lines
                .last()
                .unwrap()
                .text
                .spans
                .iter()
                .any(|s| matches!(s.style.fg, Some(ratatui::style::Color::Rgb(..))))
        );
    }
    let wide = render(&mut s, 210, 55);
    assert!(
        wide.lines()
            .any(|l| l.contains("CURRENT") && l.contains("INCOMING")),
        "{wide}"
    );
    assert!(wide.contains("<<<<<<<"), "{wide}");
    assert!(wide.contains(i18n::tr!(ConflictResult)), "{wide}");
    assert!(!wide.contains(i18n::tr!(ConflictBase)), "{wide}");
    command(&mut s, "repo diff conflict include-base");
    let base = render(&mut s, 210, 55);
    let row = |value: &str| base.lines().position(|line| line.contains(value)).unwrap();
    assert!(row("\"BASE\"") < row("\"CURRENT\""), "{base}");
    assert!(row("\"BASE\"") < row("\"INCOMING\""), "{base}");
    assert!(
        base.lines()
            .any(|line| line.contains("CURRENT") && line.contains("INCOMING")),
        "{base}"
    );
    assert!(base.contains("<<<<<<<"), "{base}");
    for (width, height) in [(80, 40), (50, 24), (24, 8), (1, 1), (0, 0), (210, 55)] {
        render(&mut s, width, height);
    }
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert_eq!(
        std::fs::read(f.repo.join("src/中文 空格.ts")).unwrap(),
        working
    );
    s.shutdown();
}
#[test]
fn base_option_completion_does_not_apply_until_enter_or_hide_the_working_result() {
    let (_f, _home, mut s) = setup();
    s.paste("中文");
    command(&mut s, "repo diff conflict");
    assert_eq!(super::options::choices(&s).options.len(), 8);
    assert_eq!(
        super::options::choices(&s).current,
        Some(command::options::Value::Conflict(DisplayMode::Overview))
    );
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Tab);
    assert!(s.bar.text().ends_with("conflict include-base"));
    assert_eq!(review(&s).mode, DisplayMode::Overview);
    key(&mut s, KeyCode::Enter);
    assert_eq!(review(&s).mode, DisplayMode::IncludeBase);
    assert_eq!(s.bar.text(), "中文");
    assert!(render(&mut s, 180, 55).contains("<<<<<<<"));
    command(&mut s, "repo diff conflict overview");
    assert_eq!(review(&s).mode, DisplayMode::Overview);
    assert!(render(&mut s, 180, 55).contains("<<<<<<<"));
    s.shutdown();
}
#[test]
fn focus_and_external_refresh_retain_separate_reading_positions_and_the_base_choice() {
    let (f, _home, mut s) = setup();
    render(&mut s, 210, 40);
    command(&mut s, "repo diff conflict include-base");
    activate(&mut s, Focus::Compare);
    scroll(&mut s, 12);
    render(&mut s, 210, 40);
    let comparison_scroll = match &s.mode {
        Mode::Changes(s) => match &s.preview {
            crate::changes::Preview::Ready { scroll, .. } => *scroll,
            _ => unreachable!(),
        },
        _ => unreachable!(),
    };
    activate(&mut s, Focus::Result);
    scroll(&mut s, 8);
    render(&mut s, 210, 40);
    let mut result = std::fs::read_to_string(f.repo.join("src/中文 空格.ts")).unwrap();
    result.push_str("// edited externally\n");
    support::write(f.repo.join("src/中文 空格.ts"), &result);
    command(&mut s, "refresh");
    wait(&mut s);
    assert_eq!(review(&s).focus(), Focus::Result);
    assert_eq!(review(&s).mode, DisplayMode::IncludeBase);
    let crate::conflict::result::Content::Source {
        body: Body::Text { source, .. },
        ..
    } = &review(&s).result.content
    else {
        panic!()
    };
    assert!(source.ends_with("// edited externally\n"));
    activate(&mut s, Focus::Compare);
    assert!(
        matches!(&s.mode,Mode::Changes(s) if matches!(&s.preview,crate::changes::Preview::Ready{scroll,..} if *scroll==comparison_scroll))
    );
    let Body::Text { lines, .. } = &review(&s).columns[1].body else {
        panic!()
    };
    assert!(
        !lines
            .iter()
            .any(|l| l.to_string().contains("edited externally"))
    );
    s.shutdown();
}
#[test]
fn copy_follows_the_focused_source_result_or_visible_input_versions() {
    let (f, _home, mut s) = setup();
    activate(&mut s, Focus::Result);
    let payload = s
        .copy_selection()
        .payload(crate::clipboard::Scope::Diff, Some(&f.repo))
        .unwrap();
    assert_eq!(payload.target, crate::clipboard::Target::ConflictResult);
    assert_eq!(
        payload.text,
        std::fs::read_to_string(f.repo.join("src/中文 空格.ts")).unwrap()
    );
    activate(&mut s, Focus::Compare);
    let payload = s
        .copy_selection()
        .payload(crate::clipboard::Scope::Diff, Some(&f.repo))
        .unwrap();
    assert_eq!(payload.target, crate::clipboard::Target::Conflict);
    for text in ["export\tconst shared_0", "CURRENT", "INCOMING"] {
        assert!(payload.text.contains(text));
    }
    assert!(!payload.text.contains("<<<<<<<"));
    assert!(!payload.text.contains("BASE"));
    command(&mut s, "repo diff conflict include-base");
    let payload = s
        .copy_selection()
        .payload(crate::clipboard::Scope::Diff, Some(&f.repo))
        .unwrap();
    assert!(payload.text.contains("BASE"));
    s.shutdown();
}

#[test]
fn missing_final_newline_is_an_unnumbered_annotation_and_is_not_copied_as_source() {
    let (f, _home, mut s) = setup();
    // 通过合法的冲突阶段更新制造仅换行不同的版本，工作区仍保留原始结果。
    let mut child = std::process::Command::new("/usr/bin/git")
        .arg("-C")
        .arg(&f.repo)
        .args(["hash-object", "-w", "--stdin"])
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"no final newline")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let oid = String::from_utf8(output.stdout).unwrap();
    let mut child = std::process::Command::new("/usr/bin/git")
        .arg("-C")
        .arg(&f.repo)
        .args(["update-index", "--index-info"])
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    writeln!(
        child.stdin.take().unwrap(),
        "100644 {} 2\tsrc/中文 空格.ts",
        oid.trim()
    )
    .unwrap();
    assert!(child.wait().unwrap().success());
    command(&mut s, "refresh");
    wait(&mut s);
    render(&mut s, 210, 40);
    let cache = review(&s).cache.as_ref().unwrap();
    let annotation = cache
        .rows
        .iter()
        .find(|r| r.kind == crate::diff::RowKind::Annotation)
        .unwrap();
    assert_eq!(
        annotation.cells[1].to_string().trim(),
        crate::diff::notice::NO_NEWLINE
    );
    assert!(annotation.cells[0].to_string().is_empty());
    assert!(
        !review(&s)
            .export()
            .contains(crate::diff::notice::NO_NEWLINE)
    );
    s.shutdown();
}

#[test]
fn empty_and_binary_versions_never_copy_placeholder_messages_as_file_content() {
    for (name, base, current, incoming, copyable) in [
        ("empty.txt", "base\n", "", "incoming\n", true),
        ("binary.dat", "base\0", "current\0", "incoming\0", false),
    ] {
        let f = support::Fixture::create();
        f.commit(name, base);
        support::git(&f.repo, &["checkout", "-b", "incoming"]);
        f.commit(name, incoming);
        let oid = support::git(&f.repo, &["rev-parse", "HEAD"]);
        support::git(&f.repo, &["checkout", "main"]);
        f.commit(name, current);
        let output = std::process::Command::new("/usr/bin/git")
            .arg("-C")
            .arg(&f.repo)
            .args(["cherry-pick", &oid])
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        command(&mut s, "repo diff");
        wait(&mut s);
        activate(&mut s, Focus::Compare);
        let text = render(&mut s, 180, 38);
        let payload = s
            .copy_selection()
            .payload(crate::clipboard::Scope::Diff, Some(&f.repo));
        if copyable {
            assert!(text.contains(i18n::tr!(ConflictEmpty)), "{text}");
            let payload = payload.unwrap();
            assert_eq!(payload.target, crate::clipboard::Target::Conflict);
            assert!(!payload.text.contains(i18n::tr!(ConflictEmpty)));
        } else {
            assert_eq!(
                payload.err().unwrap().to_string(),
                i18n::tr!(CopyConflictUnavailable)
            );
        }
        s.shutdown();
    }
}

#[test]
fn focus_navigation_follows_files_inputs_result_geometry_without_changing_content_keys() {
    let (_f, _home, mut s) = setup();
    for (width, enter, leave) in [
        (180, KeyCode::Right, KeyCode::Left),
        (80, KeyCode::Down, KeyCode::Up),
    ] {
        if let Mode::Changes(session) = &mut s.mode {
            session.focus = crate::changes::Focus::Files;
        }
        render(&mut s, width, 44);
        command(&mut s, "focus");
        key(&mut s, enter);
        assert!(
            matches!(&s.mode,Mode::Changes(session) if session.focus==crate::changes::Focus::Preview)
        );
        key(&mut s, KeyCode::Down);
        assert_eq!(review(&s).focus(), Focus::Result);
        key(&mut s, KeyCode::Up);
        assert_eq!(review(&s).focus(), Focus::Compare);
        key(&mut s, leave);
        assert!(
            matches!(&s.mode,Mode::Changes(session) if session.focus==crate::changes::Focus::Files)
        );
        key(&mut s, KeyCode::Enter);
    }
    s.shutdown();
}
#[test]
fn pointer_and_scrollbar_seek_their_own_conflict_region_and_respect_pane_selection() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let (_f, _home, mut s) = setup();
    render(&mut s, 180, 44);
    let area = match &s.mode {
        Mode::Changes(session) => crate::view::changes::areas(s.body_area.unwrap(), session)
            .into_iter()
            .find_map(|(r, f)| (f == crate::changes::Focus::Preview).then_some(r))
            .unwrap(),
        _ => unreachable!(),
    };
    let parts = crate::view::conflict::areas(area, review(&s).mode);
    let (compare, result) = (parts.comparison, parts.result.unwrap());
    let event = |kind, area: ratatui::layout::Rect| MouseEvent {
        kind,
        column: area.x + 3,
        row: area.y + 3,
        modifiers: KeyModifiers::NONE,
    };
    s.mouse(
        event(MouseEventKind::Down(MouseButton::Left), compare),
        &Cancel::default(),
    )
    .unwrap();
    assert_eq!(review(&s).focus(), Focus::Compare);
    assert!(
        s.mouse(
            event(MouseEventKind::Down(MouseButton::Left), result),
            &Cancel::default()
        )
        .unwrap()
    );
    assert_eq!(review(&s).focus(), Focus::Result);
    assert!(
        s.mouse(
            event(MouseEventKind::Down(MouseButton::Left), compare),
            &Cancel::default()
        )
        .unwrap()
    );

    scroll(&mut s, 0);
    render(&mut s, 180, 44);
    s.mouse(
        event(MouseEventKind::ScrollDown, compare),
        &Cancel::default(),
    )
    .unwrap();
    assert!(
        review(&s).scrolls(match &s.mode {
            Mode::Changes(session) => match &session.preview {
                crate::changes::Preview::Ready { scroll, .. } => *scroll,
                _ => 0,
            },
            _ => 0,
        })[0]
            > 0
    );
    // 直接命中另一块的滚动条应切换到它，不能修改比较区的活动位置。
    let seek = |area: ratatui::layout::Rect, row| MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: area.right() - 1,
        row: area.y + row,
        modifiers: KeyModifiers::NONE,
    };
    assert!(matches!(
        s.pointer.hit(ratatui::layout::Position::new(
            result.right() - 1,
            result.y + 2
        )),
        Some(crate::pointer::Hit::Scrollbar(
            crate::pointer::Action::Scroll(
                crate::pointer::ScrollTarget::Conflict(Focus::Result),
                0
            )
        ))
    ));
    assert!(s.mouse(seek(result, 2), &Cancel::default()).unwrap());
    assert_eq!(review(&s).focus(), Focus::Result);
    assert!(
        matches!(&s.mode,Mode::Changes(session) if matches!(&session.preview,crate::changes::Preview::Ready{scroll,..} if *scroll==0))
    );
    s.config.mouse.select_pane = false;
    s.mouse(
        event(MouseEventKind::Down(MouseButton::Left), compare),
        &Cancel::default(),
    )
    .unwrap();
    assert_eq!(review(&s).focus(), Focus::Result);
    assert!(!s.mouse(seek(compare, 3), &Cancel::default()).unwrap());
    s.mouse(
        event(MouseEventKind::Down(MouseButton::Left), result),
        &Cancel::default(),
    )
    .unwrap();
    assert_eq!(review(&s).focus(), Focus::Result);
    s.shutdown();
}

#[test]
fn all_pairs_use_their_own_sources_in_order_without_navigation_or_replacing_the_file_list() {
    use crate::conflict::pair::Pair;
    use i18n::Message as M;
    let (f, _home, mut s) = setup();
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    s.paste("中文");
    render(&mut s, 210, 55);
    let geometry = match &s.mode {
        Mode::Changes(session) => crate::view::changes::areas(s.body_area.unwrap(), session),
        _ => panic!(),
    };
    let depth = s.history.len();
    let crumbs = s.breadcrumbs().len();
    for (token, pair, left, right, before, after, excluded) in [
        (
            "base-left",
            Pair::BaseCurrent,
            M::ConflictBase,
            M::ConflictCurrent,
            "BASE",
            "CURRENT",
            Some("INCOMING"),
        ),
        (
            "base-right",
            Pair::BaseIncoming,
            M::ConflictBase,
            M::ConflictIncoming,
            "BASE",
            "INCOMING",
            Some("CURRENT"),
        ),
        (
            "base-result",
            Pair::BaseResult,
            M::ConflictBase,
            M::ConflictResult,
            "BASE",
            "<<<<<<<",
            None,
        ),
        (
            "left-right",
            Pair::CurrentIncoming,
            M::ConflictCurrent,
            M::ConflictIncoming,
            "CURRENT",
            "INCOMING",
            Some("BASE"),
        ),
        (
            "left-result",
            Pair::CurrentResult,
            M::ConflictCurrent,
            M::ConflictResult,
            "CURRENT",
            "<<<<<<<",
            Some("BASE"),
        ),
        (
            "result-right",
            Pair::ResultIncoming,
            M::ConflictResult,
            M::ConflictIncoming,
            "<<<<<<<",
            "INCOMING",
            Some("BASE"),
        ),
    ] {
        command(&mut s, &format!("repo diff conflict {token}"));
        assert_eq!(review(&s).mode, DisplayMode::Pair(pair));
        let text = render(&mut s, 210, 55);
        assert!(text.contains(before) && text.contains(after), "{text}");
        if let Some(excluded) = excluded {
            assert!(!text.contains(excluded), "{text}");
        }
        assert!(text.contains(i18n::tr!(ConflictCompare)), "{text}");
        assert!(text.contains(i18n::tr!(ChangesUnstaged)), "{text}");
        assert!(!text.contains(i18n::tr!(FileDiff)), "{text}");
        for width in [210, 80, 64] {
            let text = render(&mut s, width, 55);
            let heading = text
                .lines()
                .find(|line| line.contains(left.text()) && line.contains(right.text()))
                .expect("两个版本应在同一行按选择顺序左右显示");
            assert!(
                heading.find(left.text()) < heading.find(right.text()),
                "{text}"
            );
            assert_eq!(
                review(&s).cache.as_ref().unwrap().arrangement,
                crate::conflict::layout::Arrangement::Columns
            );
        }
        render(&mut s, 210, 55);
        assert_eq!(s.bar.text(), "中文");
        assert_eq!(s.history.len(), depth);
        assert_eq!(s.breadcrumbs().len(), crumbs);
        let Mode::Changes(session) = &s.mode else {
            panic!()
        };
        assert_eq!(
            crate::view::changes::areas(s.body_area.unwrap(), session),
            geometry
        );
        activate(&mut s, Focus::Compare);
        let payload = s
            .copy_selection()
            .payload(crate::clipboard::Scope::Diff, Some(&f.repo))
            .unwrap();
        assert_eq!(payload.target, crate::clipboard::Target::Conflict);
        assert!(payload.text.contains(before) && payload.text.contains(after));
        assert!(payload.text.find(left.text()) < payload.text.find(right.text()));
        command(&mut s, "repo diff conflict include-base");
        assert!(render(&mut s, 210, 55).contains(i18n::tr!(ConflictResult)));
        if let Mode::Changes(session) = &mut s.mode {
            session.focus = crate::changes::Focus::Files;
        }
        s.sync_bar();
    }
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    s.shutdown();
}
#[test]
fn display_options_complete_only_on_enter_and_refresh_keeps_result_on_the_left() {
    use crate::conflict::pair::Pair;
    let (f, _home, mut s) = setup();
    command(&mut s, "repo diff conflict");
    assert_eq!(super::options::choices(&s).options.len(), 8);
    // 从默认隐藏项向下选择显示全部，再选择基础和当前。
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Tab);
    assert!(s.bar.text().ends_with("base-left"));
    assert_eq!(review(&s).mode, DisplayMode::Overview);
    key(&mut s, KeyCode::Enter);
    assert_eq!(review(&s).mode, DisplayMode::Pair(Pair::BaseCurrent));
    command(&mut s, "repo diff conflict result-right");
    activate(&mut s, Focus::Compare);
    render(&mut s, 210, 55);
    support::write(
        f.repo.join("src/中文 空格.ts"),
        "export const resolved = \"RESOLVED\";\n",
    );
    command(&mut s, "refresh");
    wait(&mut s);
    assert_eq!(review(&s).mode, DisplayMode::Pair(Pair::ResultIncoming));
    key(&mut s, KeyCode::Home);
    let text = render(&mut s, 210, 55);
    assert!(
        text.contains("RESOLVED") && !text.contains("<<<<<<<"),
        "{text}"
    );
    let payload = s
        .copy_selection()
        .payload(crate::clipboard::Scope::Diff, Some(&f.repo))
        .unwrap();
    assert_eq!(payload.target, crate::clipboard::Target::Conflict);
    assert!(payload.text.contains("RESOLVED"));
    assert!(!payload.text.contains("<<<<<<<"));
    assert_eq!(
        support::git(&f.repo, &["ls-files", "-u"]).lines().count(),
        3
    );
    s.shutdown();
}
#[test]
fn conflict_has_one_display_command_no_header_buttons_and_no_pair_back_page() {
    use crate::conflict::pair::Pair;
    let (_f, _home, mut s) = setup();
    assert!(command::COMMANDS.iter().all(
        |cmd| cmd.name != "repo diff conflict base" && cmd.name != "repo diff conflict compare"
    ));
    command(&mut s, "repo diff conflict include-base");
    let text = render(&mut s, 210, 55);
    assert!(!text.contains(i18n::tr!(ConflictCompareCurrent)), "{text}");
    assert!(!text.contains(i18n::tr!(ConflictCompareResult)), "{text}");
    command(&mut s, "repo diff conflict base-left");
    activate(&mut s, Focus::Compare);
    key(&mut s, KeyCode::Esc); // 先返回输入页。
    key(&mut s, KeyCode::Esc); // 再返回文件列表。
    assert_eq!(review(&s).mode, DisplayMode::Pair(Pair::BaseCurrent));
    assert!(
        matches!(&s.mode,Mode::Changes(session) if session.focus==crate::changes::Focus::Files)
    );
    let Mode::Changes(session) = &mut s.mode else {
        panic!()
    };
    session.refreshed(session.files.snapshot.clone());
    let key = match &session.preview {
        crate::changes::Preview::Ready { key, .. } => key.clone(),
        _ => panic!(),
    };
    let previous = std::mem::replace(&mut session.preview, crate::changes::Preview::Loading(key));
    let now = std::time::Instant::now();
    session.feedback.begin(Some(Box::new(previous)), now);
    assert_eq!(
        crate::view::changes::areas(s.body_area.unwrap(), session).len(),
        2
    );
    session.feedback.tick(now + crate::loading::FEEDBACK_DELAY);
    assert_eq!(
        crate::view::changes::areas(s.body_area.unwrap(), session).len(),
        2
    );
    s.shutdown();
}

#[test]
fn every_pair_retains_its_own_reading_position() {
    use crate::conflict::pair::Pair;
    let (_f, _home, mut s) = setup();
    let tokens = [
        "base-left",
        "base-right",
        "base-result",
        "left-right",
        "left-result",
        "result-right",
    ];
    for (n, token) in tokens.iter().enumerate() {
        command(&mut s, &format!("repo diff conflict {token}"));
        activate(&mut s, Focus::Compare);
        scroll(&mut s, n + 2);
        render(&mut s, 210, 24);
    }
    for (n, token) in tokens.iter().enumerate() {
        command(&mut s, &format!("repo diff conflict {token}"));
        render(&mut s, 210, 24);
        assert_eq!(review(&s).mode, DisplayMode::Pair(Pair::ALL[n]));
        let Mode::Changes(session) = &s.mode else {
            panic!()
        };
        let crate::changes::Preview::Ready { scroll, .. } = &session.preview else {
            panic!()
        };
        assert_eq!(*scroll, n + 2);
    }
    s.shutdown();
}
#[test]
fn unavailable_base_does_not_disable_text_comparison_between_the_other_versions() {
    use crate::conflict::pair::Pair;
    let (f, _home, mut s) = setup();
    let binary = f.temp.path().join("binary-base.dat");
    std::fs::write(&binary, b"binary base\0").unwrap();
    let oid = support::git(&f.repo, &["hash-object", "-w", binary.to_str().unwrap()]);
    let mut child = std::process::Command::new("/usr/bin/git")
        .arg("-C")
        .arg(&f.repo)
        .args(["update-index", "--index-info"])
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    writeln!(
        child.stdin.take().unwrap(),
        "100644 {oid} 1\tsrc/中文 空格.ts"
    )
    .unwrap();
    assert!(child.wait().unwrap().success());
    command(&mut s, "refresh");
    wait(&mut s);
    assert_eq!(
        review(&s).pairs.reason(Pair::BaseCurrent),
        Some(i18n::tr!(ConflictNonText))
    );
    assert_eq!(review(&s).pairs.reason(Pair::CurrentIncoming), None);
    command(&mut s, "repo diff conflict left-right");
    assert_eq!(review(&s).mode, DisplayMode::Pair(Pair::CurrentIncoming));
    let text = render(&mut s, 210, 55);
    assert!(
        text.contains("CURRENT") && text.contains("INCOMING"),
        "{text}"
    );
    s.shutdown();
}

fn resolution_setup() -> (support::Fixture, tempfile::TempDir, Screen, String) {
    let (f, home, mut s) = setup();
    let text = "prefix\n<<<<<<< HEAD\nleft one\n=======\nright one\n>>>>>>> incoming\nunchanged\n<<<<<<< HEAD\nleft two\n=======\nright two\n>>>>>>> incoming\nsuffix\n".to_owned();
    support::write(f.repo.join("src/中文 空格.ts"), &text);
    command(&mut s, "refresh");
    wait(&mut s);
    (f, home, s, text)
}
#[test]
fn accepting_a_block_uses_parameter_completion_and_advances_without_staging_or_leaving_the_view() {
    let (f, _home, mut s, original) = resolution_setup();
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    let depth = s.history.len();
    let before = render(&mut s, 180, 44);
    assert!(
        before.contains(&i18n::tf!(ConflictBlockCount, current = 1, total = 2)),
        "{before}"
    );
    command(&mut s, "repo diff conflict accept");
    assert_eq!(super::options::choices(&s).options.len(), 3);
    key(&mut s, KeyCode::Tab);
    assert!(s.bar.text().ends_with("accept left"));
    assert_eq!(
        std::fs::read_to_string(f.repo.join("src/中文 空格.ts")).unwrap(),
        original
    );
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    let expected = original.replacen(
        "<<<<<<< HEAD\nleft one\n=======\nright one\n>>>>>>> incoming\n",
        "left one\n",
        1,
    );
    assert_eq!(
        std::fs::read_to_string(f.repo.join("src/中文 空格.ts")).unwrap(),
        expected
    );
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert_eq!(s.history.len(), depth);
    let text = render(&mut s, 180, 44);
    assert!(
        text.contains(&i18n::tf!(ConflictBlockCount, current = 1, total = 1)),
        "{text}"
    );
    let journal = s.service.store.operation_history(&f.repo, 10).unwrap();
    assert!(journal.iter().any(|record| matches!(&record.record.operation,
        engine::state::Operation::Workspace { action: engine::operations::mutation::Action::ResolveConflict { file, intent: engine::local::files::conflict::resolve::Intent::Accept { choice: engine::local::files::conflict::resolve::Choice::Left } }, .. }
            if file == std::path::Path::new("src/中文 空格.ts"))));
    assert!(review(&s).resolution.undo().is_some());
    command(&mut s, "repo diff conflict accept right");
    wait(&mut s);
    let text = render(&mut s, 180, 44);
    assert!(
        text.contains(i18n::tr!(ConflictResolvedPendingStage)),
        "{text}"
    );
    assert_eq!(
        support::git(&f.repo, &["ls-files", "-u"]).lines().count(),
        3
    );
    command(&mut s, "repo diff stage");
    wait(&mut s);
    assert_eq!(support::git(&f.repo, &["ls-files", "-u"]), "");
    s.shutdown();
}
#[test]
fn block_navigation_highlights_the_selected_region_and_preserves_all_source_bytes() {
    let (f, _home, mut s, original) = resolution_setup();
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    render(&mut s, 180, 44);
    let depth = s.history.len();
    assert!(review(&s).selected_line(3, 1));
    command(&mut s, "repo diff conflict block next");
    assert!(!review(&s).selected_line(3, 1));
    assert!(review(&s).selected_line(3, 7));
    for mode in [
        "overview",
        "include-base",
        "base-left",
        "base-right",
        "base-result",
        "left-right",
        "left-result",
        "result-right",
    ] {
        command(&mut s, &format!("repo diff conflict {mode}"));
        command(&mut s, "repo diff conflict block previous");
        assert!(review(&s).resolution.label().contains("1/2"));
        let text = render(&mut s, 180, 44);
        assert!(
            text.contains(&i18n::tf!(ConflictBlockCount, current = 1, total = 2)),
            "{text}"
        );
        command(&mut s, "repo diff conflict block next");
        assert!(review(&s).resolution.label().contains("2/2"));
    }
    assert_eq!(s.history.len(), depth);
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert_eq!(
        std::fs::read_to_string(f.repo.join("src/中文 空格.ts")).unwrap(),
        original
    );
    s.shutdown();
}
#[test]
fn undo_restores_the_last_selected_block_and_both_keeps_left_before_right() {
    let (f, _home, mut s, original) = resolution_setup();
    command(&mut s, "repo diff conflict block next");
    command(&mut s, "repo diff conflict accept right");
    wait(&mut s);
    command(&mut s, "repo diff conflict undo");
    wait(&mut s);
    assert_eq!(
        std::fs::read_to_string(f.repo.join("src/中文 空格.ts")).unwrap(),
        original
    );
    assert!(review(&s).resolution.label().contains("2/2"));
    command(&mut s, "repo diff conflict accept both");
    wait(&mut s);
    let text = std::fs::read_to_string(f.repo.join("src/中文 空格.ts")).unwrap();
    assert!(text.contains("left two\nright two\n"));
    assert!(text.contains("<<<<<<< HEAD\nleft one"));
    assert!(!text.contains("<<<<<<< HEAD\nleft two"));
    s.shutdown();
}
#[test]
fn stale_accepts_and_undo_cannot_overwrite_external_editor_saves() {
    let (f, _home, mut s, _original) = resolution_setup();
    support::write(f.repo.join("src/中文 空格.ts"), "external editor result\n");
    command(&mut s, "repo diff conflict accept left");
    wait(&mut s);
    assert_eq!(
        std::fs::read_to_string(f.repo.join("src/中文 空格.ts")).unwrap(),
        "external editor result\n"
    );
    command(&mut s, "refresh");
    wait(&mut s);
    assert!(review(&s).resolution.reason().is_some());
    assert!(review(&s).resolution.undo().is_none());
    s.shutdown();
}

#[test]
fn an_open_accept_menu_cannot_apply_to_an_externally_refreshed_block() {
    let (f, _home, mut s, original) = resolution_setup();
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    command(&mut s, "repo diff conflict accept");
    key(&mut s, KeyCode::Tab);
    let updated = original.replace("left one", "external left one");
    support::write(f.repo.join("src/中文 空格.ts"), &updated);
    let files = engine::local::files::read(&f.repo).unwrap();
    let Mode::Changes(session) = &mut s.mode else {
        panic!()
    };
    session.refreshed(files);
    wait(&mut s);
    assert!(s.bar.text().ends_with("accept left"));
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(
        std::fs::read_to_string(f.repo.join("src/中文 空格.ts")).unwrap(),
        updated
    );
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert!(
        s.message.contains(i18n::tr!(ConflictResolveChanged)),
        "{}",
        s.message
    );
    assert!(review(&s).resolution.undo().is_none());
    s.shutdown();
}

#[test]
fn accepting_the_exact_head_source_keeps_the_result_pair_comparisons_and_undo() {
    let (f, _home, mut s) = setup();
    let file = f.repo.join("src/中文 空格.ts");
    let original = std::fs::read(&file).unwrap();
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    command(&mut s, "repo diff conflict accept left");
    wait(&mut s);
    assert_eq!(
        std::fs::read_to_string(&file).unwrap().trim_end(),
        support::git(&f.repo, &["show", "HEAD:src/中文 空格.ts"])
    );
    assert!(review(&s).resolution.undo().is_some());
    assert_eq!(
        review(&s).result.source(),
        std::str::from_utf8(&std::fs::read(&file).unwrap()).ok()
    );
    let text = render(&mut s, 210, 55);
    assert!(
        text.contains(i18n::tr!(ConflictResolvedPendingStage)),
        "{text}"
    );
    command(&mut s, "repo diff conflict left-result");
    assert_eq!(
        review(&s).mode,
        DisplayMode::Pair(crate::conflict::pair::Pair::CurrentResult)
    );
    assert_eq!(
        review(&s)
            .pairs
            .reason(crate::conflict::pair::Pair::CurrentResult),
        None
    );
    command(&mut s, "repo diff conflict undo");
    wait(&mut s);
    assert_eq!(std::fs::read(&file).unwrap(), original);
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert!(review(&s).resolution.label().contains("1/1"));
    s.shutdown();
}

#[test]
fn arrows_select_blocks_and_enter_or_space_opens_accept_choices_without_typing_commands() {
    let (f, _home, mut s, original) = resolution_setup();
    let path = f.repo.join("src/中文 空格.ts");
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    render(&mut s, 210, 55);
    key(&mut s, KeyCode::Enter);
    assert!(
        matches!(&s.mode,Mode::Changes(session) if session.focus == crate::changes::Focus::Preview)
    );
    key(&mut s, KeyCode::Down);
    assert!(review(&s).resolution.label().contains("2/2"));
    key(&mut s, KeyCode::Down);
    assert!(review(&s).resolution.label().contains("2/2"));
    key(&mut s, KeyCode::Home);
    assert!(review(&s).resolution.label().contains("1/2"));
    key(&mut s, KeyCode::Up);
    assert!(review(&s).resolution.label().contains("1/2"));
    key(&mut s, KeyCode::End);
    key(&mut s, KeyCode::PageUp);
    assert!(review(&s).resolution.label().contains("2/2"));
    let geometry = (s.body_area, s.input_area);
    assert_eq!(
        s.focused_conflict_control(),
        Some(crate::conflict::controls::Action::Accept(
            engine::local::files::conflict::resolve::Choice::Left
        ))
    );
    assert!(s.bar.is_plain());
    assert!(render(&mut s, 210, 55).contains("2/3"));
    assert_eq!((s.body_area, s.input_area), geometry);
    key(&mut s, KeyCode::Esc);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    assert!(review(&s).resolution.label().contains("2/2"));
    key(&mut s, KeyCode::Char(' '));
    key(&mut s, KeyCode::Right);
    key(&mut s, KeyCode::Tab);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), original);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    let changed = std::fs::read_to_string(&path).unwrap();
    assert!(changed.contains("right two\n") && !changed.contains("left two\n"));
    assert!(changed.contains("<<<<<<< HEAD\nleft one"));
    assert!(review(&s).resolution.label().contains("1/1"));
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(review(&s).resolution.resolved());
    assert_eq!(
        s.focused_conflict_control(),
        Some(crate::conflict::controls::Action::Stage)
    );
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(support::git(&f.repo, &["ls-files", "-u"]).is_empty());
    s.shutdown();
}

#[test]
fn conflict_clicks_select_the_rendered_block_and_respect_precise_click_preferences() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let (f, _home, mut s, original) = resolution_setup();
    let click = |point: ratatui::layout::Position| MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: point.x,
        row: point.y,
        modifiers: KeyModifiers::NONE,
    };
    let locate = |s: &Screen, index| {
        (0..55).find_map(|y|(0..210).find_map(|x| {
            let point=ratatui::layout::Position::new(x,y);
            matches!(s.pointer.hit(point),Some(crate::pointer::Hit::Button(crate::pointer::Action::ConflictBlock {index:selected,focus:Focus::Result})) if selected==index).then_some(point)
        })).expect("visible result block hit")
    };
    render(&mut s, 210, 55);
    let point = locate(&s, 1);
    s.config.mouse.precise_click = false;
    s.mouse(click(point), &Cancel::default()).unwrap();
    assert!(review(&s).resolution.label().contains("1/2"));
    s.config.mouse.precise_click = true;
    render(&mut s, 210, 55);
    s.mouse(click(locate(&s, 1)), &Cancel::default()).unwrap();
    assert!(review(&s).resolution.label().contains("2/2"));
    assert_eq!(review(&s).focus(), Focus::Result);
    assert_eq!(
        std::fs::read_to_string(f.repo.join("src/中文 空格.ts")).unwrap(),
        original
    );
    key(&mut s, KeyCode::Enter);
    assert!(s.focused_conflict_control().is_some());
    assert!(s.bar.is_plain());
    s.shutdown();
}

#[test]
fn resolved_preview_stages_only_its_current_file_even_when_other_files_are_marked() {
    let (f, _home, mut s) = setup();
    support::write(f.repo.join("other.txt"), "untracked\n");
    command(&mut s, "refresh");
    wait(&mut s);
    let Mode::Changes(session) = &mut s.mode else {
        panic!()
    };
    let other = session
        .files
        .snapshot
        .changes
        .iter()
        .position(|change| change.location.name() == std::path::Path::new("other.txt"))
        .unwrap();
    session.files.mark_file(other);
    key(&mut s, KeyCode::Enter);
    key(&mut s, KeyCode::Right);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(review(&s).resolution.resolved());
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(
        support::git(
            &f.repo,
            &[
                "-c",
                "core.quotepath=false",
                "diff",
                "--cached",
                "--name-only"
            ]
        ),
        "src/中文 空格.ts"
    );
    assert_eq!(
        support::git(&f.repo, &["ls-files", "--others", "--exclude-standard"]),
        "other.txt"
    );
    s.shutdown();
}

#[test]
fn staging_a_resolved_preview_rejects_unreviewed_external_changes() {
    let (f, _home, mut s) = setup();
    key(&mut s, KeyCode::Enter);
    key(&mut s, KeyCode::Right);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(review(&s).resolution.resolved());
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    support::write(f.repo.join("src/中文 空格.ts"), "external editor result\n");
    key(&mut s, KeyCode::Char(' '));
    wait(&mut s);
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert_eq!(
        std::fs::read_to_string(f.repo.join("src/中文 空格.ts")).unwrap(),
        "external editor result\n"
    );
    s.shutdown();
}

#[test]
fn double_clicking_an_input_block_opens_the_bottom_accept_menu_without_writing() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    let (f, _home, mut s) = setup();
    let before = std::fs::read(f.repo.join("src/中文 空格.ts")).unwrap();
    render(&mut s, 210, 55);
    let point = (0..55)
        .find_map(|y| {
            (0..210).find_map(|x| {
                let point = ratatui::layout::Position::new(x, y);
                matches!(
                    s.pointer.hit(point),
                    Some(crate::pointer::Hit::Button(
                        crate::pointer::Action::ConflictBlock {
                            index: 0,
                            focus: Focus::Compare
                        }
                    ))
                )
                .then_some(point)
            })
        })
        .expect("input-side block hit");
    let click = MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: point.x,
        row: point.y,
        modifiers: KeyModifiers::NONE,
    };
    s.mouse(click, &Cancel::default()).unwrap();
    assert!(s.bar.is_plain());
    s.mouse(click, &Cancel::default()).unwrap();
    assert!(s.focused_conflict_control().is_some());
    assert_eq!(
        std::fs::read(f.repo.join("src/中文 空格.ts")).unwrap(),
        before
    );
    key(&mut s, KeyCode::Esc);
    assert!(s.bar.is_plain());
    s.shutdown();
}

#[test]
fn accepting_after_filtering_files_keeps_the_search_draft_and_block_selection() {
    let (_f, _home, mut s, _original) = resolution_setup();
    s.paste("中文");
    key(&mut s, KeyCode::Enter);
    assert!(
        matches!(&s.mode,Mode::Changes(session) if session.focus==crate::changes::Focus::Preview)
    );
    key(&mut s, KeyCode::End);
    assert!(review(&s).resolution.label().contains("2/2"));
    assert!(s.focused_conflict_control().is_some());
    key(&mut s, KeyCode::Esc);
    assert!(s.bar.is_plain());
    assert!(
        matches!(&s.mode,Mode::Changes(session) if session.focus==crate::changes::Focus::Preview && session.files.query=="中文")
    );
    assert!(review(&s).resolution.label().contains("2/2"));
    key(&mut s, KeyCode::Esc);
    assert!(
        matches!(&s.mode,Mode::Changes(session) if session.focus==crate::changes::Focus::Files)
    );
    assert_eq!(s.bar.text(), "中文");
    assert!(review(&s).resolution.label().contains("2/2"));
    s.shutdown();
}

#[test]
fn resetting_a_resolved_file_requires_confirmation_and_returns_to_the_same_view() {
    let (f, _home, mut s) = setup();
    let path = f.repo.join("src/中文 空格.ts");
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    command(&mut s, "repo diff conflict accept left");
    wait(&mut s);
    let resolved = std::fs::read_to_string(&path).unwrap();
    assert!(review(&s).resolution.resolved());
    // 重新进入文件后仍从索引重建，不依赖最近一次采用的撤销记录。
    command(&mut s, "repo diff");
    wait(&mut s);
    let depth = s.history.len();
    command(&mut s, "repo diff conflict reset");
    wait(&mut s);
    assert!(matches!(
        &s.mode,
        Mode::Confirm {
            operation: Confirmation::ConflictReset { .. },
            choice: confirmation::Choice::Return
        }
    ));
    assert!(render(&mut s, 180, 45).contains(i18n::tr!(ConflictResetConfirm)));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), resolved);
    key(&mut s, KeyCode::Enter);
    assert!(matches!(s.mode, Mode::Changes(_)));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), resolved);
    command(&mut s, "repo diff conflict reset");
    wait(&mut s);
    key(&mut s, KeyCode::Left);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(
        text.contains("<<<<<<< HEAD\n") && text.contains("CURRENT") && text.contains("INCOMING")
    );
    assert!(review(&s).resolution.label().contains("1/1"));
    assert_eq!(s.history.len(), depth);
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert!(
        s.service
            .store
            .operation_history(&f.repo, 10)
            .unwrap()
            .iter()
            .any(|record| matches!(
                record.record.operation,
                engine::state::Operation::Workspace {
                    action: engine::operations::mutation::Action::ResolveConflict {
                        intent: engine::local::files::conflict::resolve::Intent::Reset,
                        ..
                    },
                    ..
                }
            ))
    );
    command(&mut s, "repo diff conflict undo");
    wait(&mut s);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), resolved);
    assert!(review(&s).resolution.resolved());
    s.shutdown();
}
#[test]
fn reset_confirmation_refuses_external_changes_and_escape_preserves_the_original_selection() {
    let (f, _home, mut s, original) = resolution_setup();
    command(&mut s, "repo diff conflict block next");
    command(&mut s, "repo diff conflict reset");
    wait(&mut s);
    key(&mut s, KeyCode::Esc);
    assert!(review(&s).resolution.label().contains("2/2"));
    assert_eq!(
        std::fs::read_to_string(f.repo.join("src/中文 空格.ts")).unwrap(),
        original
    );
    command(&mut s, "repo diff conflict reset");
    wait(&mut s);
    let path = f.repo.join("src/中文 空格.ts");
    std::fs::write(&path, "external content\n").unwrap();
    key(&mut s, KeyCode::Left);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "external content\n"
    );
    assert_eq!(
        support::git(&f.repo, &["ls-files", "-u"]).lines().count(),
        3
    );
    s.shutdown();
}

#[test]
fn shift_tab_cycles_exactly_three_inline_pages_and_restores_the_command_draft() {
    use crate::{conflict::controls::Action as Choice, input_pages::Page};
    let (f, _home, mut s, original) = resolution_setup();
    render(&mut s, 140, 45);
    let geometry = (s.body_area, s.input_area);
    s.paste("//repo diff lay");
    key(&mut s, KeyCode::Left);
    let draft = (s.bar.text(), s.bar.editor().cursor);
    assert_eq!(s.input_page(), Page::Input);
    assert!(render(&mut s, 140, 45).contains("1/3"));
    key(&mut s, KeyCode::BackTab);
    assert_eq!(s.input_page(), Page::Conflict);
    assert!(s.bar.is_plain());
    assert_eq!(
        s.focused_conflict_control(),
        Some(Choice::Accept(
            engine::local::files::conflict::resolve::Choice::Left
        ))
    );
    let view = render(&mut s, 140, 45);
    assert!(view.contains("2/3") && view.contains(i18n::tr!(ConflictChoiceRight)));
    assert_eq!((s.body_area, s.input_area), geometry);
    key(&mut s, KeyCode::Tab);
    assert_eq!(s.input_page(), Page::Conflict);
    key(&mut s, KeyCode::Down);
    assert!(review(&s).resolution.label().contains("2/2"));
    key(&mut s, KeyCode::BackTab);
    assert_eq!(s.input_page(), Page::Replay);
    assert!(render(&mut s, 140, 45).contains("3/3"));
    assert_eq!((s.body_area, s.input_area), geometry);
    key(&mut s, KeyCode::BackTab);
    assert_eq!(s.input_page(), Page::Input);
    assert_eq!((s.bar.text(), s.bar.editor().cursor), draft);
    assert_eq!(
        std::fs::read_to_string(f.repo.join("src/中文 空格.ts")).unwrap(),
        original
    );
    key(&mut s, KeyCode::End);
    key(&mut s, KeyCode::Tab);
    assert!(s.bar.text().contains("layout"));
    s.shutdown();
}
#[test]
fn conflict_toolbar_can_resolve_reset_and_undo_without_typing_any_commands() {
    use crate::conflict::controls::Action as Choice;
    let (f, _home, mut s) = setup();
    let path = f.repo.join("src/中文 空格.ts");
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    key(&mut s, KeyCode::BackTab);
    key(&mut s, KeyCode::Right);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(review(&s).resolution.resolved());
    assert_eq!(s.focused_conflict_control(), Some(Choice::Stage));
    let resolved = std::fs::read_to_string(&path).unwrap();
    key(&mut s, KeyCode::Left);
    assert_eq!(s.focused_conflict_control(), Some(Choice::Reset));
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(matches!(
        s.mode,
        Mode::Confirm {
            operation: Confirmation::ConflictReset { .. },
            ..
        }
    ));
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.focused_conflict_control(), Some(Choice::Reset));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), resolved);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    key(&mut s, KeyCode::Left);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(review(&s).resolution.label().contains("1/1"));
    assert_eq!(s.input_page(), crate::input_pages::Page::Conflict);
    key(&mut s, KeyCode::Left);
    assert_eq!(s.focused_conflict_control(), Some(Choice::Undo));
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(std::fs::read_to_string(&path).unwrap(), resolved);
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    s.shutdown();
}

#[test]
fn contextual_defaults_open_conflict_actions_and_advance_to_recovery_after_staging() {
    use crate::{conflict::controls::Action as Choice, input_pages::Page};
    let (f, _home, mut s) = automatic_setup();
    assert_eq!(s.input_page(), Page::Conflict);
    assert!(
        matches!(&s.mode,Mode::Changes(session) if session.focus==crate::changes::Focus::Preview)
    );
    assert!(render(&mut s, 140, 45).contains("2/3"));
    key(&mut s, KeyCode::Right);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(s.focused_conflict_control(), Some(Choice::Stage));
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(support::git(&f.repo, &["ls-files", "-u"]).is_empty());
    assert_eq!(s.input_page(), Page::Replay);
    assert_eq!(
        s.focused_replay_control(),
        Some(engine::operations::replay::Control::Continue)
    );
    assert!(render(&mut s, 140, 45).contains("2/2"));
    s.shutdown();
}
#[test]
fn manual_input_page_and_command_drafts_are_not_replaced_by_passive_refresh() {
    use crate::input_pages::Page;
    let (_f, _home, mut s) = automatic_setup();
    key(&mut s, KeyCode::BackTab);
    assert_eq!(s.input_page(), Page::Replay);
    key(&mut s, KeyCode::BackTab);
    assert_eq!(s.input_page(), Page::Input);
    s.normalize_input_page();
    assert_eq!(s.input_page(), Page::Input);
    command(&mut s, "refresh");
    wait(&mut s);
    assert_eq!(s.input_page(), Page::Input);
    s.paste("//repo diff lay");
    key(&mut s, KeyCode::Left);
    let draft = (s.bar.text(), s.bar.editor().cursor);
    s.poll_changes();
    s.normalize_input_page();
    assert_eq!((s.bar.text(), s.bar.editor().cursor), draft);
    assert_eq!(s.input_page(), Page::Input);
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Esc);
    assert!(
        matches!(&s.mode,Mode::Changes(session) if session.focus==crate::changes::Focus::Files)
    );
    assert_eq!(s.input_page(), Page::Input);
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.input_page(), Page::Conflict);
    s.shutdown();
}

#[test]
fn keyboard_resolution_finishes_multiple_files_and_continues_the_original_pick() {
    use crate::{conflict::controls::Action as Control, input_pages::Page};
    use engine::local::files::conflict::resolve::Choice;
    let f = support::Fixture::create();
    let gap = (0..12)
        .map(|n| format!("export const gap_{n} = {n};\n"))
        .collect::<String>();
    let base = format!("export const first = \"base\";\n{gap}export const last = \"base\";\n");
    f.commit("a.ts", &base);
    f.commit("b.ts", "export const other = \"base\";\n");
    support::git(&f.repo, &["checkout", "-b", "incoming"]);
    support::write(
        f.repo.join("a.ts"),
        &base.replace("\"base\"", "\"incoming\""),
    );
    support::write(f.repo.join("b.ts"), "export const other = \"incoming\";\n");
    support::git(&f.repo, &["add", "."]);
    support::git(&f.repo, &["commit", "-m", "resolve several files"]);
    let incoming = support::git(&f.repo, &["rev-parse", "HEAD"]);
    support::git(&f.repo, &["checkout", "main"]);
    support::write(
        f.repo.join("a.ts"),
        &base.replace("\"base\"", "\"current\""),
    );
    support::write(f.repo.join("b.ts"), "export const other = \"current\";\n");
    support::git(&f.repo, &["add", "."]);
    support::git(&f.repo, &["commit", "-m", "current side"]);
    let head = support::git(&f.repo, &["rev-parse", "HEAD"]);
    let picked = std::process::Command::new("/usr/bin/git")
        .arg("-C")
        .arg(&f.repo)
        .args(["cherry-pick", &incoming])
        .output()
        .unwrap();
    assert_eq!(picked.status.code(), Some(1));
    support::write(f.repo.join("unrelated.txt"), "keep this untracked\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    let choose = |s: &mut Screen, target: Control| {
        for _ in 0..Control::ALL.len() {
            if s.focused_conflict_control() == Some(target) {
                break;
            }
            key(s, KeyCode::Right);
        }
        assert_eq!(s.focused_conflict_control(), Some(target));
        key(s, KeyCode::Enter);
        wait(s);
    };
    assert_eq!(s.input_page(), Page::Conflict);
    assert!(review(&s).resolution.label().contains("1/2"));
    choose(&mut s, Control::Accept(Choice::Right));
    assert!(review(&s).resolution.label().contains("1/1"));
    choose(&mut s, Control::Accept(Choice::Both));
    assert!(review(&s).resolution.resolved());
    choose(&mut s, Control::Stage);
    assert_eq!(s.input_page(), Page::Conflict);
    assert!(review(&s).resolution.label().contains("1/1"));
    choose(&mut s, Control::Accept(Choice::Left));
    choose(&mut s, Control::Stage);
    assert!(support::git(&f.repo, &["ls-files", "-u"]).is_empty());
    assert_eq!(s.input_page(), Page::Replay);
    assert_eq!(
        s.focused_replay_control(),
        Some(engine::operations::replay::Control::Continue)
    );
    assert!(render(&mut s, 140, 40).contains("2/2"));
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(matches!(s.mode, Mode::Confirm { .. }));
    key(&mut s, KeyCode::Left);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(!f.repo.join(".git/CHERRY_PICK_HEAD").exists());
    assert_ne!(support::git(&f.repo, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        support::git(&f.repo, &["log", "-1", "--format=%s"]),
        "resolve several files"
    );
    assert_eq!(
        support::git(&f.repo, &["status", "--porcelain"]),
        "?? unrelated.txt"
    );
    assert_eq!(
        std::fs::read_to_string(f.repo.join("b.ts")).unwrap(),
        "export const other = \"current\";\n"
    );
    let result = std::fs::read_to_string(f.repo.join("a.ts")).unwrap();
    assert!(result.contains(&gap));
    assert!(result.starts_with("export const first = \"incoming\";\n"));
    assert!(
        result.ends_with("export const last = \"current\";\nexport const last = \"incoming\";\n")
    );
    s.shutdown();
}

#[test]
fn automatic_conflict_page_retains_the_reset_action_after_confirmation() {
    use crate::conflict::controls::Action as Control;
    let (f, _home, mut s) = automatic_setup();
    key(&mut s, KeyCode::Right);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(s.focused_conflict_control(), Some(Control::Stage));
    key(&mut s, KeyCode::Left);
    assert_eq!(s.focused_conflict_control(), Some(Control::Reset));
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    key(&mut s, KeyCode::Left);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(s.focused_conflict_control(), Some(Control::Reset));
    assert!(!review(&s).resolution.resolved());
    for _ in 0..3 {
        key(&mut s, KeyCode::Left);
    }
    assert_eq!(
        s.focused_conflict_control(),
        Some(Control::Accept(
            engine::local::files::conflict::resolve::Choice::Right
        ))
    );
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(review(&s).resolution.resolved());
    assert!(
        !std::fs::read_to_string(f.repo.join("src/中文 空格.ts"))
            .unwrap()
            .contains("<<<<<<<")
    );
    s.shutdown();
}

#[test]
fn resolved_conflict_actions_cycle_in_both_directions_without_being_pulled_back_to_stage() {
    use crate::conflict::controls::Action as Control;
    use engine::local::files::conflict::resolve::Choice;
    let (f, _home, mut s) = automatic_setup();
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(s.focused_conflict_control(), Some(Control::Stage));
    let resolved = std::fs::read(f.repo.join("src/中文 空格.ts")).unwrap();
    for action in [
        Control::Accept(Choice::Left),
        Control::Accept(Choice::Right),
        Control::Accept(Choice::Both),
        Control::Undo,
        Control::Reset,
        Control::Stage,
    ] {
        key(&mut s, KeyCode::Right);
        s.poll_changes();
        render(&mut s, 140, 40);
        assert_eq!(s.focused_conflict_control(), Some(action));
    }
    for action in [
        Control::Reset,
        Control::Undo,
        Control::Accept(Choice::Both),
        Control::Accept(Choice::Right),
        Control::Accept(Choice::Left),
        Control::Stage,
    ] {
        key(&mut s, KeyCode::Left);
        s.poll_changes();
        assert_eq!(s.focused_conflict_control(), Some(action));
    }
    assert_eq!(
        std::fs::read(f.repo.join("src/中文 空格.ts")).unwrap(),
        resolved
    );
    key(&mut s, KeyCode::BackTab);
    key(&mut s, KeyCode::BackTab);
    key(&mut s, KeyCode::BackTab);
    assert_eq!(s.focused_conflict_control(), Some(Control::Stage));
    for _ in 0..4 {
        key(&mut s, KeyCode::Right);
    }
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(!review(&s).resolution.resolved());
    s.shutdown();
}
