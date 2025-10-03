use std::borrow::Cow;

pub mod ast;

#[allow(dead_code)]
pub fn with_caret_cursor<'a>(sql_with_caret: &'a str) -> (Cow<'a, str>, usize) {
    let pos = sql_with_caret.find('^').expect("missing ^");
    let (before, after_with_caret) = sql_with_caret.split_at(pos);
    let s = [before, &after_with_caret[1..]].concat(); // allocates once
    (Cow::Owned(s), pos)
}
