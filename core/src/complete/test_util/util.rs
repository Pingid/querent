use std::borrow::Cow;

pub fn get_caret_cursor<'a>(sql_with_caret: &'a str) -> (Cow<'a, str>, usize) {
    let pos = sql_with_caret.find('^');
    if let Some(pos) = pos {
        let (before, after_with_caret) = sql_with_caret.split_at(pos);
        let s = [before, &after_with_caret[1..]].concat(); // allocates once
        (Cow::Owned(s), pos)
    } else {
        (Cow::Borrowed(sql_with_caret), sql_with_caret.len())
    }
}

// This is a workaround to create 'static lifetimes for testing
// In reality, we leak memory here but it's fine for tests
pub fn leaky_static_caret_cursor(sql_with_caret: &str) -> (&'static str, usize) {
    let (text, pos) = get_caret_cursor(sql_with_caret);
    (Box::leak(text.to_string().into_boxed_str()), pos)
}
