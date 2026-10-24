//! Repository-list command annotations share the same typed session options.
use super::*;

pub(super) fn annotation(action: Action, screen: &Screen) -> Option<String> {
    match action {
        Action::SearchMenu => screen
            .search_support()
            .mode()
            .map(|mode| mode.label().into()),
        Action::Search(mode) if screen.search_support().mode() == Some(mode) => Some("✓".into()),
        Action::FilterMenu => Some(screen.filter.label().into()),
        Action::SortMenu => Some(screen.order.label().into()),
        Action::Filter(filter) if filter == screen.filter => Some("✓".into()),
        Action::Sort(order) if order == screen.order => Some("✓".into()),
        _ => None,
    }
}
