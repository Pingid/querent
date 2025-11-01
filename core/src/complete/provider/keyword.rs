use crate::complete::completion::Completion;
use crate::complete::completion::CompletionBuilder;
use crate::complete::completion::CompletionKind;
use crate::complete::completion::InsertTextFormat;
use crate::complete::context::Context;
use crate::dialect::rule::Next;
use crate::lex::Keyword;
use crate::lex::TokenKind;

pub fn complete(ctx: &Context, builder: &mut CompletionBuilder) {
    let follow_tokens = follow_tokens(ctx);

    for next in ctx.spec().resolve_follow_rules(&follow_tokens) {
        let is_snippet = match next {
            Next::Seq(seq) => seq.iter().any(|next| matches!(next, Next::Query)),
            _ => false,
        };
        let score = next_score(&next);
        let label = fmt_next(&next, true);
        let mut completion = Completion::new(
            CompletionKind::Keyword,
            label,
            ctx.cursor.replace,
            Some(vec![]),
            None,
        );
        if is_snippet {
            completion.insert_text_format = InsertTextFormat::Snippet;
        }
        builder.add(completion, score, None);
    }
}

fn next_score(next: &Next) -> i8 {
    match next {
        Next::Kw(kw) => keyword_score(*kw),
        Next::KwSeq(kws) => kws.iter().map(|kw| keyword_score(*kw)).max().unwrap_or(0),
        Next::Seq(seq) => seq.iter().map(|next| next_score(next)).max().unwrap_or(0),
        Next::Query => 0,
    }
}

fn keyword_score(keyword: Keyword) -> i8 {
    match keyword {
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

fn fmt_next(next: &Next, capitilize: bool) -> String {
    match next {
        Next::Kw(kw) => fmt_kw(kw, capitilize),
        Next::KwSeq(kws) => kws
            .iter()
            .map(|kw| fmt_kw(kw, capitilize))
            .collect::<Vec<String>>()
            .join(" "),
        Next::Seq(seq) => seq
            .iter()
            .map(|next| fmt_next(next, capitilize))
            .collect::<Vec<String>>()
            .join(" "),
        Next::Query => "($1)".to_string(),
    }
}

fn fmt_kw(kw: &Keyword, capitilize: bool) -> String {
    let kw = format!("{:?}", kw);
    match capitilize {
        true => kw.to_uppercase(),
        false => kw,
    }
}
