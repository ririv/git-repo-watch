use super::*;
use engine::state::view::{DiffLayout, FileLayout, Preferences, RepositoryOrder};

#[test]
fn command_and_picker_choices_survive_restart_without_restoring_transient_selections() {
    let f = support::Fixture::create();
    support::write(f.repo.join("中文 空格.rs"), "fn changed() {}\n");
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let store = s.service.store.clone();
    let scheduler = s.service.scheduler.clone();
    command(&mut s, "repo diff");
    wait(&mut s);
    command(&mut s, "repo diff view flat");
    assert_eq!(s.file_layout, crate::files::Layout::Flat);
    command(&mut s, "repo diff layout");
    key(&mut s, KeyCode::Right);
    key(&mut s, KeyCode::Enter);
    assert_eq!(s.diff_preference, crate::diff::Preference::Split);
    key(&mut s, KeyCode::Char(' '));
    assert!(!s.mode.file_session().unwrap().selected.is_empty());
    command(&mut s, "repos");
    command(&mut s, "repos sort name");
    command(&mut s, "repos filter failed");
    s.paste("临时搜索");
    assert!(!matches!(s.mode, Mode::Busy(_)));
    s.shutdown();
    let mut reopened = Screen::new(Service::new(store, scheduler), &Cancel::default()).unwrap();
    assert_eq!(reopened.file_layout, crate::files::Layout::Flat);
    assert_eq!(reopened.diff_preference, crate::diff::Preference::Split);
    assert_eq!(reopened.order, Order::Name);
    assert_eq!(reopened.filter, Filter::All);
    assert!(reopened.query.is_empty());
    command(&mut reopened, "repo diff");
    wait(&mut reopened);
    let files = reopened.mode.file_session().unwrap();
    assert_eq!(reopened.file_layout, crate::files::Layout::Flat);
    assert!(files.selected.is_empty());
}

#[test]
fn rapid_choices_stay_responsive_and_flush_the_latest_value_for_each_field_on_exit() {
    let (_home, mut s) = fixture(vec![]);
    let store = s.service.store.clone();
    let cancel = Cancel::default();
    let lock = store.transaction(&cancel).unwrap();
    for layout in [
        crate::files::Layout::Flat,
        crate::files::Layout::Tree,
        crate::files::Layout::Flat,
    ] {
        command(
            &mut s,
            &format!(
                "repo diff view {}",
                if layout == crate::files::Layout::Flat {
                    "flat"
                } else {
                    "tree"
                }
            ),
        );
        assert_eq!(s.file_layout, layout);
        assert!(matches!(s.mode, Mode::Browsing));
    }
    for layout in ["unified", "auto", "split"] {
        command(&mut s, &format!("repo diff layout {layout}"));
        assert!(matches!(s.mode, Mode::Browsing));
    }
    for order in ["path", "updated", "name"] {
        command(&mut s, &format!("repos sort {order}"));
        assert!(matches!(s.mode, Mode::Browsing));
    }
    assert!(s.view_save.pending());
    assert_eq!(store.config().unwrap().view, Preferences::default());
    drop(lock);
    s.shutdown();
    assert_eq!(
        store.config().unwrap().view,
        Preferences {
            file_layout: FileLayout::Flat,
            diff_layout: DiffLayout::Split,
            repository_order: RepositoryOrder::Name,
        }
    );
}

#[test]
fn dropping_the_screen_finishes_pending_preferences() {
    let (_home, mut s) = fixture(vec![]);
    let store = s.service.store.clone();
    command(&mut s, "repo diff view flat");
    drop(s);
    assert_eq!(store.config().unwrap().view.file_layout, FileLayout::Flat);
}

#[test]
fn saving_failures_are_reported_and_do_not_replace_invalid_configuration() {
    let _language = i18n::using(i18n::Language::Chinese);
    let (_home, mut s) = fixture(vec![]);
    let store = s.service.store.clone();
    let cancel = Cancel::default();
    let lock = store.transaction(&cancel).unwrap();
    command(&mut s, "repo diff view flat");
    std::fs::write(store.config_path(), "invalid toml [").unwrap();
    drop(lock);
    wait(&mut s);
    assert!(
        s.message.starts_with(i18n::tr!(ViewPreferenceSaveFailed)),
        "{}",
        s.message
    );
    assert_eq!(s.file_layout, crate::files::Layout::Flat);
    assert_eq!(
        std::fs::read_to_string(store.config_path()).unwrap(),
        "invalid toml ["
    );
}
