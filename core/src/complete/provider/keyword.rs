use super::super::context::Context;
use crate::complete::{Completion, CompletionBuilder, CompletionKind};

pub fn complete(ctx: &Context, builder: &mut CompletionBuilder) {
    for label in ctx.spec.resolve_follow_rules(&ctx.cursor.preceding) {
        builder.add(
            Completion::new(
                CompletionKind::Keyword,
                label,
                ctx.cursor.replace,
                Some(vec![' ', ';', '\n', '\t']),
                None,
            ),
            0,
        );
    }
}
