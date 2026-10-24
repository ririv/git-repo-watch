//! Styled quick reference with explicit headings, inline tokens and scroll position.
use super::*;

pub(super) fn render(
    frame: &mut Frame,
    area: Rect,
    scroll: &mut usize,
    hits: &mut crate::pointer::Map,
    ai_enabled: bool,
) {
    let panel = block(i18n::tr!(Instructions)).padding(ratatui::widgets::Padding::horizontal(1));
    let inner = panel.inner(area);
    let source = contents(ai_enabled);
    let text = crate::markdown::linked(&source, inner.width, None);
    let lines = Paragraph::new(text.plain())
        .wrap(Wrap { trim: false })
        .line_count(inner.width);
    let max_scroll = lines.saturating_sub(inner.height as usize);
    *scroll = (*scroll).min(max_scroll);
    frame.render_widget(panel, area);
    hits.links.paragraph(frame, inner, &text, *scroll);
    crate::scrollbar::vertical(frame, area, inner, lines, *scroll);
    hits.vertical(
        area,
        inner,
        max_scroll,
        *scroll,
        crate::pointer::ScrollTarget::Text,
    );
}

fn contents(ai_enabled: bool) -> std::borrow::Cow<'static, str> {
    i18n::tr!(TuiHelp)
        .replace(
            "<!-- optional-ai -->",
            if ai_enabled { i18n::tr!(AiHelp) } else { "" },
        )
        .into()
}
#[cfg(test)]
mod tests {
    #[test]
    fn disabled_ai_has_no_generation_or_ai_configuration_help() {
        for language in [i18n::Language::English, i18n::Language::Chinese] {
            let _language = i18n::using(language);
            let text = super::contents(false);
            assert!(!text.contains("/repo commit generate") && !text.contains("/settings ai"));
            assert!(super::contents(true).contains("/settings ai"));
        }
    }
}
