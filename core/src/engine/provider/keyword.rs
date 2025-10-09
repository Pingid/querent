use crate::{
    catalog::CatalogRead,
    dialect::{DialectSpec, FollowWord},
    engine::{
        Completion, CompletionKind,
        context::{Context, Location},
    },
    token::Keyword,
};

use super::CompletionProviderSync;

pub struct KeywordProvider;

impl CompletionProviderSync for KeywordProvider {
    fn supports(&self, ctx: &Context) -> bool {
        if matches!(ctx.cursor.location, Location::Start) {
            return true;
        }
        if matches!(ctx.cursor.location, Location::Ident) {
            return true;
        }
        // Space after ident/keyword suggests next keywords
        if let Location::Space(inner) = &ctx.cursor.location {
            match **inner {
                Location::Ident | Location::Keyword => return true,
                Location::Literal => {
                    // Support keywords after literals in CASE expressions
                    // Check if we're in a CASE context (CASE, WHEN, THEN in preceding)
                    return ctx.cursor.preceding.contains(&Keyword::Case)
                        || ctx.cursor.preceding.contains(&Keyword::When)
                        || ctx.cursor.preceding.contains(&Keyword::Then);
                }
                _ => {}
            }
        }
        false
    }
    fn complete(
        &self,
        _catalog: &(dyn CatalogRead + Send + Sync),
        spec: &DialectSpec,
        ctx: &Context,
    ) -> Vec<Completion> {
        // Convert preceding keywords to FollowWord and, when appropriate,
        // include the current keyword token (e.g., after a space following a keyword)
        let mut preceding: Vec<FollowWord> = ctx
            .cursor
            .preceding
            .iter()
            .map(|kw| FollowWord::Keyword(*kw))
            .collect();

        // If the cursor is on a keyword or immediately after a keyword (space),
        // treat that keyword as part of the preceding context to drive follow-ups.
        let after_or_on_keyword = match &ctx.cursor.location {
            Location::Keyword => true,
            Location::Space(inner) => matches!(**inner, Location::Keyword),
            _ => false,
        };
        if after_or_on_keyword {
            if let Some(kw) = ctx.cursor.current_keyword {
                preceding.push(FollowWord::Keyword(kw));
            }
        } else if let Location::Space(inner) = &ctx.cursor.location {
            // For spaces after literals in CASE expressions, we need to include
            // the most recent relevant keyword (like THEN) to get proper follow-ups
            if matches!(**inner, Location::Literal) {
                // Check if we're in a CASE context and include the most recent relevant keyword
                if ctx.cursor.preceding.contains(&Keyword::Case)
                    || ctx.cursor.preceding.contains(&Keyword::When)
                    || ctx.cursor.preceding.contains(&Keyword::Then)
                {
                    // Find the most recent relevant keyword from the preceding list
                    // but only if it's not already the last element
                    if let Some(last_relevant) =
                        ctx.cursor.preceding.iter().rev().find(|&&kw| {
                            matches!(kw, Keyword::Case | Keyword::When | Keyword::Then)
                        })
                    {
                        // Only add if it's not already the last element
                        if preceding.last() != Some(&FollowWord::Keyword(*last_relevant)) {
                            preceding.push(FollowWord::Keyword(*last_relevant));
                        }
                    }
                }
            }
        }

        let follow_keywords = spec.follow_keywords(&preceding);
        follow_keywords
            .iter()
            .filter_map(|phrase| {
                // Each phrase is a slice of FollowWords representing a single or multi-word keyword
                // Only process phrases that contain only Keywords (skip any with Operators)
                let keywords: Vec<Keyword> = phrase
                    .iter()
                    .filter_map(|fw| match fw {
                        FollowWord::Keyword(kw) => Some(*kw),
                        _ => None,
                    })
                    .collect();

                // If the phrase contains non-keywords, skip it
                if keywords.len() != phrase.len() {
                    return None;
                }

                // Build the combined keyword text
                let parts: Vec<String> = keywords
                    .iter()
                    .map(|kw| format!("{:?}", kw).to_uppercase())
                    .collect();

                let label = parts.join(" ");
                let insert_text = format!("{} ", label);

                Some(Completion {
                    label: label.clone(),
                    insert_text,
                    filter_text: Some(label),
                    kind: CompletionKind::Keyword,
                    replace: ctx.cursor.replace,
                    commit_characters: vec![' ', '\n', '\t'],
                })
            })
            .collect()
    }
}
