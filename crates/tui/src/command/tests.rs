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
        "repo diff unstaged",
        "repo diff staged",
        "repo diff select",
        "repo diff context",
        "repo diff header",
        "repo diff discard",
        "repo diff discard-file",
        "repo diff view",
        "repo diff conflict",
        "repo diff conflict undo",
        "repo diff conflict reset",
    ];
    for input in ["di", "repo diff", "  diff"] {
        assert_eq!(&names(input)[..expected.len()], expected);
    }
    assert_eq!(names("repo diff "), expected[1..]);
    assert_eq!(names("repo diff lay"), expected[3..4]);
    assert_eq!(names("repo diff layout"), expected[3..4]);
    assert_eq!(names("repo diff layout "), expected[3..4]);
    assert_eq!(names("repo diff layout s"), ["repo diff layout"]);
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
    assert_eq!(names("repos r"), ["repos remove"]);
    assert_eq!(
        names("schedule a"),
        [
            "schedule anchor",
            "schedule autopush",
            "schedule actions",
            "schedule apply"
        ]
    );
    assert!(!COMMANDS.iter().any(|command| matches!(
        command.name,
        "repo check" | "repos check" | "settings startup" | "settings startup enabled"
    )));
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
            ("FETCH CONFIGURED", "repo fetch"),
        ] {
            assert_eq!(names(query), [expected]);
        }
        assert_eq!(names("终端"), ["open terminal", "settings interaction"]);
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
fn candidate_matching_accepts_optional_spaces_only_between_words() {
    for query in ["repocommitgenerate", "repo  commit gen", "repo　commitgen"] {
        assert_eq!(
            names(query).first(),
            Some(&"repo commit generate"),
            "{query}"
        );
    }
    assert!(names("repo  com mit").is_empty());
    assert!(names("设置模 型").is_empty());
    assert_eq!(names("ENTER  A MODEL NAME"), ["settings ai model"]);
    assert_eq!(names("enteramodelname"), ["settings ai model"]);

    let input = "repo diff conflict --v i";
    let parameter = named::Query::new(input).unwrap();
    assert!(parameter.candidates().next().is_none());
    assert!(argument_input(input).is_none());

    let path = "\"/example/中文  项目/with spaces\"";
    assert_eq!(
        argument_input(&format!("repos add {path}")),
        Some((ArgumentAction::Add, path))
    );
}

#[test]
fn name_matches_precede_description_matches_without_duplicates_or_scope_leaks() {
    let checks = names("fetch");
    assert_eq!(&checks[..2], ["repo fetch", "repos fetch"]);
    assert!(checks.contains(&"settings startup-fetch"));
    assert_eq!(
        checks
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        checks.len()
    );
    let staging = names("stage");
    assert_eq!(staging[0], "repo diff stage");
    assert!(staging.contains(&"repo diff staged"));
    assert!(staging.contains(&"repo diff unstage"));
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
    let expected = ["settings interaction"];
    assert_eq!(names("settings interaction"), expected);
    assert_eq!(names("settings interaction "), expected);
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
            "repo", "repos", "cancel", "search", "focus", "zoom", "editor", "open", "copy",
            "refresh", "tasks", "schedule", "settings", "help", "quit"
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
        ("te", "open terminal"),
    ] {
        assert_eq!(matches(query)[0].name, expected);
        assert_eq!(matches(query)[0].completion(), format!("/{expected}"));
        assert!(!COMMANDS.iter().any(|c| c.name == query));
    }
    assert_eq!(matches("repos fetch")[0].action, Action::FetchAll);
    assert_eq!(matches("repo fetch")[0].action, Action::Fetch);
}

#[test]
fn nested_parameter_commands_choose_the_longest_name_and_keep_bare_undo_unambiguous() {
    assert_eq!(
        argument_input("repo diff conflict --view "),
        Some((ArgumentAction::Option(options::Kind::Conflict), ""))
    );
    assert_eq!(
        argument_input("repo diff conflict --view base-left"),
        Some((ArgumentAction::Option(options::Kind::Conflict), "base-left"))
    );
    for query in [
        "repo diff conflict ",
        "repo diff conflict acc",
        "repo diff conflict accept left",
        "repo diff conflict block next",
        "repo diff conflict base-left",
    ] {
        assert_eq!(argument_input(query), None);
    }
    assert_eq!(argument_input("repo diff conflict undo "), None);
    assert_eq!(argument_input("repo diff conflict reset "), None);
}

// 仅供目录断言拼接完整语法；产品界面分别呈现命令名和参数提示。
impl Command {
    pub fn usage(&self) -> String {
        match self.action.argument() {
            Some(argument) => format!("{} {}", self.name, argument.placeholder()),
            None => self.name.into(),
        }
    }
}
