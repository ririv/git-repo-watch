use super::*;
use crate::changes::{Focus, Preview, edit};
use engine::local::files::partial::LineId;

fn setup() -> (support::Fixture, tempfile::TempDir, Screen, String, String) {
    let f = support::Fixture::create();
    let before = (1..=25)
        .map(|line| format!("line {line}\n"))
        .collect::<String>();
    f.commit("文件 空格.txt", &before);
    let after = before
        .replace("line 2\n", "first\n")
        .replace("line 23\n", "last\n");
    support::write(f.repo.join("文件 空格.txt"), &after);
    let (home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    (f, home, s, before, after)
}
fn editing(s: &Screen) -> &edit::Session {
    let Mode::Changes(session) = &s.mode else {
        panic!("changes workspace");
    };
    let Preview::Ready {
        editing: edit::State::Ready(editing),
        ..
    } = &session.preview
    else {
        panic!("editable text preview: {:?}", session.preview);
    };
    editing
}

#[test]
fn partial_hunk_staging_and_unstaging_preserve_file_contents_and_focus() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (f, _home, mut s, before, after) = setup();
        assert_eq!(editing(&s).mode(), edit::Granularity::Hunk);
        key(&mut s, KeyCode::Char(' '));
        wait(&mut s);
        assert_eq!(
            support::git(&f.repo, &["show", ":文件 空格.txt"]),
            before.replace("line 2\n", "first\n").trim_end()
        );
        assert_eq!(
            std::fs::read_to_string(f.repo.join("文件 空格.txt")).unwrap(),
            after
        );
        assert!(matches!(&s.mode, Mode::Changes(session) if session.focus == Focus::Preview));
        key(&mut s, KeyCode::Esc);
        key(&mut s, KeyCode::Tab);
        // 区域切换仍通过文件列表，预览中的空格使用当前暂存层。
        if let Mode::Changes(session) = &mut s.mode {
            session.files.switch_group(s.file_layout);
        }
        s.poll_changes();
        wait(&mut s);
        command(&mut s, "repo diff select hunk");
        command(&mut s, "repo diff unstage");
        wait(&mut s);
        assert_eq!(
            support::git(&f.repo, &["show", ":文件 空格.txt"]),
            before.trim_end()
        );
        assert_eq!(
            std::fs::read_to_string(f.repo.join("文件 空格.txt")).unwrap(),
            after
        );
        s.shutdown();
    }
}

#[test]
fn partial_line_mode_and_shift_range_apply_only_selected_source_lines() {
    let (f, _home, mut s, before, after) = setup();
    command(&mut s, "repo diff select line");
    assert_eq!(editing(&s).cursor(), LineId::Old(2.try_into().unwrap()));
    key(&mut s, KeyCode::Down);
    assert_eq!(editing(&s).cursor(), LineId::New(2.try_into().unwrap()));
    key(&mut s, KeyCode::Char(' '));
    wait(&mut s);
    assert_eq!(
        support::git(&f.repo, &["show", ":文件 空格.txt"]),
        before.replace("line 2\n", "line 2\nfirst\n").trim_end()
    );
    assert_eq!(
        std::fs::read_to_string(f.repo.join("文件 空格.txt")).unwrap(),
        after
    );
    command(&mut s, "repo diff select line");
    s.key(
        KeyEvent::new(KeyCode::Down, KeyModifiers::SHIFT),
        &Cancel::default(),
    )
    .unwrap();
    assert_eq!(editing(&s).selected().len(), 2);
    s.key(
        KeyEvent::new(KeyCode::Up, KeyModifiers::SHIFT),
        &Cancel::default(),
    )
    .unwrap();
    assert_eq!(editing(&s).selected().len(), 1);
    s.shutdown();
}

#[test]
fn partial_discard_requires_confirmation_and_preserves_unselected_hunks() {
    let (f, _home, mut s, before, after) = setup();
    command(&mut s, "repo diff discard");
    assert!(matches!(
        s.mode,
        Mode::Confirm {
            choice: confirmation::Choice::Return,
            ..
        }
    ));
    key(&mut s, KeyCode::Enter);
    assert!(matches!(s.mode, Mode::Changes(_)));
    assert_eq!(
        std::fs::read_to_string(f.repo.join("文件 空格.txt")).unwrap(),
        after
    );
    command(&mut s, "repo diff discard");
    key(&mut s, KeyCode::Left);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert_eq!(
        std::fs::read_to_string(f.repo.join("文件 空格.txt")).unwrap(),
        before.replace("line 23\n", "last\n")
    );
    assert_eq!(
        support::git(&f.repo, &["show", ":文件 空格.txt"]),
        before.trim_end()
    );
    s.shutdown();
}

#[test]
fn partial_mouse_selection_uses_the_correct_column_in_both_layouts_and_when_wrapped() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    for layout in [
        crate::diff::Preference::Unified,
        crate::diff::Preference::Split,
    ] {
        let (f, _home, mut s, before, _) = setup();
        command(&mut s, "repo diff select line");
        s.diff_preference = layout;
        s.config.text_wrap = true;
        render(&mut s, 180, 32);
        let id = LineId::New(2.try_into().unwrap());
        let point = (0..32)
            .find_map(|y| {
                (0..180).find_map(|x| {
                    let point = ratatui::layout::Position::new(x, y);
                    (s.pointer.hit(point)
                        == Some(crate::pointer::Hit::Button(
                            crate::pointer::Action::DiffLine(id),
                        )))
                    .then_some(point)
                })
            })
            .unwrap();
        s.mouse(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: point.x,
                row: point.y,
                modifiers: KeyModifiers::NONE,
            },
            &Cancel::default(),
        )
        .unwrap();
        assert_eq!(editing(&s).cursor(), id);
        key(&mut s, KeyCode::Char(' '));
        wait(&mut s);
        assert_eq!(
            support::git(&f.repo, &["show", ":文件 空格.txt"]),
            before.replace("line 2\n", "line 2\nfirst\n").trim_end()
        );
        s.shutdown();
    }
}

#[test]
fn partial_operations_reject_a_file_modified_after_preview() {
    let (f, _home, mut s, before, _) = setup();
    support::write(f.repo.join("文件 空格.txt"), "external contents\n");
    key(&mut s, KeyCode::Char(' '));
    wait(&mut s);
    assert_eq!(
        std::fs::read_to_string(f.repo.join("文件 空格.txt")).unwrap(),
        "external contents\n"
    );
    assert_eq!(
        support::git(&f.repo, &["show", ":文件 空格.txt"]),
        before.trim_end()
    );
    s.shutdown();
}

#[test]
fn partial_commands_stay_out_of_historical_diffs_and_cannot_modify_them() {
    let f = support::Fixture::create();
    f.commit("file.txt", "old\n");
    f.commit("file.txt", "new\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(
        matches!(&s.mode, Mode::History(view) if matches!(view.as_ref(), crate::history::View::Diff {..}))
    );
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    let selection = s.copy_selection();
    let path = s.scope_path();
    for command in command::COMMANDS
        .iter()
        .filter(|command| matches!(command.action, Action::PartialSelectMenu | Action::Discard))
    {
        assert!(!selection.admits(command.action, path.as_deref()));
        assert!(!s.command_context().unwrap().admits(command));
    }
    key(&mut s, KeyCode::Char(' '));
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert_eq!(
        std::fs::read_to_string(f.repo.join("file.txt")).unwrap(),
        "new\n"
    );
    s.shutdown();
}

#[test]
fn partial_binary_preview_never_falls_back_to_whole_file_staging() {
    let f = support::Fixture::create();
    std::fs::write(f.repo.join("binary.bin"), [0, 1, 2]).unwrap();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    for name in [
        "repo diff select line",
        "repo diff stage",
        "repo diff discard",
    ] {
        command(&mut s, name);
        assert!(s.repository_tasks.is_empty());
    }
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert_eq!(std::fs::read(f.repo.join("binary.bin")).unwrap(), [0, 1, 2]);
    s.shutdown();
}

#[test]
fn partial_selection_tracks_wrapped_continuations_across_resizes() {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    for layout in [
        crate::diff::Preference::Unified,
        crate::diff::Preference::Split,
    ] {
        let f = support::Fixture::create();
        let before = format!("{}\n", "long-before ".repeat(40));
        let after = format!("{}\n", "long-after ".repeat(50));
        f.commit("long.txt", &before);
        support::write(f.repo.join("long.txt"), &after);
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        command(&mut s, "repo diff");
        wait(&mut s);
        key(&mut s, KeyCode::Enter);
        command(&mut s, "repo diff select line");
        s.config.text_wrap = true;
        s.diff_preference = layout;
        for width in [180, 90, 180] {
            render(&mut s, width, 40);
        }
        let id = LineId::New(1.try_into().unwrap());
        let mut rows = std::collections::BTreeMap::new();
        for y in 0..40 {
            for x in 0..180 {
                let point = ratatui::layout::Position::new(x, y);
                if s.pointer.hit(point)
                    == Some(crate::pointer::Hit::Button(
                        crate::pointer::Action::DiffLine(id),
                    ))
                {
                    rows.insert(y, point);
                }
            }
        }
        assert!(rows.len() > 1);
        let point = rows.last_key_value().unwrap().1;
        s.mouse(
            MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: point.x,
                row: point.y,
                modifiers: KeyModifiers::NONE,
            },
            &Cancel::default(),
        )
        .unwrap();
        assert_eq!(editing(&s).cursor(), id);
        key(&mut s, KeyCode::Char(' '));
        wait(&mut s);
        assert_eq!(
            support::git(&f.repo, &["show", ":long.txt"]),
            format!("{before}{after}").trim_end_matches('\n')
        );
        assert_eq!(
            std::fs::read_to_string(f.repo.join("long.txt")).unwrap(),
            after
        );
        s.shutdown();
    }
}

#[test]
fn partial_selection_marks_only_the_gutter_and_preserves_diff_colors_and_text() {
    use ratatui::{buffer::Buffer, layout::Position, style::Color};
    fn draw(s: &mut Screen, width: u16) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, 70)).unwrap();
        terminal.draw(|frame| view::render(frame, s)).unwrap();
        terminal.backend().buffer().clone()
    }
    for layout in [
        crate::diff::Preference::Unified,
        crate::diff::Preference::Split,
    ] {
        let f = support::Fixture::create();
        let before = (1..=25)
            .map(|line| format!("line {line}\n"))
            .collect::<String>();
        let before = before.replace(
            "line 2\n",
            &format!("const old = \"{}\";\n", "旧内容 ".repeat(35)),
        );
        let after = before
            .replace("const old", "const new")
            .replace("旧内容", "新内容")
            .replace("line 23\n", "last\n");
        f.commit("source.ts", &before);
        support::write(f.repo.join("source.ts"), &after);
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        command(&mut s, "repo diff");
        wait(&mut s);
        key(&mut s, KeyCode::Enter);
        s.diff_preference = layout;
        for wrap in [false, true] {
            s.config.text_wrap = wrap;
            for mode in ["hunk", "line"] {
                command(&mut s, &format!("repo diff select {mode}"));
                key(&mut s, KeyCode::Home);
                if mode == "line" {
                    key(&mut s, KeyCode::Down);
                }
                for width in [180, 90] {
                    draw(&mut s, width);
                    let Mode::Changes(session) = &mut s.mode else {
                        panic!("changes");
                    };
                    let Preview::Ready { content, .. } = &mut session.preview else {
                        panic!("preview");
                    };
                    let crate::document::View::Text(content) = content.as_mut() else {
                        panic!("text");
                    };
                    content.horizontal.seek(8);
                    let gutter = content.gutter_width(if layout == crate::diff::Preference::Split {
                        crate::diff::Layout::Split
                    } else {
                        crate::diff::Layout::Unified
                    }) as usize;
                    session.focus = Focus::Files;
                    let normal = draw(&mut s, width);
                    let Mode::Changes(session) = &mut s.mode else {
                        panic!("changes");
                    };
                    session.focus = Focus::Preview;
                    let selected = draw(&mut s, width);
                    let selected_ids = editing(&s).selected();
                    let mut rows = std::collections::BTreeMap::<(u16, LineId), Vec<u16>>::new();
                    for y in 0..70 {
                        for x in 0..width {
                            if let Some(crate::pointer::Hit::Button(
                                crate::pointer::Action::DiffLine(id),
                            )) = s.pointer.hit(Position::new(x, y))
                            {
                                rows.entry((y, id)).or_default().push(x);
                            }
                        }
                    }
                    let mut selected_rows = 0;
                    let mut unselected_rows = 0;
                    for ((y, id), columns) in rows {
                        let marked = selected_ids.contains(&id);
                        if marked {
                            selected_rows += 1;
                        } else {
                            unselected_rows += 1;
                        }
                        for (column, x) in columns.into_iter().enumerate() {
                            let cell = &selected[(x, y)];
                            if marked && column < gutter - 1 {
                                assert_eq!(cell.bg, view::SELECTION);
                                assert_eq!(cell.fg, Color::LightBlue);
                                if column == gutter - 2 {
                                    assert_eq!(cell.symbol(), "│");
                                }
                            } else {
                                assert_eq!(
                                    cell,
                                    &normal[(x, y)],
                                    "代码、增删标记与未选行必须保持原样: {layout:?}, wrap={wrap}, {id:?}, ({x},{y})"
                                );
                            }
                        }
                    }
                    assert!(selected_rows > 0 && unselected_rows > 0);
                }
            }
        }
        s.shutdown();
    }
}
