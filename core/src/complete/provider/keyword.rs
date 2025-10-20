use super::super::context::Context;
use crate::complete::{CompletionBuilder, CompletionKind, PossibleCompletion};

pub fn complete(ctx: &Context, builder: &mut CompletionBuilder) {
    for label in ctx.spec.resolve_follow_rules(&ctx.cursor.preceding) {
        builder.add(PossibleCompletion {
            label: label.clone(),
            insert_text: label.clone(),
            filter_text: Some(label.clone()),
            kind: CompletionKind::Keyword,
            commit_characters: vec![' ', '\n', '\t'],
            score: 0,
        });
    }
}
