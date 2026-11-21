use engine::{
    cancel::Cancel,
    operations::reset::{self, Effect, Mode},
    transport::Git,
};
use test_support::{Fixture, git, write};

#[test]
fn soft_reset_moves_the_branch_and_preserves_staged_and_unstaged_content() {
    let f = Fixture::create();
    let cancel = Cancel::default();
    let target = git(&f.repo, &["rev-parse", "HEAD"]);
    f.commit("first.txt", "first\n");
    f.commit("second.txt", "second\n");
    let before = git(&f.repo, &["rev-parse", "HEAD"]);
    write(f.repo.join("first.txt"), "staged\n");
    git(&f.repo, &["add", "first.txt"]);
    write(f.repo.join("first.txt"), "working\n");
    write(f.repo.join("untracked.txt"), "untracked\n");
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();

    let plan = reset::prepare(&f.repo, target.parse().unwrap(), Mode::Soft, &cancel).unwrap();
    let (commits, count) = plan.removed();
    assert_eq!(count, 2);
    assert_eq!(commits[0].oid.to_string(), before);
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), before);
    reset::execute(&Git::default(), &plan, &cancel).unwrap();

    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), target);
    assert_eq!(git(&f.repo, &["rev-parse", "ORIG_HEAD"]), before);
    assert_eq!(git(&f.repo, &["symbolic-ref", "--short", "HEAD"]), "main");
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert_eq!(git(&f.repo, &["show", ":first.txt"]), "staged");
    assert_eq!(
        std::fs::read_to_string(f.repo.join("first.txt")).unwrap(),
        "working\n"
    );
    assert_eq!(
        std::fs::read_to_string(f.repo.join("untracked.txt")).unwrap(),
        "untracked\n"
    );
    assert_eq!(
        git(&f.repo, &["reflog", "-1", "--format=%H", "main"]),
        target
    );
}

#[test]
fn reset_rejects_unrelated_targets_and_a_branch_switch_after_review() {
    let f = Fixture::create();
    let cancel = Cancel::default();
    let target = git(&f.repo, &["rev-parse", "HEAD"]);
    git(&f.repo, &["switch", "-c", "other"]);
    f.commit("elsewhere.txt", "other branch\n");
    let unrelated = git(&f.repo, &["rev-parse", "HEAD"]);
    git(&f.repo, &["switch", "main"]);
    f.commit("first.txt", "first\n");
    let before = git(&f.repo, &["rev-parse", "HEAD"]);
    assert!(reset::prepare(&f.repo, unrelated.parse().unwrap(), Mode::Soft, &cancel).is_err());

    let plan = reset::prepare(&f.repo, target.parse().unwrap(), Mode::Soft, &cancel).unwrap();
    git(&f.repo, &["switch", "-c", "same-tip"]);
    assert!(reset::execute(&Git::default(), &plan, &cancel).is_err());
    assert_eq!(git(&f.repo, &["rev-parse", "main"]), before);
    assert_eq!(git(&f.repo, &["rev-parse", "same-tip"]), before);
}

#[test]
fn hard_reset_restores_tracked_files_and_removes_only_obstructing_untracked_files() {
    let f = Fixture::create();
    let cancel = Cancel::default();
    f.commit("tracked.txt", "target\n");
    f.commit("blocked.txt", "tracked at target\n");
    let target = git(&f.repo, &["rev-parse", "HEAD"]);
    git(&f.repo, &["rm", "blocked.txt"]);
    git(&f.repo, &["commit", "-m", "Remove blocker"]);
    let before = git(&f.repo, &["rev-parse", "HEAD"]);
    write(f.repo.join(".git/info/exclude"), "/blocked.txt\n");
    write(f.repo.join("blocked.txt"), "ignored obstruction\n");
    write(f.repo.join("unrelated.txt"), "leave this untracked\n");
    write(f.repo.join("tracked.txt"), "staged\n");
    git(&f.repo, &["add", "tracked.txt"]);
    write(f.repo.join("tracked.txt"), "working\n");

    let plan = reset::prepare(&f.repo, target.parse().unwrap(), Mode::Hard, &cancel).unwrap();
    let Effect::Hard(review) = plan.effect() else {
        panic!("应预览硬重置")
    };
    assert_eq!(review.changes().len(), 2);
    assert_eq!(
        review.obstructions().collect::<Vec<_>>(),
        [std::path::Path::new("blocked.txt")]
    );
    reset::execute(&Git::default(), &plan, &cancel).unwrap();

    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), target);
    assert_eq!(git(&f.repo, &["rev-parse", "ORIG_HEAD"]), before);
    assert_eq!(git(&f.repo, &["show", ":tracked.txt"]), "target");
    assert_eq!(git(&f.repo, &["diff", "--name-only"]), "");
    assert_eq!(git(&f.repo, &["diff", "--cached", "--name-only"]), "");
    assert_eq!(
        std::fs::read_to_string(f.repo.join("blocked.txt")).unwrap(),
        "tracked at target\n"
    );
    assert_eq!(
        std::fs::read_to_string(f.repo.join("unrelated.txt")).unwrap(),
        "leave this untracked\n"
    );
}

#[test]
fn hard_reset_rejects_edits_made_after_review_including_ignored_obstructions() {
    let f = Fixture::create();
    let cancel = Cancel::default();
    f.commit("tracked.txt", "target\n");
    f.commit("blocked.txt", "tracked at target\n");
    let target = git(&f.repo, &["rev-parse", "HEAD"]);
    git(&f.repo, &["rm", "blocked.txt"]);
    git(&f.repo, &["commit", "-m", "Remove blocker"]);
    let before = git(&f.repo, &["rev-parse", "HEAD"]);
    write(f.repo.join(".git/info/exclude"), "/blocked.txt\n");
    write(f.repo.join("blocked.txt"), "before review\n");
    let plan = reset::prepare(&f.repo, target.parse().unwrap(), Mode::Hard, &cancel).unwrap();
    write(f.repo.join("blocked.txt"), "after review\n");
    assert!(reset::execute(&Git::default(), &plan, &cancel).is_err());
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), before);
    assert_eq!(
        std::fs::read_to_string(f.repo.join("blocked.txt")).unwrap(),
        "after review\n"
    );

    let plan = reset::prepare(&f.repo, before.parse().unwrap(), Mode::Hard, &cancel).unwrap();
    write(f.repo.join("tracked.txt"), "new work\n");
    assert!(reset::execute(&Git::default(), &plan, &cancel).is_err());
    assert_eq!(
        std::fs::read_to_string(f.repo.join("tracked.txt")).unwrap(),
        "new work\n"
    );
    let plan = reset::prepare(&f.repo, before.parse().unwrap(), Mode::Hard, &cancel).unwrap();
    reset::execute(&Git::default(), &plan, &cancel).unwrap();
    assert_eq!(git(&f.repo, &["rev-parse", "HEAD"]), before);
    assert_eq!(
        std::fs::read_to_string(f.repo.join("tracked.txt")).unwrap(),
        "target\n"
    );
}

#[test]
fn hard_reset_reviews_a_directory_that_replaced_a_tracked_file() {
    let f = Fixture::create();
    let cancel = Cancel::default();
    f.commit("tracked.txt", "file at target\n");
    let target = git(&f.repo, &["rev-parse", "HEAD"]).parse().unwrap();
    write(f.repo.join(".git/info/exclude"), "/tracked.txt/local.txt\n");
    std::fs::remove_file(f.repo.join("tracked.txt")).unwrap();
    std::fs::create_dir(f.repo.join("tracked.txt")).unwrap();
    write(f.repo.join("tracked.txt/local.txt"), "blocked directory\n");

    let plan = reset::prepare(&f.repo, target, Mode::Hard, &cancel).unwrap();
    let Effect::Hard(review) = plan.effect() else {
        panic!("应预览硬重置")
    };
    assert_eq!(
        review.obstructions().collect::<Vec<_>>(),
        [std::path::Path::new("tracked.txt")]
    );
    reset::execute(&Git::default(), &plan, &cancel).unwrap();
    assert_eq!(
        std::fs::read_to_string(f.repo.join("tracked.txt")).unwrap(),
        "file at target\n"
    );
}
