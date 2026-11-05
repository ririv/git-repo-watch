use super::*;
use crate::{
    command::{completion::Menu, loader::Request},
    external_editor::Controller,
};
fn wait_editor(s: &mut Screen) {
    let start = Instant::now();
    while s.external_editor.pending() {
        s.poll_external_editor();
        assert!(start.elapsed() < Duration::from_secs(8));
        thread::sleep(Duration::from_millis(5));
    }
    s.poll_external_editor();
}
#[test]
fn editor_completion_prefers_explicit_identity_over_current_command_details() {
    use crate::command::{
        Input,
        values::{Candidates, Choice, Kind},
    };
    let choices = vec![
        Choice {
            token: "git".into(),
            label: "✓ 跟随 Git".into(),
            description: "code --reuse-window".into(),
        },
        Choice {
            token: "code".into(),
            label: "Visual Studio Code".into(),
            description: "/usr/local/bin/code".into(),
        },
    ];
    let empty = Input::global(Editor::new("/settings editor ".into()));
    let explicit = Input::global(Editor::new("/settings editor code".into()));
    let mut values = Candidates::new(Kind::Editor, choices.clone(), &empty).unwrap();
    values.select_token("git");
    values.refine(&explicit).unwrap();
    assert_eq!(values.choose().unwrap().text, "/settings editor code");
    let values = Candidates::new(Kind::Editor, choices.clone(), &explicit).unwrap();
    assert_eq!(values.choose().unwrap().text, "/settings editor code");
    let custom = Input::global(Editor::new("/settings editor code --reuse-window".into()));
    assert!(Candidates::new(Kind::Editor, choices, &custom).is_none());
}
#[test]
fn editor_commands_open_captured_targets_without_leaving_the_current_view() {
    let f = support::Fixture::create();
    f.commit("space 中文.ts", "export const value = 1;\n");
    std::fs::create_dir(f.repo.join("目录")).unwrap();
    f.commit("目录/file.ts", "export const value = 1;\n");
    support::write(f.repo.join("space 中文.ts"), "export const value = 2;\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let opened = Arc::new(std::sync::Mutex::new(Vec::new()));
    let captured = opened.clone();
    s.external_editor = Controller::with_sink(move |target, preference| {
        captured
            .lock()
            .unwrap()
            .push((target.to_owned(), preference.clone()));
        Ok(())
    });
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    command(&mut s, "editor");
    wait_editor(&mut s);
    assert_eq!(s.page, Page::Repositories);
    assert!(matches!(s.mode, Mode::Browsing));
    assert_eq!(opened.lock().unwrap()[0].0, f.repo);
    command(&mut s, "repo");
    wait(&mut s);
    command(&mut s, "editor");
    wait_editor(&mut s);
    assert_eq!(s.page, Page::Detail);
    assert_eq!(opened.lock().unwrap()[1].0, f.repo);
    command(&mut s, "repo files");
    wait(&mut s);
    browser::ready(&mut s);
    browser::select(&mut s, "space 中文.ts");
    command(&mut s, "editor");
    wait_editor(&mut s);
    assert!(matches!(s.mode, Mode::Browser(_)));
    assert_eq!(opened.lock().unwrap()[2].0, f.repo.join("space 中文.ts"));
    browser::select(&mut s, "目录");
    command(&mut s, "editor");
    wait_editor(&mut s);
    assert_eq!(opened.lock().unwrap()[3].0, f.repo.join("目录"));
    command(&mut s, "repo diff");
    wait(&mut s);
    command(&mut s, "editor");
    wait_editor(&mut s);
    assert!(matches!(s.mode, Mode::Changes(_)));
    assert_eq!(opened.lock().unwrap()[4].0, f.repo.join("space 中文.ts"));
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    s.shutdown();
}
#[test]
fn editor_setting_is_one_parameter_command_with_detected_candidates_and_persistence() {
    let _language = i18n::using(i18n::Language::Chinese);
    let (_home, mut s) = fixture(vec![]);
    command(&mut s, "settings");
    command(&mut s, "settings editor");
    assert!(matches!(
        &s.bar,
        bar::InputBar::Command {
            menu: Menu::Loading(Request::Editors { path: None }),
            ..
        }
    ));
    let start = Instant::now();
    while s.command_reader.pending() {
        s.poll_command_completion();
        assert!(start.elapsed() < Duration::from_secs(8));
        thread::sleep(Duration::from_millis(5));
    }
    s.poll_command_completion();
    assert!(
        matches!(&s.bar, bar::InputBar::Command { menu: Menu::Values(values), .. } if values.entries().any(|entry| entry.token == "git"))
    );
    assert!(render(&mut s, 120, 36).contains("Enter 应用"));
    for (w, h) in [(120, 36), (44, 18), (1, 1)] {
        render(&mut s, w, h);
    }
    command(&mut s, "settings editor code");
    wait(&mut s);
    assert_eq!(s.config.editor.value(), "code");
    assert_eq!(s.service.store.config().unwrap().editor.value(), "code");
    command(&mut s, "settings");
    let text = render(&mut s, 120, 45);
    assert!(text.contains("Visual Studio Code"));
    command(&mut s, "settings editor git");
    wait(&mut s);
    assert_eq!(s.config.editor, engine::editor::Preference::Git);
    s.shutdown();
}
#[test]
fn historical_diffs_do_not_offer_live_file_editing() {
    let f = support::Fixture::create();
    f.commit("file", "content\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    let context = s.command_context();
    assert!(
        context
            .unwrap()
            .matches("")
            .iter()
            .any(|command| command.action == Action::OpenEditor)
    );
    assert!(
        s.command_availability(Action::OpenEditor, Some(&f.repo), &s.copy_selection())
            .reason()
            .is_some()
    );
    s.paste("//editor");
    assert_eq!(s.search_commands().unwrap()[0].name, "editor");
    let draft = s.bar.text();
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.bar.text(), draft);
    assert!(!s.external_editor.pending());
    s.shutdown();
}

#[test]
fn editor_uses_the_target_captured_when_the_global_command_opens() {
    let f = support::Fixture::create();
    f.commit("first 中文.ts", "first\n");
    f.commit("second.ts", "second\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let opened = Arc::new(std::sync::Mutex::new(Vec::new()));
    let captured = opened.clone();
    s.external_editor = Controller::with_sink(move |target, _| {
        captured.lock().unwrap().push(target.to_owned());
        Ok(())
    });
    command(&mut s, "repo files");
    browser::ready(&mut s);
    browser::select(&mut s, "first 中文.ts");
    s.paste("//editor");
    browser::select(&mut s, "second.ts");
    let bar::InputBar::Command {
        path, selection, ..
    } = &s.bar
    else {
        panic!("command draft")
    };
    assert_eq!(
        s.command_target(Action::OpenEditor, path.as_deref(), selection)
            .as_deref(),
        Some("first 中文.ts")
    );
    let text = render(&mut s, 120, 36);
    assert!(text.contains("/editor"));
    assert!(text.contains("first 中文.ts"));
    key(&mut s, KeyCode::Enter);
    wait_editor(&mut s);
    assert_eq!(*opened.lock().unwrap(), vec![f.repo.join("first 中文.ts")]);
    assert!(matches!(s.mode, Mode::Browser(_)));
    s.shutdown();
}

#[test]
fn editor_has_no_target_in_settings_or_an_empty_file_search() {
    let f = support::Fixture::create();
    f.commit("file.ts", "content\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "settings");
    assert!(
        s.command_availability(Action::OpenEditor, Some(&f.repo), &s.copy_selection())
            .reason()
            .is_some()
    );
    command(&mut s, "repo files");
    browser::ready(&mut s);
    s.paste("missing-file");
    browser::ready(&mut s);
    s.paste("//editor");
    let bar::InputBar::Command {
        path, selection, ..
    } = &s.bar
    else {
        panic!("command draft")
    };
    assert!(
        s.command_availability(Action::OpenEditor, path.as_deref(), selection)
            .reason()
            .is_some()
    );
    let draft = s.bar.text();
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.bar.text(), draft);
    assert!(!s.external_editor.pending());
    assert!(s.repository_tasks.is_empty());
    s.shutdown();
}
