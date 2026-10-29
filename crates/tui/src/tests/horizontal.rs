use super::*;
use crate::{changes::Preview, diff::Preference, document};
use crossterm::event::{MouseEvent, MouseEventKind};

fn diff(s: &Screen) -> &crate::diff::View {
    let Mode::Changes(session) = &s.mode else {
        panic!("expected changes")
    };
    let document::View::Text(view) = session.content() else {
        panic!("expected text")
    };
    view
}
fn graph(s: &Screen) -> &crate::history::Session {
    let Mode::History(view) = &s.mode else {
        panic!("expected history")
    };
    let crate::history::View::Graph(session) = view.as_ref() else {
        panic!("expected graph")
    };
    session
}
fn pan_end(s: &mut Screen) {
    for _ in 0..150 {
        key(s, KeyCode::Right);
    }
}

#[test]
fn diff_scrolls_long_unicode_lines_with_pinned_gutters_and_synchronized_columns() {
    for preference in [Preference::Unified, Preference::Split] {
        let f = support::Fixture::create();
        let prefix = "中文 e\u{301}👩‍💻 ".repeat(25);
        f.commit("long.rs", &format!("// {prefix}OLDTAIL\n"));
        support::write(f.repo.join("long.rs"), &format!("// {prefix}NEWTAIL\n"));
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        let config = std::fs::read(s.service.store.config_path()).unwrap();
        open_files(&mut s);
        wait(&mut s);
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        maximize_preview(&mut s);
        s.diff_preference = preference;
        let before = render(&mut s, 100, 32);
        assert!(!before.contains("NEWTAIL"));
        let export = match &diff(&s).export {
            crate::diff::Export::Complete(text) => text.clone(),
            _ => panic!("complete patch"),
        };
        pan_end(&mut s);
        let offset = diff(&s).horizontal.offset();
        assert!(offset > 0);
        let after = render(&mut s, 100, 32);
        assert!(after.contains("NEWTAIL"), "{after}");
        assert!(after.contains("OLDTAIL"), "{after}");
        assert!(after.contains("1 │+"), "{after}");
        assert!(after.contains("│-"), "{after}");
        key(&mut s, KeyCode::Right);
        assert_eq!(diff(&s).horizontal.offset(), offset);
        key(&mut s, KeyCode::Left);
        assert_eq!(diff(&s).horizontal.offset(), offset - 4);
        assert!(
            matches!(&diff(&s).export, crate::diff::Export::Complete(text) if **text == *export)
        );
        assert!(export.contains(&prefix));
        render(&mut s, 700, 32);
        assert_eq!(diff(&s).horizontal.offset(), 0);
        assert!(!diff(&s).horizontal.overflowing());
        render(&mut s, 100, 32);
        key(&mut s, KeyCode::Left);
        assert_eq!(diff(&s).horizontal.offset(), 0);
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        assert_eq!(
            std::fs::read(s.service.store.config_path()).unwrap(),
            config
        );
        key(&mut s, KeyCode::Esc);
        assert!(matches!(s.mode, Mode::Changes(_)));
        s.shutdown();
    }
}

#[test]
fn mouse_and_editor_are_scoped_and_new_files_reset_horizontal_position() {
    let f = support::Fixture::create();
    support::write(
        f.repo.join("a.rs"),
        &format!("// {}TAIL\n", "x".repeat(300)),
    );
    support::write(f.repo.join("b.rs"), "fn short() {}\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    maximize_preview(&mut s);
    render(&mut s, 100, 32);
    key(&mut s, KeyCode::Right);
    assert_eq!(diff(&s).horizontal.offset(), 4);
    let body = s.body_area.unwrap();
    for (kind, modifiers, expected) in [
        (MouseEventKind::ScrollRight, KeyModifiers::NONE, 8),
        (MouseEventKind::ScrollDown, KeyModifiers::SHIFT, 12),
        (MouseEventKind::ScrollLeft, KeyModifiers::NONE, 8),
    ] {
        s.mouse(
            MouseEvent {
                kind,
                modifiers,
                column: body.x + 5,
                row: body.y + 5,
            },
            &Cancel::default(),
        )
        .unwrap();
        assert_eq!(diff(&s).horizontal.offset(), expected);
    }
    s.paste("a");
    wait(&mut s);
    render(&mut s, 100, 32);
    assert!(matches!(s.bar, crate::bar::InputBar::CommandsOnly));
    let previous = diff(&s).horizontal.offset();
    key(&mut s, KeyCode::Left);
    let offset = previous.saturating_sub(4);
    assert_eq!(diff(&s).horizontal.offset(), offset);
    key(&mut s, KeyCode::Esc);
    command(&mut s, "repo diff layout");
    key(&mut s, KeyCode::Right);
    assert!(matches!(
        s.bar,
        crate::bar::InputBar::Layout {
            selected: Preference::Split,
            ..
        }
    ));
    assert_eq!(diff(&s).horizontal.offset(), offset);
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Esc);
    // 预览不接收普通文字；回到文件列表后换文件，新预览具有独立滚动位置。
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Down);
    wait(&mut s);
    assert_eq!(diff(&s).horizontal.offset(), 0);
    assert!(
        matches!(&s.mode, Mode::Changes(session) if matches!(session.preview, Preview::Ready {..}))
    );
    s.shutdown();
}

#[test]
fn history_text_scroll_keeps_topology_hash_selection_and_editor_independent() {
    let f = support::Fixture::create();
    let subject = format!("{}HISTORYTAIL", "中文内容".repeat(40));
    support::git(&f.repo, &["commit", "--allow-empty", "-m", &subject]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    render(&mut s, 100, 32);
    let oid = graph(&s).selected().unwrap().oid;
    let count = graph(&s).records.len();
    pan_end(&mut s);
    let text = render(&mut s, 100, 32);
    assert!(text.contains("HISTORYTAIL"), "{text}");
    assert!(text.contains(&oid.to_string()[..8]));
    assert_eq!(graph(&s).selected().unwrap().oid, oid);
    assert_eq!(graph(&s).records.len(), count);
    let offset = graph(&s).horizontal.offset();
    s.paste("中文");
    key(&mut s, KeyCode::Left);
    assert_eq!(graph(&s).horizontal.offset(), offset);
    assert_eq!(s.bar.editor().cursor, "中".len());
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(
        matches!(&s.mode, Mode::History(view) if matches!(view.as_ref(), crate::history::View::Commit(_)))
    );
    key(&mut s, KeyCode::Esc);
    assert_eq!(graph(&s).horizontal.offset(), offset);
    render(&mut s, 800, 32);
    assert_eq!(graph(&s).horizontal.offset(), 0);
    key(&mut s, KeyCode::Esc);
    assert!(matches!(s.mode, Mode::Browsing));
    s.shutdown();
}

fn wide_history() -> support::Fixture {
    let fixture = support::Fixture::create();
    let tree = support::git(&fixture.repo, &["rev-parse", "HEAD^{tree}"]);
    let parent = support::git(&fixture.repo, &["rev-parse", "HEAD"]);
    // 多父提交的合并明确保留并行轨道，标题保持简短以单独验证图线溢出。
    let mut parents = Vec::new();
    for index in 0..20 {
        let oid = support::git(
            &fixture.repo,
            &[
                "commit-tree",
                &tree,
                "-p",
                &parent,
                "-m",
                &format!("node-{index:02}"),
            ],
        );
        support::git(
            &fixture.repo,
            &["update-ref", &format!("refs/heads/b{index:02}"), &oid],
        );
        parents.push(oid);
    }
    let mut args = vec!["commit-tree", &tree];
    for parent in &parents {
        args.extend(["-p", parent.as_str()]);
    }
    args.extend(["-m", "wide merge"]);
    let merge = support::git(&fixture.repo, &args);
    support::git(&fixture.repo, &["update-ref", "refs/heads/main", &merge]);
    fixture
}

#[test]
fn history_arrows_scroll_wide_graphs_without_hiding_short_text_and_share_the_scrollbar() {
    let f = wide_history();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    let before = render(&mut s, 90, 40);
    let oid = graph(&s).selected().unwrap().oid;
    let subject = graph(&s).selected().unwrap().subject.clone();
    let count = graph(&s).records.len();
    assert!(graph(&s).width >= 39);
    assert!(before.contains(&subject), "{before}");
    key(&mut s, KeyCode::Right);
    let after = render(&mut s, 90, 40);
    assert_eq!(graph(&s).horizontal.offset(), 4);
    assert!(after.contains(&subject), "{after}");
    pan_end(&mut s);
    let after = render(&mut s, 90, 40);
    let offset = graph(&s).horizontal.offset();
    assert!(offset >= 4);
    assert!(after.contains(&subject), "{after}");
    assert!(after.contains(&oid.to_string()[..8]), "{after}");
    assert_eq!(graph(&s).records.len(), count);
    assert_eq!(graph(&s).selected().unwrap().oid, oid);
    key(&mut s, KeyCode::Right);
    assert_eq!(graph(&s).horizontal.offset(), offset);
    // 图线单独溢出时，滚动条位于图的下方，可直接点击回到起点。
    let area = s.body_area.unwrap();
    let hit = s
        .pointer
        .hit(ratatui::layout::Position::new(
            area.x + 2,
            area.bottom() - 1,
        ))
        .unwrap();
    assert!(matches!(
        hit,
        crate::pointer::Hit::Scrollbar(crate::pointer::Action::Scroll(
            crate::pointer::ScrollTarget::HistoryHorizontal,
            _
        ))
    ));
    s.mouse(
        MouseEvent {
            kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
            modifiers: KeyModifiers::NONE,
            column: area.x + 2,
            row: area.bottom() - 1,
        },
        &Cancel::default(),
    )
    .unwrap();
    assert_eq!(graph(&s).horizontal.offset(), 0);
    key(&mut s, KeyCode::Right);
    render(&mut s, 90, 40);
    assert!(graph(&s).horizontal.offset() > 0);
    render(&mut s, 800, 40);
    assert_eq!(graph(&s).horizontal.offset(), 0);
    assert!(!graph(&s).horizontal.overflowing());
    s.shutdown();
}

#[test]
fn unified_history_scroll_reaches_long_text_and_ignores_graph_overflow_without_pixels() {
    let f = wide_history();
    let long_subject = format!("{}LONGTAIL", "中文".repeat(80));
    support::git(&f.repo, &["commit", "--allow-empty", "-m", &long_subject]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    render(&mut s, 90, 40);
    pan_end(&mut s);
    let text = render(&mut s, 90, 40);
    assert!(text.contains("LONGTAIL"), "{text}");
    assert!(graph(&s).horizontal.offset() > graph(&s).width);
    assert!(graph(&s).graph_offset(29) < graph(&s).horizontal.offset());
    s.shutdown();

    let f = wide_history();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    s.graphics = crate::graphics::Display::new(crate::graphics::Support::Unavailable(
        "no image protocol".into(),
    ));
    command(&mut s, "repo log");
    wait(&mut s);
    render(&mut s, 90, 40);
    key(&mut s, KeyCode::Right);
    assert_eq!(graph(&s).horizontal.offset(), 0);
    assert!(!graph(&s).horizontal.overflowing());
    s.shutdown();
}
