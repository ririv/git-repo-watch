use super::*;

#[test]
fn linear_history_reuses_the_first_parent_lane_and_color() {
    let mut layout = Layout::default();
    let tip = layout.push(&3, &[2]);
    let middle = layout.push(&2, &[1]);
    let root = layout.push(&1, &[]);
    assert_eq!(tip.width, 1);
    assert_eq!(
        tip.outgoing,
        [Edge {
            from: 0,
            to: 0,
            color: 0
        }]
    );
    assert_eq!(tip.node, middle.node);
    assert_eq!(middle.node, root.node);
    assert!(root.outgoing.is_empty());
}

#[test]
fn merge_and_branch_convergence_route_all_parent_edges() {
    let mut layout = Layout::default();
    let merge = layout.push(&4, &[2, 3]);
    assert_eq!(merge.width, 3);
    assert_eq!(merge.outgoing.len(), 2);
    let left = layout.push(&2, &[1]);
    let right = layout.push(&3, &[1]);
    assert_eq!(left.node.column, 0);
    assert_eq!(right.node.column, 1);
    assert!(
        right
            .outgoing
            .iter()
            .any(|edge| edge.from == 1 && edge.to == 0)
    );
    let root = layout.push(&1, &[]);
    assert_eq!(root.width, 1);
}

#[test]
fn octopus_merges_independent_roots_and_shared_parents_remain_distinct() {
    let mut layout = Layout::default();
    let merge = layout.push(&8, &[5, 6, 7]);
    assert_eq!(merge.outgoing.len(), 3);
    assert_eq!(merge.width, 5);
    assert_eq!(
        merge
            .outgoing
            .iter()
            .map(|edge| edge.to)
            .collect::<Vec<_>>(),
        [0, 1, 2]
    );
    layout.push(&5, &[1]);
    layout.push(&6, &[2]);
    layout.push(&7, &[1, 2]);
    let first_root = layout.push(&1, &[]);
    assert!(
        first_root
            .outgoing
            .iter()
            .all(|edge| edge.from != first_root.node.column)
    );
    let second_root = layout.push(&2, &[]);
    assert!(second_root.outgoing.is_empty());
    let disconnected = layout.push(&9, &[]);
    assert_eq!(disconnected.node.column, 0);
    assert_ne!(disconnected.node.color, second_root.node.color);
}

#[test]
fn page_boundaries_do_not_reset_routes_or_colors() {
    let mut first_page = Layout::default();
    first_page.push(&4, &[2, 3]);
    let mut resumed = first_page.clone();
    assert_eq!(first_page.push(&2, &[1]), resumed.push(&2, &[1]));
    assert_eq!(first_page.push(&3, &[1]), resumed.push(&3, &[1]));
}
