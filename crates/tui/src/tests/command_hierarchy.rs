use super::*;
use crate::command::{
    completion::Menu,
    hierarchy::{Group, Location, Tree},
};

fn row(s: &mut Screen, index: usize, width: u16) -> String {
    let text = render(s, width, 32);
    let area = s
        .command_items
        .iter()
        .find(|(_, item)| *item == index)
        .unwrap()
        .0;
    text.lines()
        .nth(area.y as usize)
        .unwrap()
        .chars()
        .skip(area.x as usize)
        .take(area.width as usize)
        .collect()
}

#[test]
fn commit_parent_exposes_its_child_and_enter_still_opens_the_commit_page() {
    let f = support::Fixture::create();
    f.commit("draft.txt", "old\n");
    support::write(f.repo.join("draft.txt"), "new\n");
    support::git(&f.repo, &["add", "draft.txt"]);
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    s.config.ai.enabled = true;
    s.paste("//repo commit");
    let commands = s.search_commands().unwrap();
    assert_eq!(
        commands.iter().map(|c| c.name).collect::<Vec<_>>(),
        ["repo commit", "repo commit generate"]
    );
    for width in [50, 120] {
        assert!(row(&mut s, 0, width).starts_with("/repo commit"));
        assert!(row(&mut s, 1, width).starts_with("└─ generate"));
    }
    let hint = s.command_enter_hint().unwrap();
    assert!(hint.contains(i18n::tr!(CommandEnterOpen)), "{hint}");
    assert!(hint.contains(i18n::tr!(CommandChildrenKeys)), "{hint}");
    let head = support::git(&f.repo, &["rev-parse", "HEAD"]);
    key(&mut s, KeyCode::Enter);
    wait(&mut s);
    assert!(matches!(s.mode, Mode::CommitMessage { .. }));
    assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), head);
    s.shutdown();
}

#[test]
fn tab_completes_before_entering_a_group_and_escape_returns_to_the_parent_in_both_scopes() {
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        for scoped in [false, true] {
            let f = support::Fixture::create();
            let (_home, mut s) = fixture(vec![f.repo.clone()]);
            if scoped {
                key(&mut s, KeyCode::Enter);
                wait_workspace(&mut s);
            }
            s.config.ai.enabled = true;
            s.paste(if scoped { "/comm" } else { "//repo comm" });
            key(&mut s, KeyCode::Tab);
            assert_eq!(s.bar.text(), "/repo commit");
            assert!(s.repository_tasks.is_empty());
            key(&mut s, KeyCode::Tab);
            assert_eq!(s.bar.text(), "/repo commit ");
            let candidates = s.search_commands().unwrap();
            assert_eq!(
                candidates.iter().map(|c| c.name).collect::<Vec<_>>(),
                ["repo commit generate"]
            );
            let group = Group::current(s.bar.command_input().unwrap(), &candidates).unwrap();
            assert!(group.location == Location::Children);
            assert!(
                render(&mut s, 120, 32)
                    .contains(&i18n::tf!(CommandChildrenTitle, name = "repo commit"))
            );
            key(&mut s, KeyCode::Esc);
            assert_eq!(s.bar.text(), "/repo commit");
            assert_eq!(s.search_commands().unwrap()[0].name, "repo commit");
            assert!(s.repository_tasks.is_empty());
            key(&mut s, KeyCode::Esc);
            assert!(s.bar.is_plain());
            s.shutdown();
        }
    }
}

#[test]
fn hierarchy_is_shared_by_executable_groups_and_preserves_parameter_menus() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    for parent in ["repo files", "repo log", "repo compose"] {
        s.paste(&format!("//{parent}"));
        let commands = s.search_commands().unwrap();
        let group = Group::current(s.bar.command_input().unwrap(), &commands).unwrap();
        assert!(group.location == Location::Parent);
        assert_eq!(commands[0].name, parent);
        let input = s.bar.command_input().unwrap();
        assert!(
            Tree::new(Some(&group), &commands)
                .label(input, commands[1])
                .branch
                .starts_with(['├', '└'])
        );
        key(&mut s, KeyCode::Tab);
        assert_eq!(s.bar.text(), format!("/{parent} "));
        assert!(
            s.search_commands()
                .unwrap()
                .iter()
                .all(|c| c.name != parent)
        );
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.bar.text(), format!("/{parent}"));
        key(&mut s, KeyCode::Esc);
    }
    s.paste("//repo diff layout");
    key(&mut s, KeyCode::Enter);
    assert!(matches!(
        s.bar,
        bar::InputBar::Command {
            menu: Menu::Options(_),
            ..
        }
    ));
    s.shutdown();
}

#[test]
fn selecting_a_child_keeps_its_canonical_action_and_changes_the_enter_hint() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    s.config.ai.enabled = true;
    s.paste("//repo commit");
    key(&mut s, KeyCode::Down);
    assert!(
        !s.command_enter_hint()
            .unwrap()
            .contains(i18n::tr!(CommandChildrenKeys))
    );
    key(&mut s, KeyCode::Tab);
    assert_eq!(s.bar.text(), "/repo commit generate");
    assert_eq!(
        s.search_commands().unwrap()[0].action,
        Action::CommitGenerate
    );
    key(&mut s, KeyCode::Esc);
    assert!(s.bar.is_plain());
    s.shutdown();
}

#[test]
fn hidden_children_do_not_advertise_a_group_and_editing_a_partial_command_keeps_escape_behavior() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    s.config.ai.enabled = false;
    s.paste("//repo commit");
    assert_eq!(s.search_commands().unwrap().len(), 1);
    assert!(
        !s.command_enter_hint()
            .unwrap()
            .contains(i18n::tr!(CommandChildrenKeys))
    );
    key(&mut s, KeyCode::Tab);
    assert_eq!(s.bar.text(), "/repo commit");
    key(&mut s, KeyCode::Esc);
    assert!(s.bar.is_plain());
    for draft in ["repo diff lay", "repo diff lay "] {
        s.paste(&format!("//{draft}"));
        assert_eq!(s.bar.text(), format!("/{draft}"));
        key(&mut s, KeyCode::Esc);
        assert!(s.bar.is_plain());
    }
    s.shutdown();
}

#[test]
fn command_tree_connects_nested_siblings_and_ends_the_final_branch() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    s.paste("//repo");
    for (command, expected) in [
        ("repo branch", "├─ branch"),
        ("repo branch create", "│  ├─ create"),
        ("repo branch switch", "│  └─ switch"),
        ("repo stash", "└─ stash"),
        ("repo stash drop", "   └─ drop"),
    ] {
        let index = s
            .search_commands()
            .unwrap()
            .iter()
            .position(|c| c.name == command)
            .unwrap();
        if let bar::InputBar::Command { selected, .. } = &mut s.bar {
            *selected = index;
        }
        let line = row(&mut s, index, 120);
        assert!(line.starts_with(expected), "{command}: {line}");
    }
    s.shutdown();
}

#[test]
fn partial_search_connects_every_matched_parent_without_losing_cross_group_results() {
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        for entry in ["//commi", "//repo commi", "/commi"] {
            let f = support::Fixture::create();
            let (_home, mut s) = fixture(vec![f.repo.clone()]);
            if entry == "/commi" {
                key(&mut s, KeyCode::Enter);
                wait_workspace(&mut s);
            }
            s.config.ai.enabled = true;
            s.paste(entry);
            let before = s.bar.text();
            let commands = s.search_commands().unwrap();
            let names = commands
                .iter()
                .map(|command| command.name)
                .collect::<Vec<_>>();
            for (parent, child, label) in [
                ("repo commit", "repo commit generate", "└─ generate"),
                ("repo compose", "repo compose commit", "├─ commit"),
                ("repo log", "repo log more", "└─ more"),
            ] {
                if (entry == "//repo commi" || entry == "/commi") && parent != "repo commit" {
                    assert!(!names.contains(&child));
                    continue;
                }
                let parent_index = names.iter().position(|name| *name == parent).unwrap();
                let child_index = names.iter().position(|name| *name == child).unwrap();
                assert!(child_index > parent_index, "{names:?}");
                assert!(
                    names[parent_index + 1..child_index]
                        .iter()
                        .all(|name| name.starts_with(&format!("{parent} "))),
                    "{names:?}"
                );
                if let bar::InputBar::Command { selected, .. } = &mut s.bar {
                    *selected = child_index;
                }
                let line = row(&mut s, child_index, 120);
                assert!(
                    line.starts_with(label),
                    "{entry}: {child}: {line}; {names:?}"
                );
            }
            if entry != "//repo commi" {
                assert!(names.contains(&"repo branch create"));
            }
            assert_eq!(s.bar.text(), before);
            assert!(s.repository_tasks.is_empty());
            s.shutdown();
        }
    }
}

#[test]
fn an_independently_matched_child_keeps_its_full_command_name() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    s.paste("//repo branch create");
    assert_eq!(
        s.search_commands()
            .unwrap()
            .iter()
            .map(|c| c.name)
            .collect::<Vec<_>>(),
        ["repo branch create"]
    );
    assert!(row(&mut s, 0, 120).starts_with("/repo branch create"));
    s.shutdown();
}
