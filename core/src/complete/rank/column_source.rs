use std::collections::HashSet;

use crate::complete::candidate::Candidate;
use crate::complete::candidate::ColumnCandidate;
use crate::complete::context::Context;
use crate::complete::rank::ColumnRanker;

/// Prioritize columns from the same source as other projected columns.
/// Slightly prefer columns not already projected.
pub struct ColumnSourceRank<'a> {
    sources: Option<HashSet<&'a str>>,
    projected: Option<Vec<&'a str>>,
}

impl<'a> ColumnSourceRank<'a> {
    pub const fn new() -> Self {
        Self {
            sources: None,
            projected: None,
        }
    }
}

impl<'a> ColumnRanker<'a> for ColumnSourceRank<'a> {
    fn prepare(&mut self, ctx: &Context<'a>) {
        let mut sources = HashSet::new();
        let mut projected = Vec::new();
        for p in ctx.scope().projected() {
            if let Some(source) = p.label.table() {
                sources.insert(source);
            }
            projected.push(p.label.name());
        }
        self.sources = Some(sources);
        self.projected = Some(projected);
    }
    fn score_column(&self, _: &Context<'_>, cand: &Candidate, col: &ColumnCandidate<'_>) -> f32 {
        if self.projected.as_ref().unwrap().contains(&col.label.name()) {
            return 0.5;
        }
        if let Some(source) = col.ident.parent {
            if self.sources.as_ref().unwrap().contains(source) {
                return 1.0;
            }
        }
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::complete::provider::DefaultProviders;
    use crate::test_complete;
    use crate::test_utils::posts_schema;
    use crate::test_utils::users_schema;

    #[test]
    fn ranks_columns_from_the_same_source_higher() {
        test_complete!("SELECT title, ^" => {
            completers: [DefaultProviders, ColumnSourceRank::new()],
            schemas: [users_schema(), posts_schema()],
            in_order: ["posts.id", "users.id"],
            in_order: ["posts.content", "posts.title"],
            in_order: ["posts.title", "users.id"],
        });
    }
}
