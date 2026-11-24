use super::*;
#[cfg(unix)]
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

#[cfg(unix)]
#[test]
fn platform_handoff_preserves_arguments_and_reports_a_later_helper_failure() {
    let tmp = tempfile::tempdir().unwrap();
    let target = Target::path(tmp.path().join("space 中文 $(echo nope)")).unwrap();
    let mut pending = launch_command(
        Path::new("/bin/sh"),
        &[
            "-c".into(),
            r#"printf 'received <%s>' "$1" >&2; exit 7"#.into(),
            "opener".into(),
            target.argument(),
        ],
        None,
        target.label(),
    )
    .unwrap();
    let started = std::time::Instant::now();
    loop {
        match pending.poll() {
            Err(error) => {
                assert!(
                    error
                        .to_string()
                        .contains(&format!("received <{}> (exit status: 7)", target.label()))
                );
                break;
            }
            Ok(None) => {}
            Ok(Some(())) => panic!("must report helper exit"),
        };
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

#[test]
fn a_directory_target_never_opens_a_regular_file() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("removed directory");
    let target = Target::directory(path.clone()).unwrap();
    let missing = launch(&target, &Cancel::default()).unwrap_err();
    assert!(missing.to_string().contains(&target.label()));

    std::fs::write(&path, "regular file").unwrap();
    let replaced = launch(&target, &Cancel::default()).unwrap_err();
    assert!(replaced.to_string().contains(i18n::tr!(FolderInvalidPath)));
}
