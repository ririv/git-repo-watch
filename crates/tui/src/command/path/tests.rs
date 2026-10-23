use super::super::ArgumentAction;
use super::*;
use tempfile::TempDir;

fn complete(editor: &mut Editor) -> Result<Menu> {
    super::complete(editor, Path::new("/example/user-home"))
}

fn directories(names: &[&str]) -> TempDir {
    let temp = tempfile::tempdir().unwrap();
    for name in names {
        fs::create_dir(temp.path().join(name)).unwrap();
    }
    temp
}
fn editor(temp: &TempDir, argument: &str) -> Editor {
    Editor::new(format!("/repos add {}/{argument}", temp.path().display()))
}
fn parsed(editor: &Editor) -> String {
    let (_, value) = super::super::argument_input(editor.text.strip_prefix('/').unwrap()).unwrap();
    ArgumentAction::Add
        .parse(value, Path::new("/example/user-home"))
        .unwrap()
}

#[test]
fn unique_directory_completes_literal_spaces_unicode_and_shell_metacharacters() {
    let name = "中文 仓库 [x] $(literal) '单双引号\" ";
    let temp = directories(&[name]);
    fs::write(temp.path().join("中文 文件"), "ignored").unwrap();
    let mut input = editor(&temp, "中文");
    assert!(matches!(complete(&mut input).unwrap(), Menu::Commands));
    let expected = format!("{}/{name}/", temp.path().display());
    assert_eq!(parsed(&input), expected);
    assert_eq!(input.cursor, input.text.len());
    assert!(Path::new(&parsed(&input)).is_dir());
}

#[test]
fn quoted_paths_keep_the_quote_style_and_cursor_inside_the_quotes() {
    let temp = directories(&["中文 with spaces"]);
    for quote in ['\'', '"'] {
        for closing in ["", &quote.to_string()] {
            let mut input = Editor::new(format!(
                "/repos scan {quote}{}/中{closing}",
                temp.path().display()
            ));
            complete(&mut input).unwrap();
            assert_eq!(
                input.text,
                format!(
                    "/repos scan {quote}{}/中文 with spaces/{quote}",
                    temp.path().display()
                )
            );
            assert_eq!(input.cursor, input.text.len() - 1);
            assert_eq!(
                parsed(&input),
                format!("{}/中文 with spaces/", temp.path().display())
            );
            input.insert("子目录");
            assert!(input.text.ends_with(&format!("/子目录{quote}")));
        }
    }
}

#[test]
fn ambiguous_names_use_a_unicode_common_prefix_and_sorted_wrapping_choices() {
    let temp = directories(&["中文目录", "中午目录", "中文另一个"]);
    let mut input = editor(&temp, "中");
    let Menu::Directories(mut menu) = complete(&mut input).unwrap() else {
        panic!("ambiguous directories")
    };
    assert_eq!(input.text, editor(&temp, "中").text);
    assert_eq!(menu.names, ["中午目录", "中文另一个", "中文目录"]);
    assert_eq!(menu.selected, 0);
    menu.advance(true);
    assert_eq!(menu.selected, 2);
    menu.advance(false);
    assert_eq!(menu.selected, 0);
    assert_eq!(
        parsed(&menu.choose()),
        format!("{}/中午目录/", temp.path().display())
    );
}

#[test]
fn common_prefix_extends_without_selecting_and_does_not_split_a_character() {
    let temp = directories(&["开发 分支甲", "开发 分支乙"]);
    let mut input = editor(&temp, "开");
    assert!(matches!(
        complete(&mut input).unwrap(),
        Menu::Directories(_)
    ));
    assert_eq!(input.text, editor(&temp, "开发 分支").text);
    assert_eq!(input.cursor, input.text.len());
}

#[test]
fn cursor_completion_replaces_only_the_current_component_and_preserves_the_suffix() {
    let temp = directories(&["中文 目录"]);
    let mut input = editor(&temp, "中wrong/保留 后缀");
    input.cursor = input.text.find("wrong").unwrap();
    complete(&mut input).unwrap();
    assert_eq!(input.text, editor(&temp, "中文 目录/保留 后缀").text);
    assert_eq!(&input.text[input.cursor..], "保留 后缀");
    assert!(input.text.is_char_boundary(input.cursor));
}

#[test]
fn hidden_directories_require_a_dot_prefix_and_children_are_not_traversed() {
    let temp = directories(&["visible", ".hidden"]);
    fs::create_dir(temp.path().join("visible/nested")).unwrap();
    let mut input = editor(&temp, "");
    complete(&mut input).unwrap();
    assert_eq!(
        parsed(&input),
        format!("{}/visible/", temp.path().display())
    );
    let mut input = editor(&temp, ".");
    complete(&mut input).unwrap();
    assert_eq!(
        parsed(&input),
        format!("{}/.hidden/", temp.path().display())
    );
    let mut nested = editor(&temp, "visible/");
    complete(&mut nested).unwrap();
    assert_eq!(
        parsed(&nested),
        format!("{}/visible/nested/", temp.path().display())
    );
}

#[test]
fn errors_preserve_the_editor_and_non_path_arguments_do_not_touch_the_filesystem() {
    let temp = directories(&[]);
    fs::write(temp.path().join("file"), "content").unwrap();
    for argument in ["absent/", "file/", "nomatch"] {
        let mut input = editor(&temp, argument);
        let before = input.text.clone();
        assert!(complete(&mut input).is_err());
        assert_eq!(input.text, before);
        assert_eq!(input.cursor, before.len());
    }
    for text in [
        "/settings concurrency 4",
        "/schedule interval 6h",
        "/repos add",
    ] {
        let mut input = Editor::new(text.into());
        assert!(matches!(complete(&mut input).unwrap(), Menu::Commands));
        assert_eq!(input.text, text);
    }
}

#[test]
fn empty_path_starts_at_root_but_relative_paths_are_rejected() {
    let input = PathInput::at(
        &Editor::new("/repos add ".into()),
        Path::new("/example/user-home"),
    )
    .unwrap()
    .unwrap();
    assert_eq!(input.parent, "/");
    assert!(input.prefix.is_empty());
    assert!(
        PathInput::at(
            &Editor::new("/repos add relative".into()),
            Path::new("/example/user-home")
        )
        .is_err()
    );
    let mut editor = Editor::new("/repos add /".into());
    editor.cursor -= 1;
    assert!(
        PathInput::at(&editor, Path::new("/example/user-home"))
            .unwrap()
            .is_none()
    );
}

#[cfg(unix)]
#[test]
fn directory_symlinks_complete_but_files_and_broken_links_do_not() {
    use std::os::unix::fs::symlink;
    let temp = directories(&["target"]);
    symlink(temp.path().join("target"), temp.path().join("linked")).unwrap();
    symlink(temp.path().join("missing"), temp.path().join("broken")).unwrap();
    fs::write(temp.path().join("file"), "content").unwrap();
    symlink(temp.path().join("file"), temp.path().join("linked-file")).unwrap();
    let mut input = editor(&temp, "link");
    complete(&mut input).unwrap();
    assert_eq!(parsed(&input), format!("{}/linked/", temp.path().display()));
    assert!(complete(&mut editor(&temp, "bro")).is_err());
}

#[test]
fn control_characters_in_directory_names_are_reported_without_changing_input() {
    let temp = directories(&["line\nbreak"]);
    let mut input = editor(&temp, "line");
    let before = input.text.clone();
    let error = complete(&mut input).unwrap_err();
    assert!(
        error
            .to_string()
            .contains(i18n::tr!(UneditableDirectoryName))
    );
    assert_eq!(input.text, before);
}

// APFS 不接受非 UTF-8 文件名；Linux 文件系统中验证这类实际可创建的目录。
#[cfg(target_os = "linux")]
#[test]
fn non_utf8_directory_names_are_reported_without_lossy_conversion() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let temp = directories(&[]);
    fs::create_dir(
        temp.path()
            .join(OsString::from_vec(b"invalid\xff".to_vec())),
    )
    .unwrap();
    let mut input = editor(&temp, "invalid");
    let before = input.text.clone();
    let error = complete(&mut input).unwrap_err();
    assert!(
        error
            .to_string()
            .contains(i18n::tr!(UneditableDirectoryName))
    );
    assert_eq!(input.text, before);
}

#[test]
fn a_space_in_the_typed_prefix_is_literal_even_before_closing_a_quote() {
    let temp = directories(&["work", "work space"]);
    for quote in ["", "\"", "'"] {
        let mut input = Editor::new(format!("/repos add {quote}{}/work ", temp.path().display()));
        assert!(matches!(complete(&mut input).unwrap(), Menu::Commands));
        assert_eq!(
            parsed(&input),
            format!("{}/work space/", temp.path().display())
        );
    }
}

#[test]
fn tilde_completion_keeps_the_short_form_and_quotes_but_looks_in_the_explicit_home() {
    let temp = directories(&["中文 目录", "Proj"]);
    for (typed, completed) in [("~/中", "~/中文 目录/"), ("~/Pr", "~/Proj/")] {
        for quote in ["", "'", "\""] {
            let mut input = Editor::new(format!("/repos add {quote}{typed}{quote}"));
            assert!(matches!(
                super::complete(&mut input, temp.path()).unwrap(),
                Menu::Commands
            ));
            assert_eq!(input.text, format!("/repos add {quote}{completed}{quote}"));
            assert_eq!(input.cursor, input.text.len() - quote.len());
            let (_, value) =
                super::super::argument_input(input.text.strip_prefix('/').unwrap()).unwrap();
            let expanded = ArgumentAction::Add.parse(value, temp.path()).unwrap();
            assert_eq!(
                Path::new(&expanded),
                temp.path().join(completed.strip_prefix("~/").unwrap())
            );
        }
    }
}

#[test]
fn bare_tilde_and_ambiguous_components_complete_from_home_without_losing_the_suffix() {
    let temp = directories(&["仓库 甲", "仓库 乙"]);
    for typed in ["~", "~/"] {
        let mut input = Editor::new(format!("/repos scan {typed}"));
        let Menu::Directories(menu) = super::complete(&mut input, temp.path()).unwrap() else {
            panic!("ambiguous home children")
        };
        assert_eq!(input.text, "/repos scan ~/仓库 ");
        assert!(menu.choose().text.starts_with("/repos scan ~/仓库 "));
    }
    let mut input = Editor::new("/repos scan \"~/仓wrong/保留 后缀\"".into());
    input.cursor = input.text.find("wrong").unwrap();
    let Menu::Directories(menu) = super::complete(&mut input, temp.path()).unwrap() else {
        panic!("ambiguous path component")
    };
    let chosen = menu.choose();
    assert_eq!(chosen.text, "/repos scan \"~/仓库 乙/保留 后缀\"");
    assert_eq!(&chosen.text[chosen.cursor..], "保留 后缀\"");
    assert!(chosen.text.is_char_boundary(chosen.cursor));
}

#[test]
fn tilde_errors_preserve_the_draft_instead_of_using_another_directory() {
    let temp = directories(&[]);
    for typed in ["~someone/Proj", "~someone", "~/missing/", "~/unknown"] {
        let mut input = Editor::new(format!("/repos import {typed}"));
        let before = input.text.clone();
        assert!(super::complete(&mut input, temp.path()).is_err());
        assert_eq!(input.text, before);
    }
}

#[test]
fn tab_before_the_home_separator_does_not_replace_the_home_marker_or_the_suffix() {
    let temp = directories(&["Proj"]);
    let mut input = Editor::new("/repos add ~/Proj".into());
    input.cursor = input.text.find("~/").unwrap() + 1;
    let before = input.text.clone();
    assert!(matches!(
        super::complete(&mut input, temp.path()).unwrap(),
        Menu::Commands
    ));
    assert_eq!(input.text, before);
}
