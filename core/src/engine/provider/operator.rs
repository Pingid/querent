use crate::{
    dialect::DialectSpec,
    engine::{Completion, CompletionKind, context},
    token::{Fixity, OpTag},
};

pub fn supports(ctx: &context::Context) -> bool {
    // Suggest operators after identifiers, literals, or after closing parentheses in WHERE/SELECT clauses
    match ctx.clause {
        context::ClauseKind::Where
        | context::ClauseKind::Select
        | context::ClauseKind::GroupBy
        | context::ClauseKind::OrderBy => {
            if let context::Location::Space(inner) = &ctx.cursor.location {
                matches!(
                    **inner,
                    context::Location::Ident
                        | context::Location::Paren
                        | context::Location::Literal
                )
            } else {
                false
            }
        }
        _ => false,
    }
}

pub async fn complete(ctx: &context::Context, spec: &DialectSpec) -> Vec<Completion> {
    let mut completions = Vec::new();

    // Add comparison and logical operators
    for (op_str, op) in spec.operators.entries() {
        // Include infix operators (those that go between operands)
        if matches!(op.fixity, Fixity::Infix) {
            // Create user-friendly labels for multi-word operators
            let label = match op.semantic_tag {
                OpTag::In => "IN",
                OpTag::Like => "LIKE",
                OpTag::Ilike => "ILIKE",
                OpTag::Similar => "SIMILAR TO",
                OpTag::Between => "BETWEEN",
                OpTag::Is => "IS",
                _ => op_str,
            };

            completions.push(Completion {
                label: label.to_string(),
                insert_text: label.to_string(),
                filter_text: Some(label.to_string()),
                kind: CompletionKind::Operator,
                replace: ctx.cursor.replace,
                commit_characters: vec![' '],
            });
        }
    }

    // Add "NOT IN" and "IS NOT" as special cases
    completions.push(Completion {
        label: "NOT IN".to_string(),
        insert_text: "NOT IN".to_string(),
        filter_text: Some("NOT IN".to_string()),
        kind: CompletionKind::Operator,
        replace: ctx.cursor.replace,
        commit_characters: vec![' '],
    });

    completions.push(Completion {
        label: "IS NOT".to_string(),
        insert_text: "IS NOT".to_string(),
        filter_text: Some("IS NOT".to_string()),
        kind: CompletionKind::Operator,
        replace: ctx.cursor.replace,
        commit_characters: vec![' '],
    });

    completions
}
