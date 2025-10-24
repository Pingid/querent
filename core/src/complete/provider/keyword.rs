use crate::complete::completion::Completion;
use crate::complete::completion::CompletionBuilder;
use crate::complete::completion::CompletionKind;
use crate::complete::context::Context;
use crate::lex::Keyword;
use crate::lex::TokenKind;

pub fn complete(ctx: &Context, builder: &mut CompletionBuilder) {
    let follow_tokens = follow_tokens(ctx);

    for keywords in ctx.spec.resolve_follow_rules(&follow_tokens) {
        let label = format_label(&keywords);
        let score = keyword_score(&keywords);
        builder.add(
            Completion::new(
                CompletionKind::Keyword,
                label,
                ctx.cursor.replace,
                Some(vec![]),
                None,
            ),
            score,
        );
    }
}

fn format_label(label: &[Keyword]) -> String {
    label
        .iter()
        .map(|kw| format!("{:?}", kw))
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_uppercase()
}

fn keyword_score(keywords: &[Keyword]) -> i8 {
    let Some(first) = keywords.first() else {
        return 0;
    };

    match first {
        Keyword::Select | Keyword::From => 10,
        Keyword::With | Keyword::Where => 9,
        Keyword::Insert | Keyword::Update | Keyword::Delete | Keyword::Create => 8,
        Keyword::Alter | Keyword::Drop | Keyword::Merge => 7,
        Keyword::Limit => 1,
        Keyword::Order => 0,
        Keyword::Union | Keyword::Intersect | Keyword::Except => 0,
        _ => 0,
    }
}

fn follow_tokens(ctx: &Context) -> Vec<TokenKind> {
    let mut tokens = ctx.cursor.preceding.clone();
    if ctx.cursor.fragment.is_empty() {
        return tokens;
    }
    if let Some(TokenKind::Identifier | TokenKind::IdentifierQuoted(_)) = tokens.last() {
        tokens.pop();
    }
    tokens
}
