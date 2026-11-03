use super::*;
use crate::{
    changes::Preview,
    diff::{Context, HeaderVisibility, Preference, VisualRow},
};

fn preview(s: &mut Screen) -> (&mut crate::diff::View, &mut usize) {
    let Mode::Changes(session) = &mut s.mode else {
        panic!("变更界面")
    };
    let Preview::Ready {
        content, scroll, ..
    } = &mut session.preview
    else {
        panic!("预览就绪")
    };
    let crate::document::View::Text(view) = content.as_mut() else {
        panic!("文本预览")
    };
    (view, scroll)
}

#[test]
fn full_context_partial_staging_retains_source_and_reading_position() {
    for preference in [Preference::Unified, Preference::Split] {
        let f = support::Fixture::create();
        let before = (1..=120).map(|n| format!("line {n}\n")).collect::<String>();
        let after = before
            .replace("line 20\n", "changed 20\n")
            .replace("line 100\n", "changed 100\n");
        f.commit("source.txt", &before);
        support::write(f.repo.join("source.txt"), &after);
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        assert_eq!(s.diff_context, Context::Full);
        command(&mut s, "repo diff");
        wait(&mut s);
        maximize_preview(&mut s);
        s.diff_preference = preference;
        render(&mut s, 180, 40);
        *preview(&mut s).1 = 12;
        render(&mut s, 180, 40);
        let (view, scroll) = preview(&mut s);
        let position = view.position(*scroll).unwrap();
        command(&mut s, "repo diff stage");
        wait(&mut s);
        let screen = render(&mut s, 180, 40);
        assert!(screen.contains("changed 20"), "{screen}");
        let (view, scroll) = preview(&mut s);
        let cache = view.arrange(178, preference, false, &[], scroll, Context::Full);
        assert_eq!(*scroll, cache.locate_position(&position).unwrap());
        let cache = view.layout(
            180,
            Preference::Unified,
            false,
            &[],
            HeaderVisibility::Collapsed,
            Context::Full,
        );
        let (index, line) = cache
            .rows
            .iter()
            .enumerate()
            .find_map(|(index, row)| match row {
                VisualRow::Full { line, .. } if line.to_string().contains("changed 20") => {
                    Some((index, line))
                }
                _ => None,
            })
            .unwrap();
        assert_eq!(line.text.style.bg, None);
        assert_eq!(cache.ids[cache.rows[index].anchors().preferred().row], None);
        assert_eq!(
            support::git(&f.repo, &["show", ":source.txt"]),
            before.replace("line 20\n", "changed 20\n").trim_end()
        );
        assert_eq!(
            std::fs::read_to_string(f.repo.join("source.txt")).unwrap(),
            after
        );
        let Mode::Changes(session) = &s.mode else {
            unreachable!()
        };
        let Preview::Ready {
            editing: crate::changes::edit::State::Ready(editing),
            ..
        } = &session.preview
        else {
            unreachable!()
        };
        assert!(editing.selected().is_empty());
        let staged = support::git(&f.repo, &["diff", "--cached"]);
        command(&mut s, "repo diff stage");
        wait(&mut s);
        assert_eq!(support::git(&f.repo, &["diff", "--cached"]), staged);
        key(&mut s, KeyCode::Down);
        let Mode::Changes(session) = &s.mode else {
            unreachable!()
        };
        let Preview::Ready {
            editing: crate::changes::edit::State::Ready(editing),
            ..
        } = &session.preview
        else {
            unreachable!()
        };
        assert!(!editing.selected().is_empty());

        s.shutdown();
    }
}

#[test]
fn context_choices_complete_persist_and_apply_to_current_and_future_files() {
    use crate::command::{
        completion::Menu,
        options::{Kind, Value},
    };
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        f.commit(
            "file.txt",
            &(1..=60).map(|n| format!("line {n}\n")).collect::<String>(),
        );
        support::write(
            f.repo.join("file.txt"),
            &std::fs::read_to_string(f.repo.join("file.txt"))
                .unwrap()
                .replace("line 40\n", "changed\n"),
        );
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        let store = s.service.store.clone();
        let scheduler = s.service.scheduler.clone();
        assert_eq!(
            Kind::DiffContext.command().usage(),
            "repo diff context [full|changes]"
        );
        command(&mut s, "repo diff context");
        let bar::InputBar::Command {
            menu: Menu::Options(options),
            ..
        } = &s.bar
        else {
            panic!("选项菜单")
        };
        assert_eq!(options.current, Some(Value::DiffContext(Context::Full)));
        assert_eq!(options.options.len(), 2);
        key(&mut s, KeyCode::Down);
        key(&mut s, KeyCode::Enter);
        assert_eq!(s.diff_context, Context::Changes);
        command(&mut s, "repo diff");
        wait(&mut s);
        maximize_preview(&mut s);
        assert!(render(&mut s, 160, 30).contains(crate::diff::context_label(Context::Changes)));
        let original = match &preview(&mut s).0.export {
            crate::diff::Export::Complete(raw) => raw.clone(),
            _ => panic!("完整补丁"),
        };
        command(&mut s, "repo diff context full");
        render(&mut s, 160, 30);
        assert!(
            matches!(&preview(&mut s).0.export, crate::diff::Export::Complete(raw) if raw.as_ref() == original.as_ref())
        );
        command(&mut s, "repo diff context changes");
        s.shutdown();
        let mut reopened = Screen::new(Service::new(store, scheduler), &Cancel::default()).unwrap();
        assert_eq!(reopened.diff_context, Context::Changes);
        command(&mut reopened, "repo diff context full");
        reopened.shutdown();
    }
}

#[test]
fn fully_staged_files_keep_the_same_file_preview() {
    let f = support::Fixture::create();
    f.commit("file.txt", "before\nstable\n");
    support::write(f.repo.join("file.txt"), "after\nstable\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo diff");
    wait(&mut s);
    maximize_preview(&mut s);
    render(&mut s, 160, 32);
    command(&mut s, "repo diff stage");
    wait(&mut s);
    assert!(render(&mut s, 160, 32).contains("after"));
    assert_eq!(
        s.mode
            .file_session()
            .unwrap()
            .current()
            .unwrap()
            .location
            .name(),
        std::path::Path::new("file.txt")
    );
    s.shutdown();
}
