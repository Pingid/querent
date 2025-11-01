use super::super::completion::Completion;
use super::super::completion::CompletionBuilder;
use super::super::completion::CompletionKind;
use super::super::context::ClauseKind;
use super::super::context::Context;
use crate::lex::Assoc;
use crate::lex::Fixity;
use crate::lex::Keyword;
use crate::lex::Operator;
use crate::lex::TokenKind;

pub fn complete(ctx: &mut Context<'_>, builder: &mut CompletionBuilder) {
    let Some(op_ctx) = resolve_context(ctx) else {
        return;
    };

    for (raw_label, operator) in ctx.spec().operators {
        let operator = *operator;
        if !op_ctx.allows(operator) {
            continue;
        }

        builder.add(
            Completion::new(
                CompletionKind::Operator,
                display_label(raw_label, operator),
                ctx.cursor.replace,
                None,
                Some(display_detail(operator)),
            ),
            operator_score(operator),
            None,
        );
    }
}

#[derive(Debug, Clone, Copy)]
struct OperatorContext {
    state: ContextState,
}

#[derive(Debug, Clone, Copy)]
enum ContextState {
    ExpectOperand,
    ExpectOperator { allow_non_assoc: bool },
}

impl OperatorContext {
    fn allows(&self, operator: Operator) -> bool {
        match self.state {
            ContextState::ExpectOperand => matches!(operator.fixity, Fixity::Prefix),
            ContextState::ExpectOperator { allow_non_assoc } => match operator.fixity {
                Fixity::Infix => allow_non_assoc || !matches!(operator.assoc, Assoc::None),
                Fixity::Prefix => false,
            },
        }
    }
}

fn resolve_context(ctx: &Context) -> Option<OperatorContext> {
    if !matches!(ctx.clause.kind, ClauseKind::Where) {
        return None;
    }

    let clause_tokens = clause_tokens(ctx);
    let last = last_relevant_token(clause_tokens);
    let state = match last {
        None => ContextState::ExpectOperand,
        Some(TokenKind::Operator(op)) => match op.fixity {
            Fixity::Prefix => ContextState::ExpectOperand,
            Fixity::Infix => return None,
        },
        Some(TokenKind::LeftParen | TokenKind::Comma) => ContextState::ExpectOperand,
        Some(
            TokenKind::Identifier
            | TokenKind::IdentifierQuoted(_)
            | TokenKind::Number
            | TokenKind::Float
            | TokenKind::Str
            | TokenKind::RightParen,
        ) => ContextState::ExpectOperator {
            allow_non_assoc: allow_non_assoc(clause_tokens),
        },
        Some(TokenKind::Keyword(kw)) => {
            if keyword_is_operand(*kw) {
                ContextState::ExpectOperator {
                    allow_non_assoc: allow_non_assoc(clause_tokens),
                }
            } else {
                ContextState::ExpectOperand
            }
        }
        _ => return None,
    };

    Some(OperatorContext { state })
}

fn clause_tokens<'a>(ctx: &'a Context<'a>) -> &'a [TokenKind] {
    let tokens = &ctx.cursor.preceding;
    match ctx.clause.kind {
        ClauseKind::Where => tokens_after_keyword(tokens, Keyword::Where),
        _ => tokens,
    }
}

fn tokens_after_keyword<'a>(tokens: &'a [TokenKind], keyword: Keyword) -> &'a [TokenKind] {
    let idx = tokens
        .iter()
        .rposition(|token| matches!(token, TokenKind::Keyword(k) if *k == keyword));
    match idx {
        Some(i) => tokens.get(i + 1..).unwrap_or(&[]),
        None => tokens,
    }
}

fn last_relevant_token(tokens: &[TokenKind]) -> Option<&TokenKind> {
    tokens
        .iter()
        .rev()
        .find(|token| !matches!(token, TokenKind::Keyword(Keyword::Where)))
}

fn keyword_is_operand(kw: Keyword) -> bool {
    matches!(
        kw,
        Keyword::Null | Keyword::True | Keyword::False | Keyword::Unknown | Keyword::End
    )
}

fn allow_non_assoc(tokens: &[TokenKind]) -> bool {
    last_infix_operator(tokens)
        .map(|op| !matches!(op.assoc, Assoc::None))
        .unwrap_or(true)
}

fn last_infix_operator(tokens: &[TokenKind]) -> Option<Operator> {
    tokens.iter().rev().find_map(|token| match token {
        TokenKind::Operator(op) if matches!(op.fixity, Fixity::Infix) => Some(*op),
        _ => None,
    })
}

fn display_label(raw: &str, operator: Operator) -> String {
    match (operator.fixity, raw) {
        (Fixity::Prefix, "+u") => "+".to_string(),
        (Fixity::Prefix, "-u") => "-".to_string(),
        _ => raw.to_string(),
    }
}

fn display_detail(o: Operator) -> String {
    format!(
        "{} • {}",
        match o.fixity {
            Fixity::Prefix => "prefix",
            Fixity::Infix => "infix",
        },
        match o.assoc {
            Assoc::Left => "left associative",
            Assoc::Right => "right associative",
            Assoc::None => "non associative",
        }
    )
}

/// Scores common operators higher than less common ones
fn operator_score(operator: Operator) -> i8 {
    use crate::lex::OpTag;
    match operator.semantic_tag {
        OpTag::Eq | OpTag::And | OpTag::Or | OpTag::Lt | OpTag::Lte | OpTag::Gt | OpTag::Gte => 6,

        OpTag::Add
        | OpTag::Sub
        | OpTag::Mul
        | OpTag::Div
        | OpTag::Not
        | OpTag::In
        | OpTag::Like
        | OpTag::Between
        | OpTag::Is
        | OpTag::Overlaps => 5,

        OpTag::Concat
        | OpTag::Mod
        | OpTag::Exp
        | OpTag::Exists
        | OpTag::UnaryPlus
        | OpTag::UnaryMinus => 4,

        OpTag::Ilike
        | OpTag::Regex
        | OpTag::RegexI
        | OpTag::NotRegex
        | OpTag::NotRegexI
        | OpTag::Contains
        | OpTag::ContainedBy
        | OpTag::Overlap
        | OpTag::BitAnd
        | OpTag::BitOr
        | OpTag::BitXor
        | OpTag::Shl
        | OpTag::Shr => 3,

        OpTag::JsonArrow
        | OpTag::JsonArrowText
        | OpTag::JsonPath
        | OpTag::JsonPathText
        | OpTag::JsonGet
        | OpTag::JsonGetText
        | OpTag::JsonKeyExists
        | OpTag::JsonAnyKey
        | OpTag::JsonAllKeys
        | OpTag::JsonPathMatch
        | OpTag::JsonPathBool => 2,

        OpTag::Similar => 2,

        _ => 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::CompletionTest;
    use crate::test_util::CompletionTestResult;

    #[test]
    fn no_completion_after_incomplete_infix() {
        case("SELECT * FROM users WHERE name = ^").assert_empty();
    }

    #[test]
    fn offers_infix_after_operand() {
        let t = case("SELECT * FROM users WHERE name ^");
        t.assert_labels_contains(&["="]);
        t.assert_missing_labels(&["-u"]);
    }

    #[test]
    fn offers_prefix_at_clause_start() {
        let t = case("SELECT * FROM users WHERE ^");
        t.assert_labels_contains(&["NOT", "+", "-"]);
        t.assert_missing_labels(&["="]);
    }

    #[test]
    fn filters_non_associative_after_completed_comparison() {
        let t = case("SELECT * FROM users WHERE name = 'cool' ^");
        t.assert_labels_contains(&["AND"]);
        t.assert_missing_labels(&["="]);
    }

    fn case(input: &str) -> CompletionTestResult {
        CompletionTest::from_input(input).run_with(complete)
    }
}
