use crate::{
    complete::{Completion, CompletionKind, context},
    dialect::DialectSpec,
};

pub fn supports(_ctx: &context::Context) -> bool {
    true
}

pub fn complete(ctx: &context::Context, spec: &DialectSpec) -> Vec<Completion> {
    let keywords = spec.resolve_follow_rules(&ctx.cursor.preceding);
    keywords
        .map(|label| Completion {
            label: label.clone(),
            insert_text: label.clone(),
            filter_text: Some(label.clone()),
            kind: CompletionKind::Keyword,
            replace: ctx.cursor.replace,
            commit_characters: vec![' ', '\n', '\t'],
        })
        .collect()
}
