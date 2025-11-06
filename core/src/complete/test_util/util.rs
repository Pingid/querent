use std::borrow::Cow;

/// A utility for formatting sets of field-value pairs for display.
///
/// This struct helps create formatted output for debugging and testing purposes,
/// collecting field-value pairs and joining them with a specified separator.
pub struct FieldSetFormatter(Vec<String>);
impl FieldSetFormatter {
    pub fn new() -> Self {
        Self(vec![])
    }
    pub fn push<T: std::fmt::Debug>(&mut self, field: &str, value: &T) {
        self.0.push(format!("{}: {:?}", field, value));
    }
    pub fn push_some<T: std::fmt::Debug>(&mut self, field: &str, value: Option<&T>) {
        if let Some(value) = value {
            self.0.push(format!("{}: {:?}", field, value));
        }
    }
    pub fn join(&self, separator: &str) -> String {
        self.0.join(separator)
    }
}

/// Formats a list of items with a newline between each item.
pub fn fmt_list<T: std::fmt::Display>(list: &[T]) -> String {
    format!(
        "[{}{}]",
        list.iter()
            .map(|x| format!("\n  {}", x))
            .collect::<Vec<_>>()
            .join(","),
        if list.len() > 1 { "\n" } else { "" },
    )
}

/// Formats debug scores for display in test failure messages.
pub fn fmt_debug_scores(
    scores: Option<&std::collections::HashMap<String, Vec<(String, f32, f32)>>>,
) -> String {
    match scores {
        None => String::new(),
        Some(scores) => {
            let mut output = String::from("\nDebug Scores (Ranker, Score, Weight):\n");
            output.push_str(&"=".repeat(50));
            output.push('\n');

            let mut labels: Vec<_> = scores.keys().cloned().collect();
            labels.sort();

            for label in labels {
                if let Some(ranker_scores) = scores.get(&label) {
                    output.push_str(&format!("\n{}: ", label));
                    if ranker_scores.is_empty() {
                        output.push_str("(no scores)");
                    } else {
                        output.push('\n');
                        let total_score: f32 = ranker_scores.iter()
                            .map(|(_, score, weight)| score * weight)
                            .sum();

                        for (ranker, score, weight) in ranker_scores {
                            output.push_str(&format!(
                                "  - {}: {:.3} × {:.1} = {:.3}\n",
                                ranker, score, weight, score * weight
                            ));
                        }
                        output.push_str(&format!("  Total: {:.3}\n", total_score));
                    }
                }
            }
            output.push_str(&"=".repeat(50));
            output.push('\n');
            output
        }
    }
}

/// Errors if the two options are Some and are not equal.
pub fn some_eq<T: PartialEq + std::fmt::Debug>(
    label: &str, a: Option<T>, b: Option<T>,
) -> Result<(), String> {
    match (a, b) {
        (Some(x), Some(y)) if x != y => {
            Err(format!("{} mismatch: expected {:?}, got {:?}", label, x, y))
        }
        _ => Ok(()),
    }
}

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
