use engine::{
    cancel::Cancel,
    local::branches::{
        catalog::{self, Head, Kind},
        upstream::{Comparison, Tracking},
    },
};
use test_support::{Fixture, git, init};
fn read(path: &std::path::Path) -> catalog::Snapshot {
    catalog::read(path, &Cancel::default()).unwrap()
}

#[test]
fn catalog_marks_current_branch_reads_local_tracking_and_omits_remote_aliases_without_writes() {
    let f = Fixture::create();
    git(&f.repo, &["branch", "experiment"]);
    git(
        &f.repo,
        &[
            "symbolic-ref",
            "refs/remotes/origin/HEAD",
            "refs/remotes/origin/main",
        ],
    );
    f.commit("ahead", "ahead\n");
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    let config = std::fs::read(f.repo.join(".git/config")).unwrap();
    let refs = git(&f.repo, &["show-ref"]);
    f.advance();
    let snapshot = read(&f.repo);
    assert_eq!(snapshot.entries[0].name, "main");
    assert!(snapshot.current(&snapshot.entries[0]));
    assert!(matches!(
        &snapshot.entries[0].kind,
        Kind::Local(Ok(Tracking::Remote {
            comparison: Comparison::Known {
                ahead: 1,
                behind: 0
            },
            ..
        }))
    ));
    assert!(
        snapshot
            .entries
            .iter()
            .any(|entry| entry.name == "experiment"
                && matches!(entry.kind, Kind::Local(Ok(Tracking::Absent))))
    );
    assert!(
        snapshot
            .entries
            .iter()
            .any(|entry| entry.name == "origin/main" && matches!(entry.kind, Kind::Remote))
    );
    assert!(
        !snapshot
            .entries
            .iter()
            .any(|entry| entry.name == "origin/HEAD")
    );
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert_eq!(std::fs::read(f.repo.join(".git/config")).unwrap(), config);
    assert_eq!(git(&f.repo, &["show-ref"]), refs);
}

#[test]
fn custom_mapping_missing_upstream_and_malformed_branch_config_are_distinct() {
    let f = Fixture::create();
    git(
        &f.repo,
        &[
            "config",
            "remote.origin.fetch",
            "+refs/heads/*:refs/remotes/custom/*",
        ],
    );
    git(&f.repo, &["fetch", "origin"]);
    git(&f.repo, &["branch", "broken"]);
    git(&f.repo, &["config", "branch.broken.remote", "origin"]);
    let snapshot = read(&f.repo);
    assert!(
        matches!(&snapshot.entries[0].kind,Kind::Local(Ok(Tracking::Remote {name, comparison: Comparison::Known {ahead:0,behind:0}})) if name=="custom/main")
    );
    assert!(
        snapshot
            .entries
            .iter()
            .any(|entry| entry.name == "broken" && matches!(entry.kind, Kind::Local(Err(_))))
    );
    git(&f.repo, &["update-ref", "-d", "refs/remotes/custom/main"]);
    assert!(matches!(
        &read(&f.repo).entries[0].kind,
        Kind::Local(Ok(Tracking::Remote {
            comparison: Comparison::Missing,
            ..
        }))
    ));
}

#[test]
fn detached_unborn_and_empty_message_commits_are_valid_catalogs() {
    let f = Fixture::create();
    git(&f.repo, &["checkout", "--detach"]);
    let snapshot = read(&f.repo);
    assert!(matches!(snapshot.head, Head::Detached(_)));
    assert!(!snapshot.entries.iter().any(|entry| snapshot.current(entry)));
    let empty = f.temp.path().join("empty");
    init(&empty);
    let snapshot = read(&empty);
    assert!(matches!(snapshot.head,Head::Unborn(ref name) if name=="main"));
    assert!(snapshot.entries.is_empty());
    git(&f.repo, &["checkout", "main"]);
    git(
        &f.repo,
        &["commit", "--allow-empty", "--allow-empty-message", "-m", ""],
    );
    assert_eq!(read(&f.repo).entries[0].subject, "");
    let cancel = Cancel::default();
    cancel.cancel();
    assert!(catalog::read(&f.repo, &cancel).is_err());
}
