use super::super::context::Context;
use crate::{
    complete::{Completion, CompletionBuilder, CompletionKind, context::ClauseKind},
    lex::TokenKind,
};

pub fn complete(ctx: &Context, builder: &mut CompletionBuilder) {
    if !should_complete(ctx) {
        return;
    }

    for (key, operator) in ctx.spec.operators {
        builder.add(
            Completion::new(
                CompletionKind::Operator,
                key.to_string(),
                ctx.cursor.replace,
                None,
                None,
            ),
            0,
        );
    }
}

fn should_complete(ctx: &Context) -> bool {
    match ctx.clause {
        ClauseKind::Where => {
            if matches!(ctx.cursor.preceding.last(), Some(TokenKind::Operator(_))) {
                return false;
            }
            if matches!(
                ctx.cursor.preceding.last(),
                Some(TokenKind::Str | TokenKind::Number | TokenKind::Float)
            ) && matches!(
                ctx.cursor.preceding.get(ctx.cursor.preceding.len() - 2),
                Some(TokenKind::Operator(_))
            ) {
                return false;
            }
            true
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use crate::test_util::{CompletionTest, CompletionTestResult};

    use super::*;

    #[test]
    fn skips_at_inappropriate_locations() {
        case("SELECT * FROM users WHERE name = ^").assert_empty();
        case("SELECT * FROM users WHERE name = 'cool'").assert_empty();
        // case("SELECT * FROM users WHERE name = 'cool' ^").assert_not_empty();
    }

    #[test]
    fn completes_where_operators() {
        let t = case("SELECT * FROM users WHERE name ^");
        t.assert_labels_contains(["="]);
    }

    fn case(input: &str) -> CompletionTestResult {
        CompletionTest::from_input(input).run_with(complete)
    }
}
