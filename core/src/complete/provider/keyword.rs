use super::super::context::{ClauseKind, Context};
use crate::{
    complete::{Completion, CompletionBuilder, CompletionKind},
    lex::{Keyword, TokenKind},
};

pub fn complete(ctx: &Context, builder: &mut CompletionBuilder) {
    for keywords in ctx.spec.resolve_follow_rules(&ctx.cursor.preceding) {
        if keywords.is_empty() {
            continue;
        }
        if !should_emit_keyword(ctx, &keywords) {
            continue;
        }
        let label = format_label(&keywords);
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

fn should_emit_keyword(ctx: &Context, keywords: &[Keyword]) -> bool {
    match ctx.clause {
        ClauseKind::Where => {
            if !matches_top_level_keyword(keywords) {
                return true;
            }
            at_where_clause_boundary(ctx)
        }
        _ => true,
    }
}

fn matches_top_level_keyword(keywords: &[Keyword]) -> bool {
    if keywords.is_empty() {
        return false;
    }

    let first = keywords[0];
    match first {
        Keyword::Union | Keyword::Intersect | Keyword::Except | Keyword::Limit => true,
        _ => false,
    }
}

fn at_where_clause_boundary(ctx: &Context) -> bool {
    let Some(tokens) = clause_tokens(ctx) else {
        return false;
    };
    let Some(last) = tokens.last() else {
        return false;
    };
    match last {
        TokenKind::Identifier | TokenKind::IdentifierQuoted(_) => {
            let prev = tokens
                .len()
                .checked_sub(1)
                .and_then(|idx| tokens.get(idx.saturating_sub(1)));
            matches!(prev, Some(TokenKind::Operator(_)))
        }
        TokenKind::Number | TokenKind::Float | TokenKind::Str | TokenKind::RightParen => true,
        TokenKind::Keyword(kw)
            if matches!(
                kw,
                Keyword::True | Keyword::False | Keyword::Null | Keyword::Unknown
            ) =>
        {
            true
        }
        _ => false,
    }
}

fn clause_tokens<'a>(ctx: &'a Context<'a>) -> Option<&'a [TokenKind]> {
    let idx = ctx
        .cursor
        .preceding
        .iter()
        .rposition(|t| matches!(t, TokenKind::Keyword(Keyword::Where)));
    idx.map(|i| ctx.cursor.preceding.get(i + 1..).unwrap_or(&[]))
}

fn format_label(label: &[Keyword]) -> String {
    label
        .iter()
        .map(|kw| format!("{:?}", kw))
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_uppercase()
}

#[cfg(test)]
mod tests {
    use crate::test_util::{CompletionTest, CompletionTestResult};

    use super::*;

    #[test]
    fn where_clause_excludes_set_ops() {
        let t = case("SELECT ingest.day FROM ingest WHERE day ^");
        t.assert_missing_labels(["UNION", "INTERSECT", "EXCEPT", "LIMIT"]);
    }

    #[test]
    fn where_clause_allows_limit_after_expression() {
        let t = case("SELECT ingest.day FROM ingest WHERE day = '123' ^");
        t.assert_labels_contains(["LIMIT"]);
    }

    fn case(input: &str) -> CompletionTestResult {
        CompletionTest::from_input(input).run_with(complete)
    }
}
