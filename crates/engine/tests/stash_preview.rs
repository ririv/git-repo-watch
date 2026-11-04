use engine::{
    cancel::Cancel,
    local::{
        history::Kind,
        stash::{
            self,
            preview::{self, Scope},
        },
    },
};
use std::{fs, path::Path};
use test_support::{Fixture, git, write};

fn names(detail: &preview::Detail) -> Vec<String> {
    detail
        .files
        .iter()
        .map(|file| file.change.location.name().display().to_string())
        .collect()
}
fn patch(detail: &preview::Detail, path: &str) -> String {
    let index = detail
        .files
        .iter()
        .position(|file| file.change.location.name() == Path::new(path))
        .unwrap();
    preview::document(detail, index, &Cancel::default())
        .unwrap()
        .text()
}
#[test]
fn native_stash_layers_include_untracked_and_intermediate_staged_content_without_current_files() {
    let f = Fixture::create();
    f.commit("tracked.ts", "export const value = 'base';\n");
    f.commit("reverted.ts", "export const reverted = 'base';\n");
    write(f.repo.join("tracked.ts"), "export const value = 'index';\n");
    write(
        f.repo.join("reverted.ts"),
        "export const reverted = 'index-only';\n",
    );
    git(&f.repo, &["add", "tracked.ts", "reverted.ts"]);
    write(
        f.repo.join("tracked.ts"),
        "export const value = 'working';\n",
    );
    write(
        f.repo.join("reverted.ts"),
        "export const reverted = 'base';\n",
    );
    write(f.repo.join("中文 [1] file.md"), "# untracked snapshot\n");
    fs::write(f.repo.join("binary.dat"), [0, 1, 2, 3]).unwrap();
    git(&f.repo, &["stash", "push", "-u", "-m", "layered preview"]);
    let entry = stash::read(&f.repo, &Cancel::default()).unwrap().entries[0].clone();
    write(
        f.repo.join("tracked.ts"),
        "export const value = 'current-index';\n",
    );
    git(&f.repo, &["add", "tracked.ts"]);
    write(
        f.repo.join("tracked.ts"),
        "export const value = 'current-worktree';\n",
    );
    write(f.repo.join("中文 [1] file.md"), "current untracked\n");
    let index = fs::read(f.repo.join(".git/index")).unwrap();
    let refs = git(&f.repo, &["show-ref"]);
    let all = preview::read(&f.repo, &entry, Scope::All, &Cancel::default()).unwrap();
    assert_eq!(
        names(&all),
        ["binary.dat", "tracked.ts", "中文 [1] file.md"]
    );
    let diff = patch(&all, "tracked.ts");
    assert!(diff.contains("'base'") && diff.contains("'working'"));
    assert!(!diff.contains("current-") && !diff.contains("'index'"));
    assert!(patch(&all, "中文 [1] file.md").contains("untracked snapshot"));
    assert!(patch(&all, "binary.dat").contains("Binary"));
    let staged = preview::read(&f.repo, &entry, Scope::Staged, &Cancel::default()).unwrap();
    assert_eq!(names(&staged), ["reverted.ts", "tracked.ts"]);
    assert!(patch(&staged, "reverted.ts").contains("index-only"));
    let diff = patch(&staged, "tracked.ts");
    assert!(diff.contains("'base'") && diff.contains("'index'"));
    assert!(!diff.contains("'working'"));
    let unstaged = preview::read(&f.repo, &entry, Scope::Unstaged, &Cancel::default()).unwrap();
    assert_eq!(
        names(&unstaged),
        [
            "binary.dat",
            "reverted.ts",
            "tracked.ts",
            "中文 [1] file.md"
        ]
    );
    let diff = patch(&unstaged, "tracked.ts");
    assert!(diff.contains("'index'") && diff.contains("'working'"));
    assert!(!diff.contains("'base'"));
    assert_eq!(fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert_eq!(git(&f.repo, &["show-ref"]), refs);
    assert!(
        fs::read_to_string(f.repo.join("tracked.ts"))
            .unwrap()
            .contains("current-worktree")
    );
    assert_eq!(
        fs::read_to_string(f.repo.join("中文 [1] file.md")).unwrap(),
        "current untracked\n"
    );
    // 列表顺序变化或原记录被删除，不会把已打开的对象切换为另一份 stash。
    git(&f.repo, &["stash", "push", "-u", "-m", "another stash"]);
    git(&f.repo, &["stash", "drop", "stash@{1}"]);
    let retained = preview::read(&f.repo, &entry, Scope::All, &Cancel::default()).unwrap();
    assert_eq!(patch(&retained, "tracked.ts"), patch(&all, "tracked.ts"));
}

#[test]
fn tracked_stash_previews_preserve_renames_deletions_modes_and_empty_scopes() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::create();
    f.commit("before.md", &"rename line\n".repeat(20));
    f.commit("delete.md", "delete me\n");
    f.commit("script.sh", "echo example\n");
    git(&f.repo, &["mv", "before.md", "after.md"]);
    fs::remove_file(f.repo.join("delete.md")).unwrap();
    fs::set_permissions(f.repo.join("script.sh"), fs::Permissions::from_mode(0o755)).unwrap();
    git(&f.repo, &["add", "-u"]);
    git(&f.repo, &["stash", "push", "-m", "tracked only"]);
    let entry = stash::read(&f.repo, &Cancel::default()).unwrap().entries[0].clone();
    for scope in [Scope::All, Scope::Staged] {
        let detail = preview::read(&f.repo, &entry, scope, &Cancel::default()).unwrap();
        assert_eq!(names(&detail), ["after.md", "delete.md", "script.sh"]);
        assert_eq!(detail.files[0].change.kind, Kind::Renamed);
        assert_eq!(detail.files[1].change.kind, Kind::Deleted);
        assert!(patch(&detail, "script.sh").contains("100755"));
    }
    assert!(
        preview::read(&f.repo, &entry, Scope::Unstaged, &Cancel::default())
            .unwrap()
            .files
            .is_empty()
    );
    let cancel = Cancel::default();
    cancel.cancel();
    assert!(preview::read(&f.repo, &entry, Scope::All, &cancel).is_err());
}

#[test]
fn gw_selected_stash_inspection_never_imports_unselected_staged_files() {
    let f = Fixture::create();
    f.commit("selected.txt", "base selected\n");
    f.commit("other.txt", "base other\n");
    write(f.repo.join("selected.txt"), "saved selected\n");
    write(f.repo.join("other.txt"), "keep other staging\n");
    git(&f.repo, &["add", "other.txt"]);
    write(f.repo.join("new 中文.md"), "chosen untracked\n");
    let mode = stash::SaveMode::Selected {
        files: stash::SelectedFiles::new(["selected.txt".into(), "new 中文.md".into()]).unwrap(),
    };
    let plan = stash::prepare(
        &f.repo,
        stash::Request::Save {
            mode,
            message: "selected preview".into(),
        },
        &Cancel::default(),
    )
    .unwrap();
    stash::execute(
        &engine::transport::Git {
            timeout: std::time::Duration::from_secs(10),
        },
        &plan,
        &Cancel::default(),
    )
    .unwrap();
    let entry = stash::read(&f.repo, &Cancel::default()).unwrap().entries[0].clone();
    let index = fs::read(f.repo.join(".git/index")).unwrap();
    for scope in [Scope::All, Scope::Unstaged] {
        let detail = preview::read(&f.repo, &entry, scope, &Cancel::default()).unwrap();
        assert_eq!(names(&detail), ["new 中文.md", "selected.txt"]);
        assert!(patch(&detail, "selected.txt").contains("saved selected"));
    }
    assert!(
        preview::read(&f.repo, &entry, Scope::Staged, &Cancel::default())
            .unwrap()
            .files
            .is_empty()
    );
    assert_eq!(fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert!(git(&f.repo, &["diff", "--cached"]).contains("keep other staging"));
}
