use std::collections::VecDeque;

use super::model::HISTORY_LIMIT;

#[derive(Clone)]
struct Snapshot {
    lines: Vec<String>,
    row: usize,
    column: usize,
}

#[derive(Clone)]
pub struct TextBuffer {
    lines: Vec<String>,
    row: usize,
    column: usize,
    undo: VecDeque<Snapshot>,
    redo: Vec<Snapshot>,
}

impl TextBuffer {
    pub fn new(text: &str) -> Self {
        let lines = text
            .replace("\r\n", "\n")
            .split('\n')
            .map(ToString::to_string)
            .collect();
        Self {
            lines,
            row: 0,
            column: 0,
            undo: VecDeque::new(),
            redo: Vec::new(),
        }
    }

    pub fn text(&self) -> String {
        self.lines.join("\n")
    }
    pub fn lines(&self) -> &[String] {
        &self.lines
    }
    pub fn cursor(&self) -> (usize, usize) {
        (self.row, self.column)
    }

    pub fn insert_char(&mut self, character: char) {
        self.checkpoint();
        if character == '\n' {
            self.insert_newline_inner();
        } else {
            let byte = byte_index(&self.lines[self.row], self.column);
            self.lines[self.row].insert(byte, character);
            self.column += 1;
        }
    }

    pub fn insert_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.checkpoint();
        for character in text.replace("\r\n", "\n").replace('\r', "\n").chars() {
            if character == '\n' {
                self.insert_newline_inner();
            } else {
                let byte = byte_index(&self.lines[self.row], self.column);
                self.lines[self.row].insert(byte, character);
                self.column += 1;
            }
        }
    }

    pub fn newline(&mut self) {
        self.checkpoint();
        self.insert_newline_inner();
    }

    pub fn backspace(&mut self) {
        if self.row == 0 && self.column == 0 {
            return;
        }
        self.checkpoint();
        if self.column > 0 {
            let end = byte_index(&self.lines[self.row], self.column);
            let start = byte_index(&self.lines[self.row], self.column - 1);
            self.lines[self.row].replace_range(start..end, "");
            self.column -= 1;
        } else {
            let line = self.lines.remove(self.row);
            self.row -= 1;
            self.column = char_len(&self.lines[self.row]);
            self.lines[self.row].push_str(&line);
        }
    }

    pub fn delete(&mut self) {
        let len = char_len(&self.lines[self.row]);
        if self.row + 1 == self.lines.len() && self.column == len {
            return;
        }
        self.checkpoint();
        if self.column < len {
            let start = byte_index(&self.lines[self.row], self.column);
            let end = byte_index(&self.lines[self.row], self.column + 1);
            self.lines[self.row].replace_range(start..end, "");
        } else {
            let next = self.lines.remove(self.row + 1);
            self.lines[self.row].push_str(&next);
        }
    }

    pub fn left(&mut self) {
        if self.column > 0 {
            self.column -= 1;
        } else if self.row > 0 {
            self.row -= 1;
            self.column = char_len(&self.lines[self.row]);
        }
    }
    pub fn right(&mut self) {
        if self.column < char_len(&self.lines[self.row]) {
            self.column += 1;
        } else if self.row + 1 < self.lines.len() {
            self.row += 1;
            self.column = 0;
        }
    }
    pub fn up(&mut self) {
        if self.row > 0 {
            self.row -= 1;
            self.clamp_column();
        }
    }
    pub fn down(&mut self) {
        if self.row + 1 < self.lines.len() {
            self.row += 1;
            self.clamp_column();
        }
    }
    pub fn home(&mut self) {
        self.column = 0;
    }
    pub fn end(&mut self) {
        self.column = char_len(&self.lines[self.row]);
    }
    pub fn page_up(&mut self, height: usize) {
        self.row = self.row.saturating_sub(height);
        self.clamp_column();
    }
    pub fn page_down(&mut self, height: usize) {
        self.row = (self.row + height).min(self.lines.len() - 1);
        self.clamp_column();
    }

    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.undo.pop_back() else {
            return false;
        };
        self.redo.push(self.snapshot());
        self.restore(previous);
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.push_undo(self.snapshot());
        self.restore(next);
        true
    }

    fn insert_newline_inner(&mut self) {
        let byte = byte_index(&self.lines[self.row], self.column);
        let next = self.lines[self.row].split_off(byte);
        self.row += 1;
        self.column = 0;
        self.lines.insert(self.row, next);
    }

    fn clamp_column(&mut self) {
        self.column = self.column.min(char_len(&self.lines[self.row]));
    }
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            lines: self.lines.clone(),
            row: self.row,
            column: self.column,
        }
    }
    fn checkpoint(&mut self) {
        self.push_undo(self.snapshot());
        self.redo.clear();
    }
    fn push_undo(&mut self, snapshot: Snapshot) {
        self.undo.push_back(snapshot);
        if self.undo.len() > HISTORY_LIMIT {
            self.undo.pop_front();
        }
    }
    fn restore(&mut self, snapshot: Snapshot) {
        self.lines = snapshot.lines;
        self.row = snapshot.row;
        self.column = snapshot.column;
    }
}

fn char_len(value: &str) -> usize {
    value.chars().count()
}
fn byte_index(value: &str, character: usize) -> usize {
    value
        .char_indices()
        .nth(character)
        .map(|(index, _)| index)
        .unwrap_or(value.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_unicode_without_splitting_code_points() {
        let mut buffer = TextBuffer::new("aé");
        buffer.end();
        buffer.backspace();
        buffer.insert_text("界\nnext");
        assert_eq!(buffer.text(), "a界\nnext");
        assert_eq!(buffer.cursor(), (1, 4));
    }

    #[test]
    fn paste_is_one_undo_operation() {
        let mut buffer = TextBuffer::new("{}");
        buffer.right();
        buffer.insert_text("\n  \"x\": true\n");
        assert!(buffer.undo());
        assert_eq!(buffer.text(), "{}");
        assert!(buffer.redo());
        assert!(buffer.text().contains("\"x\""));
    }
}
