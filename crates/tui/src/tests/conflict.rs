use super::*;
use crate::{
    conflict::{Body, DisplayMode, Focus},
    document::View,
};

fn setup() -> (support::Fixture, tempfile::TempDir, Screen) {
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
    assert!(
        command::COMMANDS
            .iter()
            .all(|cmd| !cmd.name.starts_with("repo diff conflict "))
    );
    command(&mut s, "repo diff conflict include-base");
    let text = render(&mut s, 210, 55);
    assert!(!text.contains(i18n::tr!(ConflictCompareCurrent)), "{text}");
    assert!(!text.contains(i18n::tr!(ConflictCompareResult)), "{text}");
    command(&mut s, "repo diff conflict base-left");
    activate(&mut s, Focus::Compare);
    key(&mut s, KeyCode::Esc);
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
