use std::{cmp::Ordering, collections::HashSet};
use strsim::jaro_winkler;

use super::{CompletionKind, CompletionWithScore};

/// Ranker decides how a batch of completions should be ordered for a given `needle`.
pub trait Ranker {
    fn rank(&self, needle: &str, items: Vec<CompletionWithScore>) -> Vec<CompletionWithScore>;
}

/// Ranking algorithm:
/// 1) Kind descending
/// 2) Text exact match before non-exact
/// 3) Text prefix match before non-prefix
/// 4) Text fuzzy match descending
/// 5) Score descending
#[derive(Clone)]
pub struct DefaultRanker<S: Comparator> {
    comparator: S,
}

impl Default for DefaultRanker<DefaultComparator> {
    fn default() -> Self {
        Self::new(DefaultComparator)
    }
}

impl<S: Comparator> DefaultRanker<S> {
    pub fn new(comparator: S) -> Self {
        Self { comparator }
    }
}

impl<S: Comparator> Ranker for DefaultRanker<S> {
    fn rank(&self, needle: &str, mut items: Vec<CompletionWithScore>) -> Vec<CompletionWithScore> {
        let needle = needle.to_string();

        items.sort_by(|a, b| self.comparator.compare(a, b, &needle));
        items.retain({
            let mut seen = HashSet::new();
            move |item| seen.insert(item.insert_text.clone())
        });
        items
    }
}

pub trait Comparator {
    fn compare(&self, a: &CompletionWithScore, b: &CompletionWithScore, needle: &str) -> Ordering;
}

pub struct DefaultComparator;

impl Comparator for DefaultComparator {
    fn compare(&self, a: &CompletionWithScore, b: &CompletionWithScore, needle: &str) -> Ordering {
        // Sort by kind descending
        kind_order(&b.kind)
            .cmp(&kind_order(&a.kind))
            // Sort label preceding with "_" to the end
            .then_with(|| a.label.starts_with("_").cmp(&b.label.starts_with("_")))
            // Sort by text
            .then_with(|| compare_text(a, b, needle))
            // Sort by score descending
            .then_with(|| match a.kind == b.kind {
                true => b.score.cmp(&a.score),
                false => Ordering::Equal,
            })
            // Sort by label ascending
            .then_with(|| a.label.cmp(&b.label))
    }
}

fn kind_order(kind: &CompletionKind) -> i8 {
    match kind {
        CompletionKind::Column => 5,
        CompletionKind::Table => 4,
        CompletionKind::Schema => 3,
        CompletionKind::Keyword => 2,
        CompletionKind::Function => 1,
        CompletionKind::Operator => 0,
    }
}

fn compare_text(a: &CompletionWithScore, b: &CompletionWithScore, needle: &str) -> Ordering {
    if needle.is_empty() {
        return Ordering::Equal;
    }

    let (exact_a, prefix_a, _) = score_text(a, needle);
    let (exact_b, prefix_b, _) = score_text(b, needle);

    (exact_b as u8)
        .cmp(&(exact_a as u8))
        // prefix desc
        .then_with(|| (prefix_b as u8).cmp(&(prefix_a as u8)))
    // // fuzzy desc
    // .then_with(|| fuzzy_b.partial_cmp(&fuzzy_a).unwrap_or(Ordering::Equal))
}

fn score_text(a: &CompletionWithScore, needle: &str) -> (bool, bool, f64) {
    let text_to_match = a.filter_text.as_ref().unwrap_or(&a.insert_text);
    let exact = text_to_match.eq_ignore_ascii_case(needle);
    let prefix = text_to_match
        .to_lowercase()
        .starts_with(&needle.to_lowercase());
    let fuzzy = jaro_winkler(&text_to_match.to_lowercase(), &needle.to_lowercase());
    (exact, prefix, fuzzy)
}

// ---- tests ------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::super::{Completion, CompletionKind};
    use super::*;

    fn c(label: &str, kind: CompletionKind, score: i8) -> CompletionWithScore {
        CompletionWithScore {
            completion: Completion {
                label: label.to_string(),
                insert_text: label.to_string(),
                filter_text: Some(label.to_string()),
                kind,
                replace: (0, 0).into(),
                commit_characters: vec![],
                detail: None,
            },
            score,
        }
    }

    #[test]
    fn kind_order() {
        let r = DefaultRanker::new(DefaultComparator);
        let items = vec![
            c("SELECT", CompletionKind::Keyword, 0),
            c("users", CompletionKind::Table, 0),
            c("name", CompletionKind::Column, 0),
        ];
        let ranked = r.rank("", items);
        assert_eq!(ranked[0].label, "name");
        assert_eq!(ranked[1].label, "users");
        assert_eq!(ranked[2].label, "SELECT");
    }

    #[test]
    fn score_order() {
        let r = DefaultRanker::new(DefaultComparator);
        let items = vec![
            c("name", CompletionKind::Column, 10),
            c("users", CompletionKind::Table, 0),
            c("SELECT", CompletionKind::Keyword, 0),
            c("email", CompletionKind::Column, 0),
        ];
        let ranked = r.rank("", items);
        assert_eq!(ranked[0].label, "name");
        assert_eq!(ranked[1].label, "email");
    }

    #[test]
    fn score_order_with_needle() {
        let r = DefaultRanker::new(DefaultComparator);
        let items = vec![
            c("email", CompletionKind::Column, 10),
            c("name", CompletionKind::Column, 0),
            c("users", CompletionKind::Table, 0),
            c("SELECT", CompletionKind::Keyword, 0),
        ];
        let ranked = r.rank("na", items);
        assert_eq!(ranked[0].label, "name");
    }
}
