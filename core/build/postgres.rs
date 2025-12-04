use crate::build::ansi;

pub fn keywords() -> Vec<String> {
    let mut keywords = ansi::keywords();
    keywords.extend(
        [
            "LATERAL",
            "RETURNING",
            "CONFLICT",
            "DO",
            "NOTHING",
            "MATERIALIZED",
            "INDEX",
            "CONCURRENTLY",
            "DEFERRABLE",
            "INITIALLY",
            "DEFERRED",
            "IMMEDIATE",
            "JSONB",
            "JSON",
        ]
        .iter()
        .map(|s| s.to_string()),
    );
    keywords
}

pub fn operators() -> Vec<(&'static str, u8, &'static str, &'static str, &'static str)> {
    let mut operators = ansi::operators();
    operators.extend([
        // Exponentiation
        ("^", 11, "Exp", "Left", "Infix"),
        // Multiplicative (extra)
        ("%", 10, "Mod", "Left", "Infix"),
        // “Any other operator” tier (regex, bitwise, shifts, containment, JSON, etc.)
        // Regex
        ("~", 8, "Regex", "Left", "Infix"),
        ("!~", 8, "NotRegex", "Left", "Infix"),
        ("~*", 8, "RegexI", "Left", "Infix"),
        ("!~*", 8, "NotRegexI", "Left", "Infix"),
        // Bitwise & shifts
        ("&", 8, "BitAnd", "Left", "Infix"),
        ("|", 8, "BitOr", "Left", "Infix"),
        ("#", 8, "BitXor", "Left", "Infix"),
        ("<<", 8, "Shl", "Left", "Infix"),
        (">>", 8, "Shr", "Left", "Infix"),
        // Range / array containment & overlap
        ("@>", 8, "Contains", "Left", "Infix"),
        ("<@", 8, "ContainedBy", "Left", "Infix"),
        ("&&", 8, "Overlap", "Left", "Infix"),
        // JSON / JSONB
        ("->", 8, "JsonGet", "Left", "Infix"),
        ("->>", 8, "JsonGetText", "Left", "Infix"),
        ("#>", 8, "JsonPath", "Left", "Infix"),
        ("#>>", 8, "JsonPathText", "Left", "Infix"),
        ("?", 8, "JsonKeyExists", "Left", "Infix"),
        ("?|", 8, "JsonAnyKey", "Left", "Infix"),
        ("?&", 8, "JsonAllKeys", "Left", "Infix"),
        ("@?", 8, "JsonPathMatch", "Left", "Infix"),
        ("@@", 8, "JsonPathBool", "Left", "Infix"),
        // Predicates (PG-only)
        ("ILIKE", 7, "Ilike", "None", "Infix"),
        // Type cast (postfix in spirit, but parsed as infix with type RHS)
        ("::", 12, "TypeCast", "Left", "Infix"),
    ]);
    operators
}
