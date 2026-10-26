use super::browser::{ready, select, session};
use super::*;
use crate::{
    command::{
        completion::Menu,
        options::{Kind, Value},
    },
    diff::Preference,
};

#[test]
fn wrap_setting_has_localized_choices_and_persists_without_confirmation() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        let (_home, mut s) = fixture(vec![]);
        assert!(!s.config.text_wrap);
        assert_eq!(
            Kind::TextWrap.command().usage(),
            "settings wrap [true|false]"
        );
        command(&mut s, "settings wrap");
        let bar::InputBar::Command {
            menu: Menu::Options(options),
            ..
        } = &s.bar
        else {
            panic!("wrap options")
        };
        assert_eq!(options.options.len(), 2);
        assert_eq!(options.current, Some(Value::TextWrap(false)));
        key(&mut s, KeyCode::Up);
        key(&mut s, KeyCode::Enter);
        wait(&mut s);
        assert!(s.config.text_wrap && s.service.store.config().unwrap().text_wrap);
        command(&mut s, "settings");
        let text = render(&mut s, 100, 36);
        assert!(text.contains(i18n::tr!(SettingsDisplay)) && text.contains(i18n::tr!(TextWrap)));
        command(&mut s, "settings wrap false");
        wait(&mut s);
        assert!(!s.config.text_wrap && !s.service.store.config().unwrap().text_wrap);
        s.shutdown();
    }
}

#[test]
fn browser_wraps_cached_source_with_clickable_full_links_and_no_horizontal_scroll() {
    let f = support::Fixture::create();
    let url = format!("https://example.invalid/{}/complete", "segment/".repeat(15));
    f.commit(
        "long.ts",
        &format!("// {} {url} TAIL_TOKEN\n", "中文e\u{301}👩‍💻 ".repeat(18)),
    );
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    command(&mut s, "repo files");
    ready(&mut s);
    select(&mut s, "long.ts");
    let before = render(&mut s, 120, 36);
    assert!(!before.contains("TAIL_TOKEN"));
    assert!(session(&mut s).pane().position.horizontal.overflowing());
    let original = match &session(&mut s).pane().preview {
        crate::browser::Preview::Ready { document, .. } => document.clone(),
        _ => panic!("ready document"),
    };
    command(&mut s, "settings wrap true");
    wait(&mut s);
    let wrapped = render(&mut s, 120, 36);
    assert!(wrapped.contains("TAIL_TOKEN"), "{wrapped}");
    assert!(!session(&mut s).pane().position.horizontal.overflowing());
    assert!(
        matches!(&session(&mut s).pane().preview, crate::browser::Preview::Ready { document, .. } if Arc::ptr_eq(document, &original))
    );
    let linked_rows = s
        .pointer
        .links
        .cells
        .iter()
        .filter(|(_, target)| target.uri() == url)
        .map(|((y, _), _)| *y)
        .collect::<BTreeSet<_>>();
    assert!(linked_rows.len() >= 2, "{wrapped}");
    for (width, height) in [(30, 12), (0, 0), (1, 1), (120, 36)] {
        render(&mut s, width, height);
    }
    command(&mut s, "settings wrap false");
    wait(&mut s);
    assert!(!render(&mut s, 120, 36).contains("TAIL_TOKEN"));
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    s.shutdown();
}

#[test]
fn diff_wraps_in_both_layouts_while_exports_and_index_stay_unchanged() {
    for preference in [Preference::Unified, Preference::Split] {
        let f = support::Fixture::create();
        let prefix = "中文e\u{301}👩‍💻 ".repeat(12);
        f.commit("long.rs", &format!("// {prefix}OLDTAIL\n"));
        support::write(f.repo.join("long.rs"), &format!("// {prefix}NEWTAIL\n"));
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        let index = std::fs::read(f.repo.join(".git/index")).unwrap();
        command(&mut s, "repo diff");
        wait(&mut s);
        maximize_preview(&mut s);
        s.diff_preference = preference;
        assert!(!render(&mut s, 100, 36).contains("NEWTAIL"));
        let export = match &s.mode {
            Mode::Changes(changes) => match changes.content() {
                crate::document::View::Text(view) => match &view.export {
                    crate::diff::Export::Complete(text) => text.clone(),
                    _ => panic!("complete patch"),
                },
                _ => panic!("text diff"),
            },
            _ => panic!("changes"),
        };
        command(&mut s, "settings wrap true");
        wait(&mut s);
        let text = render(&mut s, 100, 36);
        assert!(
            text.contains("OLDTAIL") && text.contains("NEWTAIL"),
            "{text}"
        );
        let Mode::Changes(changes) = &s.mode else {
            panic!("changes")
        };
        let crate::document::View::Text(view) = changes.content() else {
            panic!("text diff")
        };
        assert!(!view.horizontal.overflowing());
        assert!(matches!(&view.export, crate::diff::Export::Complete(text) if **text == *export));
        for (width, height) in [(1, 1), (0, 0), (65, 24), (180, 36)] {
            render(&mut s, width, height);
        }
        assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
        s.shutdown();
    }
}
