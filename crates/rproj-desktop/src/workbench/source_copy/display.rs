use anyhow::{Result, ensure};
use rproj_core::source_copy::Snapshot;

const MAX_ROWS: usize = 100_000;
const MAX_DISPLAY_BYTES: usize = 16 * 1024 * 1024;
const CHUNK_CHARS: usize = 4096;

pub(super) struct Row {
    pub(super) line: usize,
    pub(super) continuation: bool,
    pub(super) text: String,
}

pub(in super::super) struct Prepared {
    pub(super) raw: Snapshot,
    pub(super) rows: Vec<Row>,
    pub(super) file_rows: Vec<usize>,
}

impl Prepared {
    pub(super) fn new(raw: Snapshot, abandoned: impl Fn() -> bool) -> Result<Self> {
        Self::bounded(raw, abandoned, MAX_ROWS, MAX_DISPLAY_BYTES)
    }

    fn bounded(
        raw: Snapshot,
        abandoned: impl Fn() -> bool,
        max_rows: usize,
        max_bytes: usize,
    ) -> Result<Self> {
        let mut rows = Vec::new();
        let mut file_rows = Vec::with_capacity(raw.files.len());
        let mut bytes = 0;
        let mut offset = 0;
        let mut file = 0;
        for (line, text) in raw.text.split_inclusive('\n').enumerate() {
            while file < raw.files.len() && raw.files[file].range.start == offset {
                file_rows.push(rows.len());
                file += 1;
            }
            let (text, ending) = if let Some(body) = text.strip_suffix("\r\n") {
                (body, " ⟦CRLF⟧")
            } else if let Some(body) = text.strip_suffix('\n') {
                (body, " ⟦LF⟧")
            } else {
                (text, " ⟦no newline⟧")
            };
            let mut chars = text.chars().peekable();
            let mut continuation = false;
            loop {
                ensure!(!abandoned(), "Source display preparation abandoned");
                let mut display = String::new();
                for c in chars.by_ref().take(CHUNK_CHARS) {
                    if c.is_control()
                        || matches!(c, '\u{061c}' | '\u{200e}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
                    {
                        display.extend(c.escape_unicode());
                    } else {
                        display.push(c);
                    }
                }
                let more = chars.peek().is_some();
                display.push_str(if more { " ⟦continues⟧" } else { ending });
                bytes += display.len();
                ensure!(
                    rows.len() < max_rows && bytes <= max_bytes,
                    "Source display exceeds {max_rows} rows or {max_bytes} annotated bytes; review refused without truncation"
                );
                rows.push(Row {
                    line: line + 1,
                    continuation,
                    text: display,
                });
                if !more {
                    break;
                }
                continuation = true;
            }
            offset += text.len()
                + match ending {
                    " ⟦CRLF⟧" => 2,
                    " ⟦LF⟧" => 1,
                    _ => 0,
                };
        }
        ensure!(
            file_rows.len() == raw.files.len(),
            "Invalid source snapshot ranges"
        );
        Ok(Self {
            raw,
            rows,
            file_rows,
        })
    }

    pub(super) fn summary(&self) -> String {
        if self.raw.source_missing {
            "No src folder. Nothing to copy.".into()
        } else if self.raw.files.is_empty() && self.raw.skipped.is_empty() {
            "src contains no source files. Nothing to copy.".into()
        } else {
            format!(
                "{} included files · {} skipped paths · {} UTF-8 bytes",
                self.raw.files.len(),
                self.raw.skipped.len(),
                self.raw.text.len()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot(text: &str) -> Snapshot {
        Snapshot {
            project: "project".into(),
            source_missing: false,
            text: text.into(),
            files: vec![],
            skipped: vec![],
        }
    }
    #[test]
    fn annotations_never_change_payload_and_long_lines_continue() {
        let text = format!("one\r\n\t\0\u{202e}\n{}", "雪".repeat(CHUNK_CHARS + 1));
        let p = Prepared::new(snapshot(&text), || false).unwrap();
        assert_eq!(p.raw.text, text);
        assert!(p.rows[0].text.ends_with("⟦CRLF⟧"));
        assert!(p.rows[1].text.contains("\\u{202e}"));
        assert_eq!(p.rows[3].line, 3);
        assert!(p.rows[3].continuation);
        assert!(p.rows[3].text.ends_with("⟦no newline⟧"));
    }
    #[test]
    fn display_limits_refuse_instead_of_truncating() {
        assert!(Prepared::bounded(snapshot("a\nb\n"), || false, 1, 100).is_err());
        assert!(Prepared::bounded(snapshot("\0"), || false, 2, 5).is_err());
        assert!(Prepared::new(snapshot("text"), || true).is_err());
        assert!(Prepared::new(snapshot(&"\n".repeat(MAX_ROWS + 1)), || false).is_err());
    }
}
