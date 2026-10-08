use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Context,
    Added,
    Removed,
    Meta,
}

pub(super) struct Row {
    pub text: String,
    pub kind: Kind,
}

impl Row {
    pub fn meta(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: Kind::Meta,
        }
    }
}

pub(super) fn visible(text: &str) -> String {
    text.chars()
        .flat_map(|ch| {
            if ch.is_control() {
                ch.escape_default().collect::<Vec<_>>()
            } else {
                vec![ch]
            }
        })
        .collect()
}

fn row(kind: Kind, line: &str) -> Row {
    let (content, ending) = if let Some(content) = line.strip_suffix("\r\n") {
        (content, " [CRLF]")
    } else if let Some(content) = line.strip_suffix('\n') {
        (content, "")
    } else {
        (line, " [no newline]")
    };
    let prefix = match kind {
        Kind::Added => '+',
        Kind::Removed => '-',
        _ => ' ',
    };
    Row {
        text: format!("{prefix}{}{ending}", visible(content)),
        kind,
    }
}

/// Match exact lines, including endings. Bound the quadratic matcher; a large
/// changed region remains fully reviewable as one removal/addition block.
fn operations<'a>(old: &[&'a str], new: &[&'a str]) -> Vec<(Kind, &'a str)> {
    let prefix = old.iter().zip(new).take_while(|(a, b)| a == b).count();
    let suffix = old[prefix..]
        .iter()
        .rev()
        .zip(new[prefix..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let left = &old[prefix..old.len() - suffix];
    let right = &new[prefix..new.len() - suffix];
    let mut result: Vec<_> = old[..prefix]
        .iter()
        .map(|line| (Kind::Context, *line))
        .collect();
    let cells = (left.len() + 1).checked_mul(right.len() + 1);
    if cells.is_some_and(|cells| cells <= 1_000_000) {
        let width = right.len() + 1;
        let mut lengths = vec![0_u32; cells.unwrap()];
        for i in (0..left.len()).rev() {
            for j in (0..right.len()).rev() {
                lengths[i * width + j] = if left[i] == right[j] {
                    1 + lengths[(i + 1) * width + j + 1]
                } else {
                    lengths[(i + 1) * width + j].max(lengths[i * width + j + 1])
                };
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < left.len() || j < right.len() {
            if i < left.len() && j < right.len() && left[i] == right[j] {
                result.push((Kind::Context, left[i]));
                i += 1;
                j += 1;
            } else if i < left.len()
                && (j == right.len() || lengths[(i + 1) * width + j] >= lengths[i * width + j + 1])
            {
                result.push((Kind::Removed, left[i]));
                i += 1;
            } else {
                result.push((Kind::Added, right[j]));
                j += 1;
            }
        }
    } else {
        result.extend(left.iter().map(|line| (Kind::Removed, *line)));
        result.extend(right.iter().map(|line| (Kind::Added, *line)));
    }
    result.extend(
        old[old.len() - suffix..]
            .iter()
            .map(|line| (Kind::Context, *line)),
    );
    result
}

pub(super) fn rows(original: Option<&str>, replacement: &str, name: &str) -> Vec<Row> {
    let old: Vec<_> = original.unwrap_or("").split_inclusive('\n').collect();
    let new: Vec<_> = replacement.split_inclusive('\n').collect();
    let ops = operations(&old, &new);
    let mut hunks: Vec<Range<usize>> = Vec::new();
    for (index, (kind, _)) in ops.iter().enumerate() {
        if *kind == Kind::Context {
            continue;
        }
        let range = index.saturating_sub(3)..(index + 4).min(ops.len());
        if let Some(last) = hunks.last_mut().filter(|last| last.end >= range.start) {
            last.end = range.end;
        } else {
            hunks.push(range);
        }
    }
    let mut rows = vec![
        Row::meta(format!(
            "--- {}",
            if original.is_some() {
                visible(name)
            } else {
                "/dev/null".into()
            }
        )),
        Row::meta(format!("+++ {}", visible(name))),
    ];
    let (mut old_pos, mut new_pos, mut cursor) = (0, 0, 0);
    for range in hunks {
        for (kind, _) in &ops[cursor..range.start] {
            old_pos += usize::from(*kind != Kind::Added);
            new_pos += usize::from(*kind != Kind::Removed);
        }
        let hunk = &ops[range.clone()];
        let old_count = hunk.iter().filter(|(kind, _)| *kind != Kind::Added).count();
        let new_count = hunk
            .iter()
            .filter(|(kind, _)| *kind != Kind::Removed)
            .count();
        rows.push(Row::meta(format!(
            "@@ -{},{} +{},{} @@",
            old_pos + usize::from(old_count > 0),
            old_count,
            new_pos + usize::from(new_count > 0),
            new_count
        )));
        rows.extend(hunk.iter().map(|(kind, line)| row(*kind, line)));
        old_pos += old_count;
        new_pos += new_count;
        cursor = range.end;
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(rows: &[Row]) -> String {
        rows.iter()
            .map(|row| row.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn creation_replacement_and_endings_are_visible() {
        let created = rows(None, "新\tfile\r\nlast", "new.toml");
        assert_eq!(
            text(&created),
            "--- /dev/null\n+++ new.toml\n@@ -0,0 +1,2 @@\n+新\\tfile [CRLF]\n+last [no newline]"
        );
        let changed = rows(Some("a\r\nlast"), "a\nlast\n", "config");
        assert!(text(&changed).contains("-a [CRLF]"));
        assert!(text(&changed).contains("-last [no newline]"));
        assert!(text(&changed).contains("+a\n+last"));
    }

    #[test]
    fn separated_changes_have_context_and_correct_line_numbers() {
        let old = (1..=20).map(|i| format!("line{i}\n")).collect::<String>();
        let new = old
            .replace("line2\n", "changed2\n")
            .replace("line18\n", "changed18\n");
        let result = rows(Some(&old), &new, "config");
        let output = text(&result);
        assert!(output.contains("@@ -1,5 +1,5 @@"), "{output}");
        assert!(output.contains("@@ -15,6 +15,6 @@"), "{output}");
        assert!(!output.contains("line10"));
        assert!(output.contains("-line2\n+changed2"));
        assert!(output.contains("-line18\n+changed18"));
    }

    #[test]
    fn large_regions_fall_back_without_losing_any_lines() {
        let old = (0..1200).map(|i| format!("old{i}\n")).collect::<String>();
        let new = (0..1200).map(|i| format!("new{i}\n")).collect::<String>();
        let result = rows(Some(&old), &new, "large");
        let removed: Vec<_> = result
            .iter()
            .filter(|row| row.kind == Kind::Removed)
            .collect();
        let added: Vec<_> = result
            .iter()
            .filter(|row| row.kind == Kind::Added)
            .collect();
        assert_eq!(removed.len(), 1200);
        assert_eq!(added.len(), 1200);
        assert_eq!(removed.last().unwrap().text, "-old1199");
        assert_eq!(added.last().unwrap().text, "+new1199");
    }

    #[test]
    fn insertion_deletion_empty_files_and_controls_are_faithful() {
        assert!(text(&rows(Some("a\nb\n"), "a\n", "config")).contains("-b"));
        assert!(text(&rows(Some("a\n"), "a\nb\n", "config")).contains("+b"));
        assert_eq!(text(&rows(Some(""), "", "empty")), "--- empty\n+++ empty");
        assert_eq!(visible("text\x1b[2J\r"), "text\\u{1b}[2J\\r");
    }
}
