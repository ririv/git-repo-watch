use engine::{
    cancel::Cancel,
    local::files::{
        self, Area,
        patch::{Document, Extent, Source, SourceOmission},
    },
};
use std::{fs, path::Path};
use test_support::*;

fn document(f: &Fixture, name: &str, area: Area) -> Document {
    let snapshot = files::read(&f.repo).unwrap();
    let change = snapshot
        .changes
        .iter()
        .find(|change| change.location.name() == Path::new(name) && change.area == area)
        .unwrap();
    files::document(&snapshot, change, &Cancel::default()).unwrap()
}
fn sources(document: &Document) -> (&Source, &Source) {
    let Document::Patch { files, .. } = document else {
        panic!("应该生成 patch")
    };
    assert_eq!(files.len(), 1);
    (&files[0].before, &files[0].after)
}

#[test]
fn staged_worktree_and_untracked_sources_use_the_correct_versions() {
    let f = Fixture::create();
    f.commit("源码.rs", "fn baseline() {}\n");
    write(f.repo.join("源码.rs"), "fn staged() {}\n");
    git(&f.repo, &["add", "源码.rs"]);
    write(f.repo.join("源码.rs"), "fn working() {}\n");
    let staged = document(&f, "源码.rs", Area::Staged);
    assert!(
        matches!(sources(&staged), (Source::Text { text: old, .. }, Source::Text { text: new, .. }) if old.contains("baseline") && new.contains("staged"))
    );
    let working = document(&f, "源码.rs", Area::Unstaged);
    assert!(
        matches!(sources(&working), (Source::Text { text: old, .. }, Source::Text { text: new, .. }) if old.contains("staged") && new.contains("working"))
    );
    write(f.repo.join("新 文件.py"), "print('中文')\n");
    let added = document(&f, "新 文件.py", Area::Untracked);
    assert!(
        matches!(sources(&added), (Source::Absent, Source::Text { text, .. }) if text.contains("中文"))
    );
    fs::remove_file(f.repo.join("源码.rs")).unwrap();
    let deleted = document(&f, "源码.rs", Area::Unstaged);
    assert!(
        matches!(sources(&deleted), (Source::Text { text, .. }, Source::Absent) if text.contains("staged"))
    );
}

#[test]
fn binary_and_oversized_sources_remain_explicit_without_blocking_the_patch() {
    let f = Fixture::create();
    write(f.repo.join("binary.bin"), "before\0bytes");
    git(&f.repo, &["add", "binary.bin"]);
    git(&f.repo, &["commit", "-m", "binary"]);
    write(f.repo.join("binary.bin"), "after\0bytes");
    let binary = document(&f, "binary.bin", Area::Unstaged);
    assert!(binary.text().contains("Binary"));
    assert!(matches!(
        sources(&binary),
        (
            Source::Unavailable(SourceOmission::Binary),
            Source::Unavailable(SourceOmission::Binary)
        )
    ));
    let large = "// context\n".repeat(110_000);
    f.commit("large.rs", &large);
    write(
        f.repo.join("large.rs"),
        &format!("fn changed() {{}}\n{}", &large[11..]),
    );
    let limited = document(&f, "large.rs", Area::Unstaged);
    assert!(limited.text().contains("+fn changed()"));
    assert!(matches!(
        sources(&limited),
        (
            Source::Unavailable(SourceOmission::TooLarge),
            Source::Unavailable(SourceOmission::TooLarge)
        )
    ));
}

#[test]
fn patch_output_stops_at_the_bound_and_retains_line_coordinates() {
    let f = Fixture::create();
    write(f.repo.join("many.rs"), &"let value = 1;\n".repeat(20_000));
    let patch = document(&f, "many.rs", Area::Untracked);
    let Document::Patch { files, extent } = patch else {
        panic!("应该生成 patch")
    };
    assert_eq!(extent, Extent::Truncated);
    let lines = &files[0].lines;
    let added = lines
        .iter()
        .filter_map(|line| match line {
            files::patch::Line::Added { new, .. } => Some(new.get()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(added.len() < 20_000 && added.len() > 100);
    assert_eq!(added[0], 1);
    assert_eq!(*added.last().unwrap(), added.len() as u32);
}
