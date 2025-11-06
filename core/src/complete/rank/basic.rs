use strsim::jaro_winkler;

use crate::complete::candidate::Candidate;
use crate::complete::candidate::CandidateKind;
use crate::complete::context::Context;
use crate::complete::rank::Ranker;

pub struct KindMatchRank;
impl<'a> Ranker<'a> for KindMatchRank {
    fn score(&self, _: &Context<'_>, cand: &Candidate) -> f32 {
        match cand.kind {
            CandidateKind::Column(_) => 1.0,      // Columns are usually most relevant
            CandidateKind::Table(_) => 0.85,      // Tables are important but less frequent
            CandidateKind::Function(_) => 0.8,    // Functions are contextual
            CandidateKind::Keyword(_) => 0.75,    // Keywords are structural
            CandidateKind::Operator => 0.6,       // Operators are less commonly completed
        }
    }
}

pub struct IgnoreRank;
impl<'a> Ranker<'a> for IgnoreRank {
    fn score(&self, _: &Context<'_>, cand: &Candidate) -> f32 {
        if cand.completion.label.starts_with("_") {
            return 0.0;
        }
        1.0
    }
}

pub struct TypeCompatRank;
impl<'a> Ranker<'a> for TypeCompatRank {
    fn score(&self, ctx: &Context<'_>, cand: &Candidate) -> f32 {
        if let Some(expected) = ctx.expected_data_type() {
            match &cand.kind {
                CandidateKind::Column(col) => {
                    if col.dt == Some(expected) {
                        return 1.0;
                    }
                }
                CandidateKind::Function(func) => {
                    if func.return_type == Some(expected) {
                        return 1.0;
                    }
                }
                _ => {}
            }
        }
        0.0
    }
}

pub struct ExactMatchRanker;
impl<'a> Ranker<'a> for ExactMatchRanker {
    fn score(&self, ctx: &Context<'_>, cand: &Candidate) -> f32 {
        let needle = ctx.cursor().fragment;
        if needle.is_empty() {
            return 0.0;
        }

        let text_to_match = cand
            .completion
            .filter_text
            .as_ref()
            .unwrap_or(&cand.completion.insert_text);

        if text_to_match.eq_ignore_ascii_case(needle) {
            1.0
        } else {
            0.0
        }
    }
}

/// Ranker that gives high score to prefix matches
pub struct PrefixMatchRanker;
impl<'a> Ranker<'a> for PrefixMatchRanker {
    fn score(&self, ctx: &Context<'_>, cand: &Candidate) -> f32 {
        let needle = ctx.cursor().fragment;
        if needle.is_empty() {
            return 0.0;
        }

        let text_to_match = cand
            .completion
            .filter_text
            .as_ref()
            .unwrap_or(&cand.completion.insert_text);

        let text_lower = text_to_match.to_lowercase();
        let needle_lower = needle.to_lowercase();

        if text_lower.starts_with(&needle_lower) {
            1.0
        } else {
            0.0
        }
    }
}

/// Ranker that scores based on Jaro-Winkler fuzzy string similarity
pub struct FuzzyMatchRanker;
impl<'a> Ranker<'a> for FuzzyMatchRanker {
    fn score(&self, ctx: &Context<'_>, cand: &Candidate) -> f32 {
        let needle = ctx.cursor().fragment;
        if needle.is_empty() {
            return 0.0;
        }

        let text_to_match = cand
            .completion
            .filter_text
            .as_ref()
            .unwrap_or(&cand.completion.insert_text);

        let text_lower = text_to_match.to_lowercase();
        let needle_lower = needle.to_lowercase();

        // jaro_winkler returns a value between 0.0 and 1.0
        jaro_winkler(&text_lower, &needle_lower) as f32
    }
}
