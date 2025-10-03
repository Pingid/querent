pub mod ansi;
pub mod postgres;

pub fn generate_keyword_map(keywords: Vec<String>) -> String {
    let mut builder = phf_codegen::Map::new();
    for keyword in keywords {
        let variant = format!(
            "Keyword::{}{}",
            keyword[0..1].to_uppercase(),
            keyword[1..].to_lowercase()
        );
        builder.entry(keyword, variant);
    }
    format!("{}", builder.build())
}

pub fn generate_operator_map(
    operators: Vec<(&'static str, u8, &'static str, &'static str, &'static str)>,
) -> String {
    let mut builder = phf_codegen::Map::new();
    for (text, precedence, tag, assoc, fixity) in operators {
        let variant = format!(
            "Operator::new({}, OpTag::{}, Assoc::{}, Fixity::{})",
            precedence, tag, assoc, fixity
        );
        builder.entry(text, variant);
    }
    format!("{}", builder.build())
}
