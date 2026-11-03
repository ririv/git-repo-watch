use engine::{
    scheduler::Manager,
    service::{ConfigEdit, Service},
    state::{Store, view::*},
};
use std::{fs, sync::Barrier};

fn service(store: &Store) -> Service {
    let manager = Manager::for_user(store.config_dir(), unsafe { libc::getuid() }).unwrap();
    Service::new(store.clone(), manager)
}

#[test]
fn legacy_configuration_defaults_without_rewriting_and_adds_only_the_changed_preference() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::custom(temp.path().to_owned());
    store.initialize().unwrap();
    let mut document: toml_edit::DocumentMut = fs::read_to_string(store.config_path())
        .unwrap()
        .parse()
        .unwrap();
    document.remove("view");
    let legacy = document.to_string();
    fs::write(store.config_path(), &legacy).unwrap();
    assert_eq!(store.config().unwrap().view, Preferences::default());
    assert_eq!(fs::read_to_string(store.config_path()).unwrap(), legacy);
    service(&store)
        .configure(ConfigEdit::TextWrap(true))
        .unwrap();
    let text = fs::read_to_string(store.config_path()).unwrap();
    assert_eq!(
        text,
        legacy.replace("text_wrap = false", "text_wrap = true")
    );
    service(&store)
        .configure(ConfigEdit::View(Edit::FileLayout(FileLayout::Flat)))
        .unwrap();
    let saved = store.config().unwrap();
    assert_eq!(
        saved.view,
        Preferences {
            file_layout: FileLayout::Flat,
            ..Default::default()
        }
    );
    assert!(saved.text_wrap);
    let document: toml_edit::DocumentMut = fs::read_to_string(store.config_path())
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(document["view"].as_table().unwrap().len(), 1);
}

#[test]
fn field_edits_preserve_comments_and_unrelated_configuration() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::custom(temp.path().to_owned());
    store.initialize().unwrap();
    let original = fs::read_to_string(store.config_path())
        .unwrap()
        .replace(
            "file_layout = \"tree\"",
            "file_layout = 'tree' # 用户的文件列表偏好",
        )
        .replace("[view]", "[view] # 保留表注释");
    fs::write(store.config_path(), &original).unwrap();
    let mut expected = store.config().unwrap();
    service(&store)
        .configure(ConfigEdit::View(Edit::FileLayout(FileLayout::Flat)))
        .unwrap();
    let text = fs::read_to_string(store.config_path()).unwrap();
    assert!(text.contains("# 用户的文件列表偏好") && text.contains("[view] # 保留表注释"));
    assert!(text.contains("# 文件列表：tree 为树形，flat 为平铺。"));
    expected.view.file_layout = FileLayout::Flat;
    assert_eq!(store.config().unwrap(), expected);
    service(&store)
        .configure(ConfigEdit::FileIcons(false))
        .unwrap();
    expected.file_icons = false;
    assert_eq!(store.config().unwrap(), expected);
    let text = fs::read_to_string(store.config_path()).unwrap();
    assert!(text.contains("# 用户的文件列表偏好") && text.contains("[view] # 保留表注释"));
}

#[test]
fn independent_instances_merge_preference_edits_under_the_configuration_lock() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::custom(temp.path().to_owned());
    store.initialize().unwrap();
    let barrier = Barrier::new(5);
    std::thread::scope(|scope| {
        for edit in [
            Edit::FileLayout(FileLayout::Flat),
            Edit::DiffLayout(DiffLayout::Split),
            Edit::DiffHeader(HeaderVisibility::Expanded),
            Edit::RepositoryOrder(RepositoryOrder::Name),
            Edit::DiffContext(DiffContext::Changes),
        ] {
            let service = service(&store);
            let barrier = &barrier;
            scope.spawn(move || {
                barrier.wait();
                service.configure(ConfigEdit::View(edit)).unwrap();
            });
        }
    });
    let reopened = Store::custom(temp.path().to_owned());
    assert_eq!(
        reopened.config().unwrap().view,
        Preferences {
            file_layout: FileLayout::Flat,
            diff_layout: DiffLayout::Split,
            diff_header: HeaderVisibility::Expanded,
            diff_context: DiffContext::Changes,
            repository_order: RepositoryOrder::Name,
        }
    );
}
