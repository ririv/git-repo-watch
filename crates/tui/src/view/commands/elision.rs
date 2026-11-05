//! Elide paths in terminal cells while keeping the selected file name visible.
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

fn prefix(text: &str, width: usize) -> String {
    let mut used = 0;
    text.graphemes(true)
        .take_while(|value| {
            used += value.width();
            used <= width
        })
        .collect()
}
fn suffix(text: &str, width: usize) -> String {
    let mut used = 0;
    let mut values = text
        .graphemes(true)
        .rev()
        .take_while(|value| {
            used += value.width();
            used <= width
        })
        .collect::<Vec<_>>();
    values.reverse();
    values.concat()
}
pub(super) fn middle(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.into();
    }
    if width == 0 {
        return String::new();
    }
    let tail = text.rsplit('/').next().expect("path has a final segment");
    if tail.width() + 2 <= width {
        let head = prefix(&text[..text.len() - tail.len()], width - tail.width() - 2);
        return format!("{head}…/{tail}");
    }
    // 文件名自身过长时保留尾部和扩展名，不截断中文、组合字符或 Emoji。
    let right = suffix(text, (width - 1) * 2 / 3);
    let left = prefix(text, width - 1 - right.width());
    format!("{left}…{right}")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paths_keep_file_names_extensions_and_whole_graphemes() {
        for value in [
            "src/very/long/components/main.ts",
            "路径/很长的目录/中文文件.ts",
            "src/e\u{301}/👩‍💻/界面.ts",
        ] {
            for width in 0..55 {
                assert!(middle(value, width).width() <= width);
            }
            assert!(middle(value, 20).ends_with(value.rsplit('/').next().unwrap()));
        }
        assert_eq!(middle("src/main.ts", 20), "src/main.ts");
        assert!(middle("src/a-very-very-long-file-name.ts", 12).ends_with(".ts"));
    }
}
