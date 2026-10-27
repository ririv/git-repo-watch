mod summary;
use super::*;
pub(super) fn header(screen: &mut Screen, area: Rect) -> Text<'static> {
    let mut lines = vec![super::breadcrumbs::render(screen, area)];
    if screen.page == Page::Detail && crate::workspace::Workspace::is_root(&screen.mode) {
        let size_hint = match screen
            .workspace
            .session()
            .map(|session| session.arrangement)
        {
            Some(crate::workspace::Arrangement::Maximized) => i18n::tr!(WorkspaceRestoreHint),
            _ => i18n::tr!(WorkspaceMaximizeHint),
        };
        lines[0].spans.push(Span::styled(
            if crate::workspace::Section::focused(&screen.mode).is_some() {
                format!("  · {} · {size_hint}", i18n::tr!(WorkspaceFocusHint))
            } else {
                format!("  · {}", i18n::tr!(WorkspaceFocusHint))
            },
            Style::default().fg(MUTED),
        ));
    }
    if matches!(screen.mode, Mode::Browsing) && screen.page == Page::Repositories {
        if screen.snapshot.registration == Registration::NotLoaded
            && screen.page == Page::Repositories
        {
            lines[0].spans.push(Span::styled(
                i18n::tr!(ScheduleNotLoaded),
                Style::default().fg(WARN),
            ))
        }
        lines.extend(summary::lines(screen, area.width));
    }
    Text::from(lines)
}
