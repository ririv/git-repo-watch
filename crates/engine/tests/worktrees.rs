use engine::{
    cancel::Cancel,
    local::{
        open,
        worktrees::{self, Branch, Kind, Request, State, Status},
    },
    transport::Git,
};
use test_support::{Fixture, git, write};

#[test]
fn worktree_plans_preserve_the_source_and_refuse_unreviewed_files() {
    let fixture = Fixture::create();
    fixture.commit(".gitignore", "*.local\n");
    let cancel = Cancel::default();
    let git_backend = Git::default();
    let source = &fixture.repo;
    let source_head = open(source).unwrap().head().unwrap().target();
    let linked = fixture.temp.path().join("工作树 with spaces");
    let request = Request::Add {
        branch: Branch::New,
        name: "topic".into(),
        directory: linked.clone(),
    };
    let plan = worktrees::prepare(source, request, &cancel).unwrap();
    assert!(!linked.exists());
    worktrees::execute(&git_backend, &plan, &cancel).unwrap();
    let linked = linked.canonicalize().unwrap();
    assert_eq!(open(source).unwrap().head().unwrap().target(), source_head);
    assert_eq!(
        open(&linked).unwrap().head().unwrap().shorthand().unwrap(),
        "topic"
    );

    let snapshot = worktrees::read(source, &cancel).unwrap();
    assert_eq!(snapshot.entries.len(), 2);
    assert!(snapshot.entries[0].current);
    assert_eq!(snapshot.entries[0].kind, Kind::Main);
    assert!(
        worktrees::prepare(
            source,
            Request::Remove(snapshot.entries[0].clone()),
            &cancel
        )
        .is_err()
    );
    let entry = snapshot.entries[1].clone();
    let reviewed = worktrees::prepare(source, Request::Remove(entry), &cancel).unwrap();
    write(linked.join("credentials.local"), "keep\n");
    assert!(worktrees::execute(&git_backend, &reviewed, &cancel).is_err());
    assert_eq!(
        std::fs::read_to_string(linked.join("credentials.local")).unwrap(),
        "keep\n"
    );
    let dirty = &worktrees::read(source, &cancel).unwrap().entries[1];
    assert!(matches!(
        dirty.state,
        State::Available {
            status: Status::IgnoredFiles,
            ..
        }
    ));
    assert!(worktrees::prepare(source, Request::Remove(dirty.clone()), &cancel).is_err());
    std::fs::remove_file(linked.join("credentials.local")).unwrap();

    let from_linked = worktrees::read(&linked, &cancel).unwrap();
    assert_eq!(from_linked.entries.len(), 2);
    assert!(from_linked.entries[0].current);
    assert_eq!(from_linked.entries[0].path, linked);
    assert!(
        from_linked
            .entries
            .iter()
            .any(|entry| entry.kind == Kind::Main)
    );
    worktrees::execute(&git_backend, &reviewed, &cancel).unwrap();
    assert!(!linked.exists());
    assert!(
        open(source)
            .unwrap()
            .find_branch("topic", git2::BranchType::Local)
            .is_ok()
    );

    let elsewhere = fixture.temp.path().join("existing branch");
    let plan = worktrees::prepare(
        source,
        Request::Add {
            branch: Branch::Existing,
            name: "topic".into(),
            directory: elsewhere.clone(),
        },
        &cancel,
    )
    .unwrap();
    worktrees::execute(&git_backend, &plan, &cancel).unwrap();
    assert_eq!(
        open(&elsewhere)
            .unwrap()
            .head()
            .unwrap()
            .shorthand()
            .unwrap(),
        "topic"
    );
    git(source, &["worktree", "lock", elsewhere.to_str().unwrap()]);
    let snapshot = worktrees::read(source, &cancel).unwrap();
    assert!(matches!(snapshot.entries[1].kind, Kind::Locked(_)));
    assert!(
        worktrees::prepare(
            source,
            Request::Remove(snapshot.entries[1].clone()),
            &cancel
        )
        .is_err()
    );
}
