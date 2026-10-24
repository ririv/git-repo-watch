use super::*;

fn names(input: &str) -> Vec<&'static str> {
    matches(input).iter().map(|command| command.name).collect()
}

#[test]
fn prefixes_include_all_descendants_without_a_trailing_space() {
    let expected = [
        "repo diff",
        "repo diff stage",
        "repo diff unstage",
        "repo diff layout",
        "repo diff layout auto",
        "repo diff layout split",
        "repo diff layout unified",
        "repo diff refresh",
        "repo diff copy",
        "repo diff view",
        "repo diff view tree",
        "repo diff view flat",
    ];
    for input in ["di", "repo diff", "  diff"] {
        assert_eq!(&names(input)[..expected.len()], expected);
    }
    assert_eq!(names("repo diff "), expected[1..]);
    assert_eq!(names("repo diff lay"), expected[3..7]);
    assert_eq!(names("repo diff layout"), expected[3..7]);
    assert_eq!(names("repo diff layout "), expected[4..7]);
    assert_eq!(names("repo diff layout s"), ["repo diff layout split"]);
    assert!(names("repo diff unknown").is_empty());
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
        assert!(candidates.len() > children.len());
        assert!(children.iter().all(|(_, child)| {
            candidates
                .iter()
                .any(|candidate| candidate.name == child.name)
        }));
    }
    assert_eq!(names("repos r"), ["repos refresh", "repos remove"]);
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
        let input = format!("repos add {value}");
        assert_eq!(names(&input), ["repos add"]);
        assert_eq!(argument_input(&input), Some((ArgumentAction::Add, value)));
    }
    assert_eq!(names("schedule interval 6h"), ["schedule interval"]);
}

#[test]
fn chinese_and_english_descriptions_are_searchable_in_either_interface_language() {
    for language in [i18n::Language::English, i18n::Language::Chinese] {
        let _language = i18n::using(language);
        for (query, expected) in [
            ("检查频率", "schedule interval"),
            ("LIVE DIFF", "repo diff"),
            ("CHECK SELECTED", "repo check"),
        ] {
            assert_eq!(names(query), [expected]);
        }
        assert_eq!(
            names("终端"),
            ["repo terminal", "settings interaction text"]
        );
        let staging = names("暂存");
        assert!(staging.contains(&"repo diff stage") && staging.contains(&"repo diff unstage"));
        assert!(names("没有这样的说明文字").is_empty());
        let diff = names("DIFF");
        let group = names("repo diff");
        assert_eq!(&diff[..group.len()], group);
        assert!(diff.contains(&"repo files diff"));
    }
}

#[test]
fn name_matches_precede_description_matches_without_duplicates_or_scope_leaks() {
    let checks = names("check");
    assert_eq!(&checks[..2], ["repo check", "repos check"]);
    assert!(checks.contains(&"schedule interval"));
    assert_eq!(
        checks
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        checks.len()
    );
    assert_eq!(
        &names("stage")[..2],
        ["repo diff stage", "repo diff unstage"]
    );
    assert!(names("stage").contains(&"repo commit"));
    assert_eq!(names("REPO DIFF "), names("repo diff "));
    assert!(
        names("repo diff ")
            .iter()
            .all(|name| name.starts_with("repo diff "))
    );
    let input = "REPOS ADD \"/example/中文 项目/终端\"";
    assert_eq!(names(input), ["repos add"]);
    assert_eq!(
        argument_input(input),
        Some((ArgumentAction::Add, "\"/example/中文 项目/终端\""))
    );
}

#[test]
fn interaction_is_nested_under_settings_and_uses_its_context() {
    let expected = [
        "settings interaction",
        "settings interaction interactive",
        "settings interaction text",
    ];
    assert_eq!(names("settings interaction"), expected);
    assert_eq!(names("settings interaction "), expected[1..]);
    assert!(
        COMMANDS
            .iter()
            .all(|command| !command.name.starts_with("interaction"))
    );
    assert_eq!(
        context::Context::Settings
            .matches("interaction")
            .iter()
            .map(|command| command.name)
            .collect::<Vec<_>>(),
        expected
    );
}

#[test]
fn repository_namespaces_have_disjoint_members_and_short_searches_complete_canonical_names() {
    assert_eq!(
        names(""),
        [
            "repo", "repos", "cancel", "search", "focus", "zoom", "tasks", "schedule", "settings",
            "help", "quit"
        ]
    );
    for group in ["repo", "repos"] {
        let candidates = matches(group);
        assert_eq!(candidates[0].name, group);
        assert!(
            candidates
                .iter()
                .all(|c| c.name == group || c.name.starts_with(&format!("{group} ")))
        );
    }
    for (query, expected) in [
        ("diff", "repo diff"),
        ("log", "repo log"),
        ("branch", "repo branch"),
        ("stash", "repo stash"),
        ("push", "repo push"),
        ("te", "repo terminal"),
    ] {
        assert_eq!(matches(query)[0].name, expected);
        assert_eq!(matches(query)[0].completion(), format!("/{expected}"));
        assert!(!COMMANDS.iter().any(|c| c.name == query));
    }
    assert_eq!(matches("repos check")[0].action, Action::CheckAll);
    assert_eq!(matches("repo check")[0].action, Action::Check);
}
