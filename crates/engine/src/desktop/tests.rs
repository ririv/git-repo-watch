use super::*;
#[test]
fn local_references_keep_spaces_unicode_and_shell_metacharacters_literal() {
    let base = Path::new("/example/repository");
    let target =
        Target::reference("docs/%E4%B8%AD%E6%96%87%20dir/guide.md#intro", Some(base)).unwrap();
    assert_eq!(target.label(), "/example/repository/docs/中文 dir/guide.md");
    assert_eq!(Target::reference(&target.uri(), None), Some(target));
    for value in [
        "/example/a b/中文",
        "/example/100% #/$(echo nope); & 'quoted'",
    ] {
        let target = Target::path(value.into()).unwrap();
        assert_eq!(target.argument(), OsString::from(value));
        assert_eq!(Target::reference(&target.uri(), None), Some(target));
    }
    assert_eq!(
        Target::reference("https://example.com/path?q=a%20b#part", None)
            .unwrap()
            .argument(),
        OsString::from("https://example.com/path?q=a%20b#part")
    );
}
#[test]
fn malformed_or_nonlocal_references_are_rejected_and_missing_paths_report_the_target() {
    for value in [
        "",
        "https://",
        "https://bad host/",
        "javascript:alert(1)",
        "data:text/plain,a",
        "ssh://host/a",
        "#intro",
        "file://remote.example/a",
        "file:///tmp/%",
        "file:///tmp/%00",
        "file:///tmp/%1B",
        "https://example.com/\x1b]8;;injected",
    ] {
        assert!(
            Target::reference(value, Some(Path::new("/example"))).is_none(),
            "{value:?}"
        );
    }
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("missing 中文");
    let target = Target::path(path.clone()).unwrap();
    let error = launch(&target, &Cancel::default()).unwrap_err().to_string();
    assert!(error.contains(path.to_str().unwrap()));
}

#[test]
fn platform_handoff_preserves_arguments_and_reports_a_later_helper_failure() {
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().unwrap();
    let program = tmp.path().join("opener");
    std::fs::write(&program, "#!/bin/sh\nprintf '%s' \"$1\" >&2\nexit 7\n").unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
    let target = Target::path(tmp.path().join("space 中文 $(echo nope)")).unwrap();
    let mut pending = launch_using(&program, &target).unwrap();
    let started = std::time::Instant::now();
    loop {
        match pending.poll() {
            Err(error) => {
                assert!(error.to_string().contains(&target.label()));
                break;
            }
            Ok(None) => {}
            Ok(Some(())) => panic!("must report helper exit"),
        };
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
