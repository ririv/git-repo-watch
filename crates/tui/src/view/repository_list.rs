//! Repository-list command annotations share the same typed session options.
use super::*;

pub(super) fn annotation(action: Action, screen: &Screen) -> Option<String> {
    match action {
        Action::FilterMenu => Some(screen.filter.label().into()),
        Action::SortMenu => Some(screen.order.label().into()),
        Action::Filter(filter) if filter == screen.filter => Some("✓".into()),
        Action::Sort(order) if order == screen.order => Some("✓".into()),
        _ => None,
    }
}

pub(super) fn menu_title(screen: &Screen) -> Option<&'static str> {
    match screen.bar.text().to_ascii_lowercase().as_str() {
        "/repo filter " => Some(i18n::tr!(RepositoryFilterCommand)),
        "/repo sort " => Some(i18n::tr!(RepositorySortCommand)),
        _ => None,
    }
}
