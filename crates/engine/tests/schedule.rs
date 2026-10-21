use engine::{
    cancel::Cancel,
    scheduler::{Control, Location, Manager, digest},
    service::Service,
    state::{Config, Store, write_json},
};
use std::fs;

fn fixture() -> (tempfile::TempDir, Service) {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("project");
    fs::create_dir(&home).unwrap();
    let service = Service::new(
        home.clone(),
        Manager::for_user(temp.path().join("user"), unsafe { libc::getuid() }).unwrap(),
    );
    service
        .store
        .save_config(&Config {
            interval_seconds: 21600,
            repositories: vec![],
            excluded_roots: vec![],
            push_policy: engine::domain::PushPolicy::Disabled,
            max_concurrency: 4,
        })
        .unwrap();
    (temp, service)
}
fn owner(service: &Service, home: std::path::PathBuf, hash: &str) {
    let manager = &service.scheduler;
    fs::create_dir_all(manager.binary().parent().unwrap()).unwrap();
    fs::create_dir_all(manager.location().parent().unwrap()).unwrap();
    fs::write(manager.binary(), b"owned").unwrap();
    write_json(
        &manager.location(),
        &Location {
            home,
            binary_sha256: hash.into(),
        },
    )
    .unwrap();
}
#[test]
fn schedule_controls_reject_another_installation_before_native_commands_or_file_changes() {
    let _language = i18n::using(i18n::Language::English);
    let (_temp, service) = fixture();
    owner(
        &service,
        service.store.home.join("another"),
        &digest(b"owned"),
    );
    let before = fs::read(service.scheduler.location()).unwrap();
    for operation in [Control::Apply, Control::Disable, Control::Enable] {
        let error = service
            .control_schedule(operation, &Cancel::default())
            .unwrap_err();
        assert!(
            error.to_string().contains("another installation"),
            "{error:#}"
        );
    }
    assert_eq!(fs::read(service.scheduler.location()).unwrap(), before);
    assert_eq!(fs::read(service.scheduler.binary()).unwrap(), b"owned");
    assert!(service.store.report().unwrap().is_none());
    assert!(!service.scheduler.definition().exists());
}
#[test]
fn controls_validate_binary_ownership_and_require_an_existing_definition_for_apply_or_disable() {
    let _language = i18n::using(i18n::Language::English);
    let (_temp, service) = fixture();
    owner(&service, service.store.home.clone(), "wrong hash");
    assert!(
        service
            .control_schedule(Control::Enable, &Cancel::default())
            .is_err()
    );
    owner(&service, service.store.home.clone(), &digest(b"owned"));
    for operation in [Control::Apply, Control::Disable] {
        let error = service
            .control_schedule(operation, &Cancel::default())
            .unwrap_err();
        assert!(
            error.to_string().contains("definition is missing"),
            "{error:#}"
        );
    }
    assert!(!service.scheduler.definition().exists());
}
#[test]
fn controls_share_the_check_lock_and_cancel_without_changes() {
    let (_temp, service) = fixture();
    let store = Store {
        home: service.store.home.clone(),
    };
    let lock = store.lock().unwrap();
    assert!(
        service
            .control_schedule(Control::Enable, &Cancel::default())
            .unwrap_err()
            .downcast_ref::<engine::state::Busy>()
            .is_some()
    );
    drop(lock);
    let cancel = Cancel::default();
    cancel.cancel();
    for operation in [Control::Apply, Control::Disable, Control::Enable] {
        assert!(service.control_schedule(operation, &cancel).is_err());
    }
    assert!(!service.scheduler.location().exists());
}
