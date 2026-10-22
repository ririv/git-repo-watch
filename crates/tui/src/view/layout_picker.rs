//! Command annotations resolve the actual visible text layout without reloading Git.
use super::*;
use crate::{
    changes::{Focus, Preview},
    diff::{Layout as DiffLayout, Preference},
    document,
};

pub(super) fn summary(screen: &Screen, body: Rect) -> String {
    let actual = match &screen.mode {
        Mode::Changes(session) => match &session.preview {
            Preview::Ready { content, .. } => changes::areas(body, session)
                .into_iter()
                .find(|(_, focus)| *focus == Focus::Preview)
                .and_then(|(area, _)| effective(content, area, screen.diff_preference)),
            _ => None,
        },
        Mode::History(view) => match view.as_ref() {
            crate::history::View::Diff { content, .. } => {
                effective(content, body, screen.diff_preference)
            }
            _ => None,
        },
        _ => None,
    };
    match (screen.diff_preference, actual) {
        (Preference::Auto, Some(layout)) => format!(
            "{} ({} {})",
            Preference::Auto.label(),
            i18n::tr!(LayoutVisible),
            layout.label()
        ),
        (preference, _) => preference.label().to_owned(),
    }
}
fn effective(view: &document::View, area: Rect, preference: Preference) -> Option<DiffLayout> {
    let document::View::Text(text) = view else {
        return None;
    };
    let width = area.width.saturating_sub(2);
    let layout = text.selected_layout(width, preference);
    (width >= text.minimum_width(layout)).then_some(layout)
}
pub(super) fn annotation(action: Action, screen: &Screen, body: Rect) -> Option<String> {
    match action {
        Action::Diff | Action::DiffLayoutMenu => Some(format!(
            "{}: {}",
            i18n::tr!(LayoutCurrent),
            summary(screen, body)
        )),
        Action::DiffLayout(preference) if preference == screen.diff_preference => {
            Some(format!("✓ {}", i18n::tr!(LayoutCurrent)))
        }
        _ => None,
    }
}
pub(crate) fn options(inner: Rect, current: Preference) -> Vec<(Rect, Preference, String)> {
    let mut x = inner.x.saturating_add(1);
    Preference::ALL
        .into_iter()
        .map(|preference| {
            let label = format!(
                " {} {} ",
                if current == preference { "✓" } else { " " },
                preference.label()
            );
            let width = unicode_width::UnicodeWidthStr::width(label.as_str()) as u16;
            let area = Rect::new(x, inner.y, width, 1).intersection(inner);
            x = x.saturating_add(width + 2);
            (area, preference, label)
        })
        .collect()
}
