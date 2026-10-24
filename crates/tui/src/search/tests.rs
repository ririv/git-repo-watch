use super::Matches;

#[test]
fn navigation_wraps_and_accepts_a_selection_between_matches() {
    let matches = Matches::new([false, true, false, true, false]);
    assert_eq!(matches.nearest(Some(2)), Some(3));
    assert_eq!(matches.nearest(Some(4)), Some(1));
    assert_eq!(matches.step(Some(1), false), Some(3));
    assert_eq!(matches.step(Some(3), false), Some(1));
    assert_eq!(matches.step(Some(1), true), Some(3));
    assert_eq!(matches.step(Some(2), true), Some(1));
    assert_eq!(matches.label(Some(3)), "2/2");
    assert_eq!(matches.label(Some(2)), "—/2");
}

#[test]
fn empty_and_single_match_have_explicit_navigation_results() {
    let empty = Matches::new([false, false]);
    assert_eq!(empty.nearest(Some(1)), None);
    assert_eq!(empty.step(Some(1), false), None);
    assert_eq!(empty.step(Some(1), true), None);
    assert_eq!(empty.label(Some(1)), "0/0");
    let single = Matches::new([false, true]);
    assert_eq!(single.step(Some(1), false), Some(1));
    assert_eq!(single.step(Some(1), true), Some(1));
    assert_eq!(single.step(None, false), Some(1));
    assert_eq!(single.step(None, true), Some(1));
}
