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
