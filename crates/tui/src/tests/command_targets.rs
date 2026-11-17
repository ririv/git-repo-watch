use super::*;
use crate::command::{
    availability::Availability,
    target::{CommandContext, Target},
};

#[test]
fn empty_history_is_a_real_target_while_selection_is_temporarily_unavailable() {
    let root = tempfile::tempdir().unwrap();
    let repo = root.path().join("empty");
    support::init(&repo);
    let repo = repo.canonicalize().unwrap();
    let (_home, mut s) = fixture(vec![repo.clone()]);
    let selection = s.copy_selection();
    let context = CommandContext::new(&s, Some(&repo), &selection);
    let Target::Ready(repository) = context.repository() else {
        panic!("repository target")
    };
    assert_eq!(repository, repo);
    assert!(matches!(context.history(), Target::OutOfScope(_)));
    command(&mut s, "repo log");
    wait(&mut s);
    let selection = s.copy_selection();
    let context = CommandContext::new(&s, Some(&repo), &selection);
    let Target::Ready(graph) = context.history() else {
        panic!("history target")
    };
    assert!(graph.records.is_empty());
    assert_eq!(graph.references.path, repo);
    assert!(matches!(
        context.resolve(Action::LogSelect),
        Target::Unavailable(_)
    ));
    let unavailable = s.resolve_command(Action::LogSelect, Some(&repo), &selection);
    assert!(matches!(
        unavailable,
        Availability::Target(Target::Unavailable(_))
    ));
    assert_eq!(unavailable.reason(), Some(i18n::tr!(ReplaySelectCommit)));
    s.shutdown();
}

fn select_file(s: &mut Screen, name: &str) {
    let Mode::Changes(session) = &mut s.mode else {
        panic!("changes view")
    };
    let index = session
        .files
        .rows()
        .iter()
        .position(|change| change.location.name() == std::path::Path::new(name))
        .unwrap();
    session.files.cursor = crate::files::Cursor::File(index);
}

#[test]
fn executing_a_ready_index_target_uses_its_captured_files_after_the_cursor_moves() {
    let f = support::Fixture::create();
    for name in ["a.txt", "b.txt"] {
        f.commit(name, "before\n");
        support::write(f.repo.join(name), "after\n");
    }
    let head = support::git(&f.repo, &["rev-parse", "HEAD"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    select_file(&mut s, "a.txt");
    let selection = s.copy_selection();
    let execution = s
        .prepare_command(Action::Stage, Some(&f.repo), &selection)
        .unwrap();
    select_file(&mut s, "b.txt");
    execution.execute(&mut s, &Cancel::default()).unwrap();
    wait(&mut s);
    assert_eq!(support::git(&f.repo, &["show", ":a.txt"]), "after");
    assert_eq!(support::git(&f.repo, &["show", ":b.txt"]), "before");
    assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), head);
    assert_eq!(
        std::fs::read_to_string(f.repo.join("b.txt")).unwrap(),
        "after\n"
    );
    s.shutdown();
}

#[test]
fn index_resolution_rejects_a_captured_repository_that_does_not_own_the_view() {
    let first = support::Fixture::create();
    let second = support::Fixture::create();
    first.commit("changed.txt", "before\n");
    support::write(first.repo.join("changed.txt"), "after\n");
    let (_home, mut s) = fixture(vec![first.repo.clone(), second.repo.clone()]);
    s.focus(Some(first.repo.clone()));
    command(&mut s, "repo diff");
    wait(&mut s);
    let selection = s.copy_selection();
    let result = s.resolve_command(Action::Stage, Some(&second.repo), &selection);
    assert!(matches!(
        result,
        Availability::Target(Target::Unavailable(_))
    ));
    assert_eq!(result.reason(), Some(i18n::tr!(PartialChanged)));
    assert!(support::git(&first.repo, &["diff", "--cached"]).is_empty());
    assert!(support::git(&second.repo, &["diff", "--cached"]).is_empty());
    assert!(s.repository_tasks.is_empty());
    s.shutdown();
}
