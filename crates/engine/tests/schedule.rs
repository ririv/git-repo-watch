#![cfg(feature = "developer-scheduler")]

#[cfg(unix)]
use engine::scheduler::{BINARY_MARKER, task_marker};
use engine::{
    cancel::Cancel,
    scheduler::{Control, Manager},
    service::Service,
    state::{Config, Store},
};
use std::fs;

fn fixture() -> (tempfile::TempDir, Service) {
    let temp = tempfile::tempdir().unwrap();
    let home = temp.path().join("project");
    fs::create_dir(&home).unwrap();
    let service = Service::new(
        Store::custom(home),
        Manager::for_current_user_in(temp.path().join("user")).unwrap(),
    );
    service.store.save_config(&Config::default()).unwrap();
    (temp, service)
}
#[cfg(unix)]
fn binary(service: &Service, contents: &[u8]) {
    let path = service.scheduler.binary();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}
#[cfg(unix)]
#[test]
fn schedule_controls_reject_another_installation_before_native_commands_or_file_changes() {
    let _language = i18n::using(i18n::Language::English);
    let (_temp, service) = fixture();
    binary(&service, BINARY_MARKER.as_bytes());
    let another = Store::custom(service.store.config_dir().join("another"));
    let path = service.scheduler.definition();
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let contents = format!("# {}\n", task_marker(&another, &service.scheduler.binary()));
    fs::write(&path, &contents).unwrap();
    for operation in [Control::Apply, Control::Disable, Control::Enable] {
        let error = service
            .control_schedule(operation, &Cancel::default())
            .unwrap_err();
        assert!(
            error.to_string().contains("another installation"),
            "{error:#}"
        );
    }
    assert_eq!(fs::read_to_string(path).unwrap(), contents);
    assert_eq!(
        fs::read(service.scheduler.binary()).unwrap(),
        BINARY_MARKER.as_bytes()
    );
    assert!(service.store.report().unwrap().is_none());
}
#[cfg(unix)]
#[test]
fn controls_validate_binary_ownership_and_require_an_existing_definition_for_apply_or_disable() {
    let _language = i18n::using(i18n::Language::English);
    let (_temp, service) = fixture();
    binary(&service, b"unowned");
    assert!(
        service
            .control_schedule(Control::Enable, &Cancel::default())
            .is_err()
    );
    binary(&service, BINARY_MARKER.as_bytes());
    for operation in [Control::Apply, Control::Disable] {
        let error = service
            .control_schedule(operation, &Cancel::default())
            .unwrap_err();
        assert!(
            error.to_string().contains("definition is missing"),
            "{error:#}"
        );
    }
    #[cfg(unix)]
    assert!(!service.scheduler.definition().exists());
}
#[test]
fn controls_share_the_configuration_lock_and_cancel_without_changes() {
    let (_temp, service) = fixture();
    let lock = service.store.lock().unwrap();
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
    #[cfg(unix)]
    assert!(!service.scheduler.definition().exists());
}
