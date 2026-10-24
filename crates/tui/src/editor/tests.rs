use super::*;

fn alt(editor: &mut Editor, code: KeyCode) {
    editor.key(KeyEvent::new(code, KeyModifiers::ALT));
}

#[test]
fn word_navigation_handles_paths_unicode_combining_marks_and_text_boundaries() {
    for (back, forward) in [
        (KeyCode::Char('b'), KeyCode::Char('f')),
        (KeyCode::Left, KeyCode::Right),
    ] {
        let mut editor = Editor::new("alpha/βeta  e\u{301} 👩‍💻 end!".into());
        editor.cursor = 0;
        alt(&mut editor, back);
        assert_eq!(editor.cursor, 0);
        for prefix in [
            "alpha",
            "alpha/βeta",
            "alpha/βeta  e\u{301}",
            "alpha/βeta  e\u{301} 👩‍💻 end",
        ] {
            alt(&mut editor, forward);
            assert_eq!(editor.cursor, prefix.len());
        }
        alt(&mut editor, forward);
        assert_eq!(editor.cursor, editor.text.len());
        alt(&mut editor, forward);
        assert_eq!(editor.cursor, editor.text.len());
        for word in ["end", "e\u{301}", "βeta", "alpha"] {
            alt(&mut editor, back);
            assert_eq!(editor.cursor, editor.text.find(word).unwrap());
        }
        alt(&mut editor, back);
        assert_eq!(editor.cursor, 0);
    }
}

#[test]
fn forward_word_deletion_keeps_the_prefix_cursor_and_following_text() {
    for word in ["hello", "café", "e\u{301}"] {
        let mut editor = Editor::new(format!("保留 / {word}, suffix"));
        editor.cursor = "保留".len();
        alt(&mut editor, KeyCode::Char('d'));
        assert_eq!(editor.text, "保留, suffix");
        assert_eq!(editor.cursor, "保留".len());
        alt(&mut editor, KeyCode::Char('d'));
        assert_eq!(editor.text, "保留");
        alt(&mut editor, KeyCode::Char('d'));
        assert_eq!(editor.text, "保留");
    }
}
