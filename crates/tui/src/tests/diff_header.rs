use super::*;
use crate::diff::{Export, Interaction};

fn text(s: &Screen) -> (&crate::diff::View, usize) {
    let Mode::Changes(session) = &s.mode else {
        panic!("changes expected")
    };
    let crate::changes::Preview::Ready {
        content, scroll, ..
    } = &session.preview
    else {
        panic!("preview expected")
    };
    let crate::document::View::Text(content) = content.as_ref() else {
        panic!("text expected")
    };
    (content, *scroll)
}

#[test]
fn header_command_preserves_export_selection_and_the_reading_position_in_both_layouts() {
    for layout in ["unified", "split"] {
        let f = support::Fixture::create();
        let before = (0..45)
            .map(|n| format!("const line_{n} = 'before';\n"))
            .collect::<String>();
        f.commit("中文 文件.ts", &before);
        support::write(
            f.repo.join("中文 文件.ts"),
            &before.replace("before", "after"),
        );
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        command(&mut s, "repo diff");
        wait(&mut s);
        maximize_preview(&mut s);
        command(&mut s, &format!("repo diff layout {layout}"));
        let initial = render(&mut s, 160, 24);
        assert!(!initial.contains("diff --git") && !initial.contains("index "));
        let Mode::Changes(session) = &mut s.mode else {
            unreachable!()
        };
        let crate::changes::Preview::Ready { scroll, .. } = &mut session.preview else {
            unreachable!()
        };
        *scroll = 15;
        render(&mut s, 160, 24);
        let (content, old_scroll) = text(&s);
        let Export::Complete(export) = &content.export else {
            panic!("complete patch expected")
        };
        let original = export.clone();
        let selection = match &content.interaction {
            Interaction::Editable(value) => value.clone(),
            _ => panic!("editable diff"),
        };
        command(&mut s, "repo diff header show");
        let expanded = render(&mut s, 160, 24);
        assert!(
            expanded.contains("diff --git") && expanded.contains("index "),
            "{expanded}"
        );
        assert!(expanded.contains("--- ") && expanded.contains("+++ "));
        assert_eq!(s.diff_header_label(), i18n::tr!(DiffHeaderExpanded));
        let expanded_scroll = text(&s).1;
        command(&mut s, "repo diff header show");
        render(&mut s, 160, 24);
        assert_eq!(text(&s).1, expanded_scroll);
        command(&mut s, "repo diff header");
        assert_eq!(
            super::options::choices(&s).current,
            Some(command::options::Value::DiffHeader(
                crate::diff::HeaderVisibility::Expanded
            ))
        );
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Tab);
        assert_eq!(s.diff_header, crate::diff::HeaderVisibility::Expanded);
        key(&mut s, KeyCode::Esc);
        key(&mut s, KeyCode::Esc);
        command(&mut s, "repo diff header hide");
        render(&mut s, 160, 24);
        command(&mut s, "repo diff header hide");
        render(&mut s, 160, 24);
        let (content, scroll) = text(&s);
        assert_eq!(scroll, old_scroll);
        assert!(
            matches!(&content.export, Export::Complete(export) if export.as_ref() == original.as_ref())
        );
        let Interaction::Editable(current) = &content.interaction else {
            unreachable!()
        };
        assert_eq!(
            current.as_ref().map(|m| (&m.lines, m.cursor)),
            selection.as_ref().map(|m| (&m.lines, m.cursor))
        );
        assert_eq!(s.diff_header_label(), i18n::tr!(DiffHeaderCollapsed));
        assert!(support::git(&f.repo, &["diff", "--cached"]).is_empty());
    }
}

#[test]
fn unstaged_rename_keeps_a_real_rename_patch_without_changing_the_index() {
    use engine::local::files;
    let f = support::Fixture::create();
    f.commit("原文件.md", "# 尚未暂存的重命名\n\n保留文件内容。\n");
    std::fs::rename(f.repo.join("原文件.md"), f.repo.join("新文件.md")).unwrap();
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    let snapshot = files::read(&f.repo).unwrap();
    let change = snapshot
        .changes
        .iter()
        .find(|change| change.kind == 'R')
        .expect("未暂存重命名");
    let document = files::document(&snapshot, change, &Cancel::default()).unwrap();
    let patch = document.text();
    assert!(
        patch.contains("rename from") && patch.contains("rename to"),
        "{patch}"
    );
    assert!(patch.contains("similarity index 100%"));
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
}

#[cfg(unix)]
#[test]
fn permission_only_changes_remain_visible_without_hunks_or_raw_headers() {
    use std::os::unix::fs::PermissionsExt;
    let _language = i18n::using(i18n::Language::Chinese);
    let f = support::Fixture::create();
    f.commit("script.sh", "#!/bin/sh\nprintf demo\n");
    std::fs::set_permissions(
        f.repo.join("script.sh"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    support::git(&f.repo, &["add", "--", "script.sh"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    maximize_preview(&mut s);
    for size in [(60, 18), (160, 32)] {
        let preview = render(&mut s, size.0, size.1);
        assert!(
            preview.contains("100644") && preview.contains("100755"),
            "{preview}"
        );
        assert!(preview.contains("文件模式") && preview.contains("普通文件，可执行"));
        assert!(!preview.contains("old mode") && !preview.contains("diff --git"));
    }
    let mut terminal = Terminal::new(TestBackend::new(160, 32)).unwrap();
    terminal.draw(|frame| view::render(frame, &mut s)).unwrap();
    let buffer = terminal.backend().buffer();
    let label = buffer
        .content
        .iter()
        .position(|cell| cell.symbol() == "模")
        .unwrap();
    assert_eq!(buffer.content[label].fg, ratatui::style::Color::DarkGray);
    let value = label + unicode_width::UnicodeWidthStr::width("模式: ");
    assert_eq!(buffer.content[value].fg, ratatui::style::Color::LightBlue);
    command(&mut s, "repo diff header show");
    let preview = render(&mut s, 160, 32);
    assert!(preview.contains("old mode 100644") && preview.contains("new mode 100755"));
}

#[test]
fn header_visibility_follows_files_and_survives_restart() {
    let f = support::Fixture::create();
    for path in ["a.rs", "b.rs"] {
        f.commit(path, "fn before() {}\n");
        support::write(f.repo.join(path), "fn after() {}\n");
    }
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let store = s.service.store.clone();
    let scheduler = s.service.scheduler.clone();
    command(&mut s, "repo diff view flat");
    command(&mut s, "repo diff");
    wait(&mut s);
    assert!(!render(&mut s, 160, 32).contains("diff --git"));
    command(&mut s, "repo diff header show");
    assert!(render(&mut s, 160, 32).contains("diff --git a/a.rs"));
    key(&mut s, KeyCode::Down);
    wait(&mut s);
    assert!(render(&mut s, 160, 32).contains("diff --git a/b.rs"));
    command(&mut s, "repo diff header hide");
    assert!(!render(&mut s, 160, 32).contains("diff --git"));
    key(&mut s, KeyCode::Up);
    wait(&mut s);
    assert!(!render(&mut s, 160, 32).contains("diff --git"));
    command(&mut s, "repo diff header show");
    s.shutdown();
    let mut reopened = Screen::new(Service::new(store, scheduler), &Cancel::default()).unwrap();
    command(&mut reopened, "repo diff");
    wait(&mut reopened);
    assert!(render(&mut reopened, 160, 32).contains("diff --git a/a.rs"));
    reopened.shutdown();
}

#[test]
fn ordinary_additions_hide_default_mode_but_keep_it_in_raw_headers_and_export() {
    let _language = i18n::using(i18n::Language::Chinese);
    let f = support::Fixture::create();
    f.commit("base.md", "# base\n");
    support::write(f.repo.join("新增.md"), "# 新增文件内容\n");
    support::git(&f.repo, &["add", "--", "新增.md"]);
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    maximize_preview(&mut s);
    for layout in ["unified", "split"] {
        command(&mut s, &format!("repo diff layout {layout}"));
        let preview = render(&mut s, 160, 32);
        assert!(preview.contains("新增文件"), "{preview}");
        assert!(
            !preview.contains("100644") && !preview.contains("普通文件"),
            "{preview}"
        );
    }
    let (content, _) = text(&s);
    assert!(
        matches!(&content.export, Export::Complete(patch) if patch.contains("new file mode 100644"))
    );
    command(&mut s, "repo diff header show");
    assert!(render(&mut s, 160, 32).contains("new file mode 100644"));
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    s.shutdown();
}

#[test]
fn real_rename_and_copy_headers_keep_similarity_and_only_the_selected_file_patch() {
    use engine::local::history::{self, Kind, Parent};
    let _language = i18n::using(i18n::Language::Chinese);
    let f = support::Fixture::create();
    let body = (0..30)
        .map(|n| format!("第 {n} 行是演练内容。\n"))
        .collect::<String>();
    f.commit("复制来源.md", &body);
    f.commit("重命名前.md", &body.replace("演练", "重命名"));
    support::git(&f.repo, &["config", "diff.renames", "copies"]);
    support::write(f.repo.join("复制目标.md"), &body);
    support::write(f.repo.join("复制来源.md"), &format!("来源继续编辑\n{body}"));
    support::git(&f.repo, &["mv", "--", "重命名前.md", "重命名后.md"]);
    support::write(
        f.repo.join("重命名后.md"),
        &format!("新标题\n{}", body.replace("演练", "重命名")),
    );
    support::git(&f.repo, &["add", "."]);
    support::git(&f.repo, &["commit", "-m", "复制与重命名"]);
    let oid = support::git(&f.repo, &["rev-parse", "HEAD"])
        .parse()
        .unwrap();
    let detail = history::detail(&f.repo, oid, Parent::First, &Cancel::default()).unwrap();
    for (kind, label) in [(Kind::Renamed, "重命名"), (Kind::Copied, "复制")] {
        let change = detail
            .changes
            .iter()
            .find(|change| change.kind == kind)
            .expect("真实重命名／复制");
        let document = history::document(&detail, change, &Cancel::default()).unwrap();
        let engine::local::files::patch::Document::Patch { files, .. } = &document else {
            panic!("patch")
        };
        assert_eq!(files.len(), 1, "复制的来源改动不应混入目标文件预览");
        let view = crate::diff::prepare_history(
            &change.location.display(),
            document,
            detail.base,
            detail.oid,
            &Cancel::default(),
        )
        .unwrap();
        let facts = &view.information.as_ref().unwrap().facts;
        assert!(facts.iter().any(|fact| fact.text().contains(label)));
        assert!(facts.iter().any(|fact| fact.text().contains("相似度")));
    }
    use crate::command::context::{Context, DiffContext};
    assert_eq!(
        Context::Diff(DiffContext::History).matches("header")[0].name,
        "repo diff header"
    );
}
