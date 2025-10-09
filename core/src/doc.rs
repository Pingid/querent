use ropey::Rope;

#[derive(Debug, Clone, Default)]
pub struct Content {
    text: Rope,
    cursor: usize,
}

impl Content {
    pub fn new(content: &str) -> Self {
        Self {
            text: Rope::from(content),
            cursor: 0,
        }
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn content(&self) -> &Rope {
        &self.text
    }

    pub fn set_content(&mut self, content: &str) {
        self.text = Rope::from(content);
    }

    pub fn apply_edit(
        &mut self,
        start: impl Into<Location>,
        end: impl Into<Location>,
        new_text: &str,
    ) {
        let start = self.get_offset(start);
        let end = self.get_offset(end);
        self.text.remove(start..end);
        self.text.insert(start, new_text);
    }

    pub fn set_cursor(&mut self, cursor: impl Into<Location>) {
        self.cursor = self.get_offset(cursor);
    }

    pub fn current_statement(&self) -> String {
        get_statement_at_cursor(&self.text, self.cursor)
    }

    fn get_offset(&self, location: impl Into<Location>) -> usize {
        match location.into() {
            Location::Offset(offset) => offset,
            Location::LineCol(line, col) => self.text.line_to_char(line) + col,
        }
    }

    pub fn get_line_col(&self, location: impl Into<Location>) -> (usize, usize) {
        match location.into() {
            Location::Offset(offset) => (
                self.text.char_to_line(offset),
                offset - self.text.line_to_char(self.text.char_to_line(offset)),
            ),
            Location::LineCol(line, col) => (line, col),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Location {
    Offset(usize),
    LineCol(usize, usize),
}

impl From<usize> for Location {
    fn from(offset: usize) -> Self {
        Location::Offset(offset)
    }
}

impl From<(usize, usize)> for Location {
    fn from((line, col): (usize, usize)) -> Self {
        Location::LineCol(line, col)
    }
}

/// Get sql statement text at cursor
fn get_statement_at_cursor(rope: &Rope, cursor: usize) -> String {
    let n = rope.len_chars();

    // find start (char after previous ';')
    let mut start = 0usize;
    let mut i = cursor;
    while i > 0 {
        if rope.char(i - 1) == ';' {
            start = i;
            break;
        }
        i -= 1;
    }

    // find end (char before next ';')
    let mut end = n;
    let mut i = cursor;
    while i < n {
        if rope.char(i) == ';' {
            end = i;
            break;
        }
        i += 1;
    }

    rope.slice(start..end).to_string()
}
