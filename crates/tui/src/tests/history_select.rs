use super::*;
use crate::history::{SelectionAction, Session};

fn graph(s: &Screen) -> &Session {
    s.history_graph_target().expect("history graph")
}
fn graph_mut(s: &mut Screen) -> &mut Session {
    let Mode::History(view) = &mut s.mode else {
        panic!("history");
    };
    let crate::history::View::Graph(graph) = view.as_mut() else {
        panic!("graph");
    };
    graph
}

#[test]
fn selection_subcommands_are_discoverable_and_tab_does_not_toggle_selection() {
    for language in [i18n::Language::Chinese, i18n::Language::English] {
        let _language = i18n::using(language);
        let f = support::Fixture::create();
        let (_home, mut s) = fixture(vec![f.repo.clone()]);
        command(&mut s, "repo log");
        wait(&mut s);
        s.paste("/select");
        assert_eq!(
            s.search_commands()
                .unwrap()
                .iter()
                .map(|c| c.name)
                .collect::<Vec<_>>(),
            [
                "repo log select",
                "repo log select all",
                "repo log select clear"
            ]
        );
        let text = render(&mut s, 150, 32);
        assert!(
            text.contains("├─ all") && text.contains("└─ clear"),
            "{text}"
        );
        assert!(
            s.command_enter_hint()
                .unwrap()
                .contains(i18n::tr!(CommandChildrenKeys))
        );
        key(&mut s, KeyCode::Tab);
        assert_eq!(s.bar.text(), "/repo log select ");
        assert!(!graph(&s).selection.active());
        key(&mut s, KeyCode::Esc);
        assert_eq!(s.bar.text(), "/repo log select");
        key(&mut s, KeyCode::Enter);
        assert!(graph(&s).selection.active());
        assert_eq!(graph(&s).selection.count(), 0);
        s.shutdown();
    }
}

#[test]
fn all_selects_the_loaded_list_without_paging_and_clear_keeps_the_selection_mode() {
    let f = support::Fixture::create();
    for index in 0..105 {
        f.commit("pages.txt", &format!("page {index}\n"));
    }
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    let head = support::git(&f.repo, &["rev-parse", "HEAD"]);
    command(&mut s, "repo log");
    wait(&mut s);
    assert_eq!(graph(&s).records.len(), 100);
    assert!(graph(&s).cursor().is_some());
    let query = graph(&s).records[4].entry.oid.to_string();
    s.paste(&query);
    let selected = graph(&s).state.selected();
    command(&mut s, "repo log select all");
    assert!(graph(&s).selection.active());
    assert_eq!(graph(&s).selection.count(), 100);
    assert_eq!(graph(&s).records.len(), 100);
    assert!(graph(&s).cursor().is_some());
    assert_eq!(graph(&s).query, query);
    assert_eq!(graph(&s).state.selected(), selected);
    assert!(s.repository_tasks.is_empty());
    // 检索保留整份拓扑，没有命中关键词的提交也属于当前已加载列表。
    assert!(
        graph(&s)
            .records
            .iter()
            .any(|r| !graph(&s).matches_entry(&r.entry))
    );
    assert!(
        graph(&s)
            .records
            .iter()
            .all(|r| graph(&s).selection.contains(&r.entry.oid))
    );
    command(&mut s, "repo log select clear");
    assert!(graph(&s).selection.active());
    assert_eq!(graph(&s).selection.count(), 0);
    assert_eq!(s.bar.text(), query);
    key(&mut s, KeyCode::Insert);
    assert_eq!(graph(&s).selection.count(), 1);
    command(&mut s, "repo log select");
    assert!(!graph(&s).selection.active());
    assert_eq!(graph(&s).selection.count(), 0);
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert_eq!(support::git(&f.repo, &["rev-parse", "HEAD"]), head);
    s.shutdown();
}

#[test]
fn the_selection_limit_is_atomic_and_clear_remains_available_above_it() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    command(&mut s, "repo log");
    wait(&mut s);
    let seed = graph(&s).records[0].clone();
    let limit = engine::operations::replay::batch::MAX_COMMITS;
    let records = (0..=limit)
        .map(|index| {
            let mut record = seed.clone();
            let mut bytes = [0u8; 20];
            bytes[12..].copy_from_slice(&(index as u64 + 1).to_be_bytes());
            record.entry.oid = engine::local::history::Oid::from_bytes(&bytes).unwrap();
            record
        })
        .collect::<Vec<_>>();
    graph_mut(&mut s).records = Arc::new(records[..limit].to_vec());
    command(&mut s, "repo log select all");
    assert_eq!(graph(&s).selection.count(), limit);
    graph_mut(&mut s).records = Arc::new(records);
    let before = graph(&s).picks();
    command(&mut s, "repo log select all");
    assert_eq!(s.message, i18n::tr!(ReplayBatchTooLarge));
    assert_eq!(graph(&s).picks(), before);
    assert!(graph_mut(&mut s).mark(SelectionAction::All).is_err());
    assert_eq!(graph(&s).picks(), before);
    key(&mut s, KeyCode::Esc);
    command(&mut s, "repo log select clear");
    assert!(graph(&s).selection.active());
    assert_eq!(graph(&s).selection.count(), 0);
    s.shutdown();
}

#[test]
fn selection_children_require_a_history_graph_and_cannot_modify_repository_files() {
    let f = support::Fixture::create();
    let (_home, mut s) = fixture(vec![f.repo.clone()]);
    let index = std::fs::read(f.repo.join(".git/index")).unwrap();
    for name in ["repo log select all", "repo log select clear"] {
        command(&mut s, name);
        assert!(matches!(s.mode, Mode::Browsing));
        assert_eq!(s.message, i18n::tr!(CommandRequiresHistory));
        key(&mut s, KeyCode::Esc);
    }
    assert_eq!(std::fs::read(f.repo.join(".git/index")).unwrap(), index);
    assert!(s.repository_tasks.is_empty());
    s.shutdown();
}
