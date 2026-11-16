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
    let text = view.text()?;
    let width = area.width.saturating_sub(2);
    let layout = text.selected_layout(width, preference);
    (width >= text.minimum_width(layout)).then_some(layout)
}
pub(super) fn annotation(action: Action, screen: &Screen, body: Rect) -> Option<String> {
    match action {
        Action::InteractionMenu => {
            Some(crate::interaction::label(screen.config.interaction_mode).into())
        }
        Action::Interaction(mode) if mode == screen.config.interaction_mode => Some("✓".into()),
        Action::FileLayoutMenu => Some(screen.file_layout.label().into()),
        Action::FileLayout(layout) if layout == screen.file_layout => Some("✓".into()),
        Action::DiffLayoutMenu => Some(summary(screen, body)),
        Action::DiffHeader => Some(screen.diff_header_label().to_owned()),
        Action::DiffContextMenu => Some(crate::diff::context_label(screen.diff_context).into()),
        Action::DiffLayout(preference) if preference == screen.diff_preference => Some("✓".into()),
        _ => None,
    }
}
