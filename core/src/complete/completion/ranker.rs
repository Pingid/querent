use std::cmp::Ordering;
use strsim::jaro_winkler;

use super::{CompletionKind, PossibleCompletion};

/// Ranker decides how a batch of completions should be ordered for a given `needle`.
pub trait Ranker {
    fn rank(&self, needle: &str, items: Vec<PossibleCompletion>) -> Vec<PossibleCompletion>;
}

/// Default ranker: uses a provided `Scorer` (from `completion.rs`) and
/// applies stable, deterministic ordering:
/// 1) Non-keywords before keywords
/// 2) Exact match before non-exact
/// 3) Prefix match before non-prefix
/// 4) Higher fuzzy score first
/// 5) Label ascending (tie-break)
#[derive(Clone)]
pub struct DefaultRanker<S: Scorer> {
    scorer: S,
}

impl<S: Scorer> DefaultRanker<S> {
    pub fn new(scorer: S) -> Self {
        Self { scorer }
    }
}

impl<S: Scorer> Ranker for DefaultRanker<S> {
    fn rank(&self, needle: &str, mut items: Vec<PossibleCompletion>) -> Vec<PossibleCompletion> {
        // Small optimization: precompute lowercased needle once.
        let needle = needle.to_string();

        // Filter items if there's a needle
        if !needle.is_empty() {
            items.retain(|item| {
                let text_to_match = item.filter_text.as_ref().unwrap_or(&item.insert_text);
                text_to_match
                    .to_lowercase()
                    .starts_with(&needle.to_lowercase())
            });
        }

        items.sort_by(|a, b| compare(a, b, &needle, &self.scorer));
        items.dedup();
        items
    }
}

/// Sorting that mirrors the scorer’s tuple but also keeps keywords last.
fn compare(
    a: &PossibleCompletion,
    b: &PossibleCompletion,
    needle: &str,
    scorer: &impl Scorer,
) -> Ordering {
    let ka = is_keyword(&a.kind);
    let kb = is_keyword(&b.kind);

    ka.cmp(&kb).then_with(|| {
        let sa = scorer.score(a, needle);
        let sb = scorer.score(b, needle);

        // exact desc
        (sb.exact as u8)
            .cmp(&(sa.exact as u8))
            // prefix desc
            .then_with(|| (sb.prefix as u8).cmp(&(sa.prefix as u8)))
            // fuzzy desc
            .then_with(|| sb.fuzzy.partial_cmp(&sa.fuzzy).unwrap_or(Ordering::Equal))
            // label asc
            .then_with(|| a.label.cmp(&b.label))
    })
}

fn is_keyword(kind: &CompletionKind) -> bool {
    matches!(kind, CompletionKind::Keyword)
}

/// Scorer decides how we rank completions for a given `needle`.
/// Return tuple fields are ordered by decreasing priority in `compare_with_scorer`.
pub trait Scorer {
    fn score(&self, c: &PossibleCompletion, needle: &str) -> Score;
}

#[derive(Debug, Clone, PartialEq)]
pub struct Score {
    pub exact: bool,
    pub prefix: bool,
    pub fuzzy: f64,
}

#[derive(Clone, Copy)]
pub struct DefaultScorer;

impl Scorer for DefaultScorer {
    fn score(&self, c: &PossibleCompletion, needle: &str) -> Score {
        // Use filter_text if available, otherwise fall back to insert_text
        let text_to_match = c.filter_text.as_ref().unwrap_or(&c.insert_text);

        let exact = text_to_match.eq_ignore_ascii_case(needle);
        let prefix = text_to_match
            .to_lowercase()
            .starts_with(&needle.to_lowercase());
        let fuzzy = jaro_winkler(&text_to_match.to_lowercase(), &needle.to_lowercase());
        Score {
            exact,
            prefix,
            fuzzy,
        }
    }
}

// ---- tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use crate::complete::TableCompletion;

    use super::super::CompletionKind;
    use super::*;

    fn c(label: &str, kind: CompletionKind) -> PossibleCompletion {
        PossibleCompletion {
            label: label.to_string(),
            insert_text: label.to_string(),
            filter_text: Some(label.to_string()),
            kind,
            commit_characters: vec![],
            score: 0,
        }
    }

    #[test]
    fn non_keywords_before_keywords() {
        let r = DefaultRanker::new(DefaultScorer);
        let items = vec![
            c("SELECT", CompletionKind::Keyword),
            c(
                "users",
                CompletionKind::Table(TableCompletion {
                    qualifier: None,
                    table: None,
                }),
            ),
        ];
        let ranked = r.rank("", items);
        assert_eq!(ranked[0].label, "users");
        assert_eq!(ranked[1].label, "SELECT");
    }
}
