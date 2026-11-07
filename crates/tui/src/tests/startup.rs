use super::*;

#[test]
fn current_directory_start_opens_registered_or_session_repositories_and_back_returns_home() {
    for registered in [true, false] {
        let f = support::Fixture::create();
        support::write(f.repo.join("文件.ts"), "export const value = 1;\n");
        let (_home, old) = fixture(if registered {
            vec![f.repo.clone()]
        } else {
            vec![]
        });
        let config = std::fs::read(old.service.store.config_path()).unwrap();
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        let entry = crate::Start::CurrentDirectory
            .in_directory(&f.repo, &Cancel::default())
            .unwrap();
        let mut s = entry
            .open((*old.service).clone(), &Cancel::default())
            .unwrap();
        assert_eq!(s.page, Page::Detail);
        assert!(matches!(s.mode, Mode::Browsing));
        assert_eq!(s.scope_path(), Some(f.repo.clone()));
        assert_eq!(s.breadcrumbs().len(), 2);
        wait_workspace(&mut s);
        super::overview::wait_overview(&mut s);
        assert!(matches!(
            &s.workspace.session().unwrap().files,
            crate::workspace::Panel::Ready(_)
        ));
        assert!(matches!(
            &s.workspace.session().unwrap().branches,
            crate::workspace::Panel::Ready(_)
        ));
        assert!(matches!(
            &s.workspace.session().unwrap().history,
            crate::workspace::Panel::Ready(_)
        ));
        assert!(render(&mut s, 160, 36).contains("文件.ts"));
        command(&mut s, "repo diff");
        wait(&mut s);
        assert_eq!(s.mode.file_session().unwrap().snapshot.path, f.repo);
        key(&mut s, KeyCode::Esc);
        key(&mut s, KeyCode::Left);
        assert_eq!(s.page, Page::Repositories);
        assert_eq!(
            std::fs::read(s.service.store.config_path()).unwrap(),
            config
        );
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        s.shutdown();
    }
}

#[test]
fn explicit_repository_list_start_skips_directory_discovery_and_outside_git_opens_home() {
    let f = support::Fixture::create();
    let outside = tempfile::tempdir().unwrap();
    for (start, directory) in [
        (crate::Start::Repositories, f.repo.as_path()),
        (
            crate::Start::Repositories,
            std::path::Path::new("/missing/directory"),
        ),
        (crate::Start::CurrentDirectory, outside.path()),
    ] {
        let entry = start.in_directory(directory, &Cancel::default()).unwrap();
        assert_eq!(entry, crate::startup::Entry::Repositories);
        let (_home, old) = fixture(vec![f.repo.clone()]);
        let mut s = entry
            .open((*old.service).clone(), &Cancel::default())
            .unwrap();
        assert_eq!(s.page, Page::Repositories);
        assert!(s.workspace.path().is_none());
        assert!(s.history.is_empty());
        s.shutdown();
    }
}

#[test]
fn session_repository_observations_survive_reload_without_joining_the_repository_list() {
    let f = support::Fixture::create();
    let (_home, old) = fixture(vec![]);
    let config = std::fs::read(old.service.store.config_path()).unwrap();
    let entry = crate::Start::CurrentDirectory
        .in_directory(&f.repo, &Cancel::default())
        .unwrap();
    let mut s = entry
        .open((*old.service).clone(), &Cancel::default())
        .unwrap();
    s.service
        .check_repository(&f.repo, &Cancel::default(), |_| {})
        .unwrap();
    s.reload(&Cancel::default()).unwrap();
    assert!(s.latest_update(&f.repo).is_some());
    assert!(s.rows().is_empty());
    assert_eq!(s.scope_path(), Some(f.repo));
    assert_eq!(
        std::fs::read(s.service.store.config_path()).unwrap(),
        config
    );
    s.shutdown();
}
