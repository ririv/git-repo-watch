use super::*;
use crate::{
    branches::{Filter as BranchFilter, View},
    workspace::{Arrangement, Section},
};

fn control(s: &mut Screen, c: char) {
    s.key(
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL),
        &Cancel::default(),
    )
    .unwrap();
    wait(s);
}

#[test]
fn branch_command_lists_filters_and_inspects_real_branches_without_changing_the_repository() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        support::git(&f.repo, &["branch", "中文实验"]);
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        let config = std::fs::read(s.service.store.config_path()).unwrap();
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        let head = support::git(&f.repo, &["rev-parse", "HEAD"]);
        s.paste("//bra");
        key(&mut s, KeyCode::Tab);
        assert_eq!(s.bar.text(), "/repo branch");
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert!(matches!(s.mode, Mode::Branches(_)));
        assert!(render(&mut s, 120, 32).contains("origin/main"));
        s.paste("中文");
        assert!(matches!(&s.mode,Mode::Branches(view) if view.session().rows().len()==1));
        key(&mut s, KeyCode::Enter);
        assert!(matches!(&s.mode,Mode::Branches(view) if matches!(view.as_ref(),View::Detail{..})));
        assert!(render(&mut s, 120, 32).contains("refs/heads/中文实验"));
        key(&mut s, KeyCode::Esc);
        key(&mut s, KeyCode::Tab);
        key(&mut s, KeyCode::Tab);
        assert!(
            matches!(&s.mode,Mode::Branches(view) if view.session().filter==BranchFilter::Remote)
        );
        assert_eq!(s.scope_path(), Some(f.repo.clone()));
        assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), head);
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        assert_eq!(
            std::fs::read(s.service.store.config_path()).unwrap(),
            config
        );
        assert!(s.service.store.report().unwrap().is_none());
        assert!(s.service.store.repository_reports().unwrap().is_empty());
    }
}

#[test]
fn four_panes_share_focus_fullscreen_and_preserved_branch_filters() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    key(&mut s, KeyCode::Enter);
    wait_workspace(&mut s);
    super::overview::wait_overview(&mut s);
    control(&mut s, 'd');
    assert_eq!(Section::of(&s.mode), Section::Branches);
    key(&mut s, KeyCode::Tab);
    for (width, height) in [(60, 18), (140, 32), (200, 50)] {
        let text = render(&mut s, width, height);
        assert_eq!(s.workspace_areas.len(), 4);
        for label in [
            i18n::tr!(WorkspaceOverview),
            i18n::tr!(BranchTitle),
            i18n::tr!(Worktree),
            i18n::tr!(WorkspaceHistory),
        ] {
            assert!(text.contains(label), "{label}: {text}");
        }
        control(&mut s, 'f');
        render(&mut s, width, height);
        assert_eq!(
            s.workspace_areas,
            vec![(s.body_area.unwrap(), Section::Branches)]
        );
        control(&mut s, 'f');
    }
    control(&mut s, 'd');
    assert_eq!(Section::of(&s.mode), Section::Worktree);
    command(&mut s, "repo branch");
    assert!(matches!(&s.mode,Mode::Branches(view) if view.session().filter==BranchFilter::Local));
    control(&mut s, 'f');
    key(&mut s, KeyCode::Enter);
    assert!(matches!(&s.mode,Mode::Branches(view) if matches!(view.as_ref(),View::Detail{..})));
    key(&mut s, KeyCode::Esc);
    assert_eq!(
        s.workspace.session().unwrap().arrangement,
        Arrangement::Maximized
    );
    key(&mut s, KeyCode::Esc);
    assert_eq!(
        s.workspace.session().unwrap().arrangement,
        Arrangement::Tiled
    );
    control(&mut s, 'd');
    control(&mut s, 'd');
    control(&mut s, 'd');
    assert_eq!(Section::of(&s.mode), Section::Overview);
}

#[test]
fn branch_refresh_observes_external_changes_and_drafts_survive_passive_loading() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    key(&mut s, KeyCode::Enter);
    command(&mut s, "repo branch");
    s.paste("//comm");
    wait_workspace(&mut s);
    assert_eq!(s.bar.text(), "/comm");
    key(&mut s, KeyCode::Esc);
    support::git(&f.repo, &["branch", "new-branch"]);
    control(&mut s, 'r');
    wait_workspace(&mut s);
    assert!(
        matches!(&s.mode,Mode::Branches(view) if view.session().snapshot.entries.iter().any(|entry|entry.name=="new-branch"))
    );
}
