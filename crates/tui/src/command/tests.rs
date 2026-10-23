use super::*;

fn names(input: &str) -> Vec<&'static str> {
    matches(input).iter().map(|command| command.name).collect()
}

#[test]
fn prefixes_include_all_descendants_without_a_trailing_space() {
    let expected = [
        "diff",
        "diff stage",
        "diff unstage",
        "diff layout",
        "diff layout auto",
        "diff layout split",
        "diff layout unified",
    ];
    for input in ["di", "diff", "  diff"] {
        assert_eq!(names(input), expected);
    }
    assert_eq!(names("diff "), expected[1..]);
    assert_eq!(names("diff lay"), expected[3..]);
    assert_eq!(names("diff layout"), expected[3..]);
    assert_eq!(names("diff layout "), expected[4..]);
    assert_eq!(names("diff layout s"), ["diff layout split"]);
    assert!(names("diff unknown").is_empty());
}

#[test]
fn root_menu_stays_compact_and_each_group_defaults_to_its_parent() {
    assert!(names("").iter().all(|name| !name.contains(' ')));
    assert_eq!(names("   "), names(""));
    for (index, command) in COMMANDS.iter().enumerate() {
        let children = COMMANDS
            .iter()
            .enumerate()
            .filter(|(_, child)| child.name.starts_with(&format!("{} ", command.name)))
            .collect::<Vec<_>>();
        if children.is_empty() {
            continue;
        }
        assert!(children.iter().all(|(child_index, _)| *child_index > index));
        let candidates = matches(command.name);
        assert_eq!(candidates[0].action, command.action);
        assert_eq!(candidates.len(), children.len() + 1);
    }
    assert_eq!(names("repo r"), ["repo remove"]);
    assert_eq!(
        names("schedule a"),
        ["schedule anchor", "schedule autopush", "schedule apply"]
    );
    assert_eq!(names("settings st"), ["settings startup-check"]);
}

#[test]
fn argument_values_remain_arguments_when_they_contain_spaces_or_command_names() {
    for value in [
        "",
        "~/项目",
        "\"/example/中文 项目/diff layout\"",
        "/example/repo add",
    ] {
        let input = format!("repo add {value}");
        assert_eq!(names(&input), ["repo add"]);
        assert_eq!(argument_input(&input), Some((ArgumentAction::Add, value)));
    }
    assert_eq!(names("schedule interval 6h"), ["schedule interval"]);
}
