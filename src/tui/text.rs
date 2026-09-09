pub fn wrap_lines(text: &str, width: usize) -> Vec<String> {
    use unicode_width::UnicodeWidthChar;
    let width = width.max(1);
    let mut lines = Vec::new();
    for line in text.lines() {
        let code = line.starts_with(' ') || line.starts_with('\t');
        let mut current = String::new();
        let mut used = 0;
        for word in line.split_inclusive(' ') {
            let word_width = unicode_width::UnicodeWidthStr::width(word);
            if !code && used > 0 && used + word_width > width && word_width <= width {
                lines.push(current.trim_end().to_owned());
                current = String::new();
                used = 0;
            }
            for ch in word.chars() {
                let size = ch.width().unwrap_or(0);
                if used + size > width && !current.is_empty() {
                    lines.push(current);
                    current = String::new();
                    used = 0;
                }
                current.push(ch);
                used += size;
            }
        }
        lines.push(current);
    }
    lines
}
