use ropey::Rope;

#[derive(Debug, Clone, Default)]
pub struct Doc {
    content: Rope,
    cursor: usize,
}

impl Doc {
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn content(&self) -> &Rope {
        &self.content
    }

    pub fn set_content(&mut self, content: &str) {
        self.content = Rope::from(content);
    }

    pub fn apply_edit(&mut self, start: usize, end: usize, new_text: &str) {
        self.content.remove(start..end);
        self.content.insert(start, new_text);
    }

    pub fn set_cursor(&mut self, cursor: usize) {
        self.cursor = cursor;
    }

    pub fn current_statement(&self) -> String {
        get_statement_at_cursor(&self.content, self.cursor)
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
