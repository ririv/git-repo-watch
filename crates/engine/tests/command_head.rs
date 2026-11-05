use engine::{
    cancel::Cancel,
    local::head::{self, Head},
};
use test_support::{Fixture, git};

#[test]
fn head_identity_is_local_and_does_not_modify_repository_contents() {
    let f = Fixture::create();
    git(&f.repo, &["branch", "-m", "feature/中文"]);
    git(
        &f.repo,
        &["remote", "set-url", "origin", "/missing/offline-remote"],
    );
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    let refs = git(&f.repo, &["show-ref"]);
    assert!(
        matches!(head::read(&f.repo, &Cancel::default()).unwrap(), Head::Branch(name) if name == "feature/中文")
    );
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert_eq!(git(&f.repo, &["show-ref"]), refs);
    let oid = git(&f.repo, &["rev-parse", "HEAD"]);
    git(&f.repo, &["checkout", "--detach", "HEAD"]);
    assert!(
        matches!(head::read(&f.repo, &Cancel::default()).unwrap(), Head::Detached(actual) if actual == oid)
    );
}

#[test]
fn unborn_checkout_retains_its_explicit_branch_name() {
    let temp = tempfile::tempdir().unwrap();
    test_support::init(temp.path());
    let branch = git(temp.path(), &["symbolic-ref", "--short", "HEAD"]);
    assert!(
        matches!(head::read(temp.path(), &Cancel::default()).unwrap(), Head::Unborn(actual) if actual == branch)
    );
}
