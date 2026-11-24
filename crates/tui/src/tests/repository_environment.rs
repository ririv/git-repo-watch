use super::*;
use test_support::Fixture;

#[test]
fn repository_environment_page_edits_the_captured_worktree_and_keeps_variable_values_out_of_history()
 {
    let first = Fixture::create();
    let second = Fixture::create();
    let (_home, mut s) = fixture(vec![first.repo.clone(), second.repo.clone()]);
    s.focus(Some(first.repo.clone()));
    command(&mut s, "repo env");
    assert_eq!(
        s.page,
        Page::Configuration(Configuration::RepositoryEnvironment)
    );
    assert!(matches!(&s.mode, Mode::RepositoryEnvironment(path) if path == &first.repo));
    key(&mut s, KeyCode::Enter);
    s.paste(".venv/bin");
    key(&mut s, KeyCode::Enter);
    assert!(matches!(s.mode, Mode::Confirm { .. }));
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(matches!(&s.mode, Mode::RepositoryEnvironment(path) if path == &first.repo));
    assert!(
        render(&mut s, 90, 25)
            .replace('\\', "/")
            .contains(".venv/bin")
    );
    s.focus(Some(second.repo.clone()));
    command(&mut s, "repo env set 'GW_TEST_VALUE=fixture value'");
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    let config = s.service.store.config().unwrap();
    let environment = &config.repository_environment[&first.repo];
    assert_eq!(environment.path_prepend, [PathBuf::from(".venv/bin")]);
    assert_eq!(environment.variables["GW_TEST_VALUE"], "fixture value");
    assert!(!config.repository_environment.contains_key(&second.repo));
    assert!(!format!("{:?}", s.service.store.command_history().unwrap()).contains("fixture value"));
    s.shutdown();
}

#[test]
fn reopening_repository_environment_displays_persisted_absolute_path_directories_in_order() {
    use engine::{git_environment::Edit, service::ConfigEdit};
    let _language = i18n::using(i18n::Language::English);
    let f = Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo env");
    assert!(render(&mut s, 160, 25).contains("Use inherited PATH"));
    key(&mut s, KeyCode::Esc);
    s.service
        .configure(ConfigEdit::RepositoryEnvironment {
            path: f.repo.clone(),
            edit: Edit::Path(vec![
                ".venv/bin".into(),
                fixture_path!("/example/shared-tools").into(),
            ]),
        })
        .unwrap();
    assert!(s.config.repository_environment.is_empty());
    command(&mut s, "repo env");
    let page = render(&mut s, 160, 25);
    let relative = f.repo.join(".venv/bin");
    assert!(
        page.find(relative.to_str().unwrap()).unwrap()
            < page.find(fixture_path!("/example/shared-tools")).unwrap(),
        "{page}"
    );
    key(&mut s, KeyCode::Enter);
    assert_eq!(
        s.bar.text(),
        format!(
            "/repo env path .venv/bin {}",
            fixture_path!("/example/shared-tools")
        )
    );
    s.shutdown();
}
