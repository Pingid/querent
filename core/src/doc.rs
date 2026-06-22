use ropey::Rope;

#[derive(Debug, Clone, Default)]
pub struct Content {
    text: Rope,
    /// Byte offset. The completion engine indexes the document `&str` by byte,
    /// so the cursor and any offsets it returns are byte offsets, not chars.
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

    /// Replace a `(line, col)` range. Rope edits use char indices.
    pub fn apply_edit(
        &mut self, start: impl Into<Location>, end: impl Into<Location>, new_text: &str,
    ) {
        let start = self.char_offset(start);
        let end = self.char_offset(end);
        self.text.remove(start..end);
        self.text.insert(start, new_text);
    }

    /// Store the cursor as a byte offset (the unit the engine works in).
    pub fn set_cursor(&mut self, cursor: impl Into<Location>) {
        self.cursor = self.byte_offset(cursor);
    }

    pub fn to_string(&self) -> String {
        self.text.to_string()
    }

    /// `(line, col)` or char offset -> char offset (for rope editing).
    fn char_offset(&self, location: impl Into<Location>) -> usize {
        match location.into() {
            Location::Offset(offset) => offset,
            Location::LineCol(line, col) => self.text.line_to_char(line) + col,
        }
    }

    /// `(line, col)` or byte offset -> byte offset (for the engine cursor).
    fn byte_offset(&self, location: impl Into<Location>) -> usize {
        match location.into() {
            Location::Offset(offset) => offset,
            Location::LineCol(line, col) => {
                self.text.char_to_byte(self.text.line_to_char(line) + col)
            }
        }
    }

    /// Byte offset -> `(line, col)`. `col` is a (UTF-16/BMP) char column.
    pub fn get_line_col(&self, location: impl Into<Location>) -> (usize, usize) {
        match location.into() {
            Location::Offset(byte) => {
                let byte = byte.min(self.text.len_bytes());
                let line = self.text.byte_to_line(byte);
                let col = self.text.byte_to_char(byte) - self.text.line_to_char(line);
                (line, col)
            }
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
