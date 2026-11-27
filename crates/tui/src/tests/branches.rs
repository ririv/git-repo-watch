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
        command(&mut s, "repo branch filter remote");
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
    focus_next(&mut s);
    assert_eq!(Section::focused(&s.mode), Some(Section::Overview));
    focus_next(&mut s);
    assert_eq!(Section::of(&s.mode), Section::Branches);
    command(&mut s, "repo branch filter local");
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
        zoom(&mut s);
        render(&mut s, width, height);
        assert_eq!(
            s.workspace_areas,
            vec![(s.body_area.unwrap(), Section::Branches)]
        );
        zoom(&mut s);
    }
    focus_next(&mut s);
    assert_eq!(Section::of(&s.mode), Section::Worktree);
    command(&mut s, "repo branch");
    assert!(matches!(&s.mode,Mode::Branches(view) if view.session().filter==BranchFilter::Local));
    assert_eq!(s.page, Page::View(crate::navigation::View::Branches));
    render(&mut s, 120, 32);
    assert!(s.workspace_areas.is_empty());
    key(&mut s, KeyCode::Enter);
    assert!(matches!(&s.mode,Mode::Branches(view) if matches!(view.as_ref(),View::Detail{..})));
    key(&mut s, KeyCode::Esc);
    key(&mut s, KeyCode::Esc);
    assert_eq!(s.page, Page::Detail);
    assert_eq!(Section::of(&s.mode), Section::Worktree);
    assert_eq!(
        s.workspace.session().unwrap().arrangement,
        Arrangement::Tiled
    );
    focus_next(&mut s);
    focus_next(&mut s);
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
    wait(&mut s);
    assert_eq!(s.bar.text(), "/comm");
    assert_eq!(s.page, Page::Detail);
    key(&mut s, KeyCode::Esc);
    command(&mut s, "repo branch");
    wait(&mut s);
    support::git(&f.repo, &["branch", "new-branch"]);
    control(&mut s, 'r');
    wait_workspace(&mut s);
    assert!(
        matches!(&s.mode,Mode::Branches(view) if view.session().snapshot.entries.iter().any(|entry|entry.name=="new-branch"))
    );
}

#[test]
fn branch_filter_options_complete_without_saving_and_apply_only_on_enter() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo branch");
    wait(&mut s);
    command(&mut s, "repo branch filter");
    assert_eq!(super::options::choices(&s).options.len(), 3);
    assert_eq!(
        super::options::choices(&s).current,
        Some(crate::command::options::Value::BranchFilter(
            BranchFilter::All
        ))
    );
    key(&mut s, KeyCode::Down);
    key(&mut s, KeyCode::Tab);
    assert!(s.bar.text().ends_with("filter local"));
    assert!(matches!(&s.mode, Mode::Branches(view) if view.session().filter == BranchFilter::All));
    key(&mut s, KeyCode::Enter);
    assert!(
        matches!(&s.mode, Mode::Branches(view) if view.session().filter == BranchFilter::Local)
    );
    for code in [KeyCode::Tab, KeyCode::BackTab] {
        key(&mut s, code);
        assert!(
            matches!(&s.mode, Mode::Branches(view) if view.session().filter == BranchFilter::Local)
        );
    }
    s.shutdown();
}

#[test]
fn long_branch_names_scroll_without_moving_the_selection_or_stealing_the_filter_cursor() {
    let f = support::Fixture::create();
    let name = format!("feature/{}长分支末尾", "long-directory/".repeat(6));
    support::git(&f.repo, &["branch", &name]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo branch");
    wait(&mut s);
    key(&mut s, KeyCode::Down);

    let start = render(&mut s, 64, 22);
    assert!(!start.contains("长分支末尾"), "{start}");
    assert!(start.contains(i18n::tr!(HorizontalScrollHint)), "{start}");
    assert!(
        s.pointer
            .has_horizontal_track(crate::horizontal::Pane::Branches)
    );
    for _ in 0..30 {
        key(&mut s, KeyCode::Right);
    }
    assert!(render(&mut s, 64, 22).contains("长分支末尾"));
    assert!(matches!(&s.mode, Mode::Branches(view)
        if view.session().selected().is_some_and(|entry| entry.name == name)));

    s.paste("feature");
    let cursor = s.bar.editor().cursor;
    let before = match &s.mode {
        Mode::Branches(view) => view.session().horizontal.offset(),
        _ => panic!("branch list expected"),
    };
    key(&mut s, KeyCode::Left);
    assert_eq!(s.bar.editor().cursor, cursor - 1);
    assert!(matches!(&s.mode, Mode::Branches(view)
        if view.session().horizontal.offset() == before));
    key(&mut s, KeyCode::Enter);
    assert!(
        matches!(&s.mode, Mode::Branches(view) if matches!(view.as_ref(), View::Detail { .. }))
    );
}
