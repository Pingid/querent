use crate::build::ansi;

pub fn keywords() -> Vec<String> {
    let mut keywords = ansi::keywords();
    // Only add SQLite keywords that exist in the Keyword enum
    keywords.extend(
        [
            "CONFLICT",
            "DEFERRABLE",
            "DEFERRED",
            "DO",
            "IMMEDIATE",
            "INDEX",
            "INITIALLY",
            "RETURNING",
        ]
        .iter()
        .map(|s| s.to_string()),
    );
    keywords
}

pub fn operators() -> Vec<(&'static str, u8, &'static str, &'static str, &'static str)> {
    let mut operators = ansi::operators();
    // Only add SQLite operators that exist in the OpTag enum
    operators.extend([
        // Modulo
        ("%", 7, "Mod", "Left", "Infix"),
        // Bitwise operators (from Postgres)
        ("&", 5, "BitAnd", "Left", "Infix"),
        ("|", 5, "BitOr", "Left", "Infix"),
        ("<<", 5, "Shl", "Left", "Infix"),
        (">>", 5, "Shr", "Left", "Infix"),
        // SQLite uses REGEXP (similar to Postgres Regex)
        ("REGEXP", 4, "Regex", "None", "Infix"),
    ]);
    operators
}
