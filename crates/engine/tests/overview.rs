use engine::{
    cancel::Cancel,
    local::overview::{self, Checkout, Comparison, Operation, Tracking},
};
use test_support::{Fixture, git, init, write};

fn read(path: &std::path::Path) -> overview::Snapshot {
    overview::read(path, &Cancel::default()).unwrap()
}
fn comparison(snapshot: &overview::Snapshot) -> &Comparison {
    match snapshot.checkout.as_ref().unwrap() {
        Checkout::Branch {
            tracking: Ok(Tracking::Remote { comparison, .. } | Tracking::Local { comparison, .. }),
            ..
        } => comparison,
        other => panic!("{other:?}"),
    }
}

#[test]
fn local_snapshot_counts_both_sides_and_respects_ignores_without_writing_or_fetching() {
    let f = Fixture::create();
    f.commit("tracked", "base\n");
    f.commit(".gitignore", "ignored/\n");
    git(&f.repo, &["push"]);
    write(f.repo.join("tracked"), "staged\n");
    git(&f.repo, &["add", "tracked"]);
    write(f.repo.join("tracked"), "unstaged\n");
    for n in 0..6 {
        write(f.repo.join(format!("中文 {n}")), "new\n");
    }
    std::fs::create_dir(f.repo.join("ignored")).unwrap();
    write(f.repo.join("ignored/private"), "ignored\n");
    f.advance();
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    let config = std::fs::read(f.repo.join(".git/config")).unwrap();
    let reference = git(&f.repo, &["rev-parse", "origin/main"]);
    let snapshot = read(&f.repo);
    let changes = snapshot.changes.as_ref().unwrap();
    assert_eq!(
        (
            changes.staged,
            changes.unstaged,
            changes.untracked,
            changes.conflicts
        ),
        (1, 1, 6, 0)
    );
    assert_eq!(
        comparison(&snapshot),
        &Comparison::Known {
            ahead: 0,
            behind: 0
        }
    );
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert_eq!(std::fs::read(f.repo.join(".git/config")).unwrap(), config);
    assert_eq!(git(&f.repo, &["rev-parse", "origin/main"]), reference);
    git(&f.repo, &["fetch", "origin"]);
    assert_eq!(
        comparison(&read(&f.repo)),
        &Comparison::Known {
            ahead: 0,
            behind: 1
        }
    );
}

#[test]
fn tracking_honors_custom_refspec_and_distinguishes_missing_refs_and_absent_tracking() {
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
    let snapshot = read(&f.repo);
    assert!(
        matches!(snapshot.checkout.unwrap(), Checkout::Branch { tracking: Ok(Tracking::Remote { ref name, comparison: Comparison::Known { ahead: 0, behind: 0 } }), .. } if name=="custom/main")
    );
    git(&f.repo, &["update-ref", "-d", "refs/remotes/custom/main"]);
    assert_eq!(comparison(&read(&f.repo)), &Comparison::Missing);
    git(&f.repo, &["config", "--remove-section", "branch.main"]);
    assert!(matches!(
        read(&f.repo).checkout.unwrap(),
        Checkout::Branch {
            tracking: Ok(Tracking::Absent),
            ..
        }
    ));
    git(&f.repo, &["branch", "experiment"]);
    git(&f.repo, &["config", "branch.main.remote", "."]);
    git(
        &f.repo,
        &["config", "branch.main.merge", "refs/heads/experiment"],
    );
    assert!(matches!(
        read(&f.repo).checkout.unwrap(),
        Checkout::Branch {
            tracking: Ok(Tracking::Local { .. }),
            ..
        }
    ));
}

#[test]
fn detached_and_unborn_repositories_still_have_usable_local_information() {
    let f = Fixture::create();
    git(&f.repo, &["checkout", "--detach"]);
    let snapshot = read(&f.repo);
    assert!(matches!(snapshot.checkout.unwrap(), Checkout::Detached(_)));
    let path = f.temp.path().join("empty");
    init(&path);
    write(path.join("new"), "new\n");
    let snapshot = read(&path);
    assert!(matches!(snapshot.checkout.unwrap(), Checkout::Unborn(ref name) if name=="main"));
    assert_eq!(snapshot.changes.unwrap().untracked, 1);
}

#[test]
fn merge_conflicts_are_visible_instead_of_rejecting_the_overview() {
    let f = Fixture::create();
    f.commit("conflict", "base\n");
    git(&f.repo, &["checkout", "-b", "feature"]);
    f.commit("conflict", "feature\n");
    git(&f.repo, &["checkout", "main"]);
    f.commit("conflict", "main\n");
    let merge = std::process::Command::new("/usr/bin/git")
        .arg("-C")
        .arg(&f.repo)
        .args(["merge", "feature"])
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .unwrap();
    assert_eq!(merge.status.code(), Some(1));
    let snapshot = read(&f.repo);
    assert_eq!(snapshot.operation, Operation::Merge);
    let changes = snapshot.changes.unwrap();
    assert_eq!(changes.conflicts, 1);
    assert_eq!(changes.staged + changes.unstaged, 0);
}

#[test]
fn malformed_tracking_is_reported_separately_from_worktree() {
    let f = Fixture::create();
    git(&f.repo, &["config", "--unset", "branch.main.merge"]);
    let snapshot = read(&f.repo);
    assert!(matches!(
        snapshot.checkout.unwrap(),
        Checkout::Branch {
            tracking: Err(_),
            ..
        }
    ));
    assert!(snapshot.changes.is_ok());
}
