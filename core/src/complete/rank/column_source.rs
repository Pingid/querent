use std::collections::HashSet;

use crate::complete::completion::Candidate;
use crate::complete::completion::ColumnCandidate;
use crate::complete::context::Context;
use crate::complete::rank::ColumnRanker;

/// Prioritize columns from the same source as other projected columns.
/// Slightly prefer columns not already projected.
#[derive(Default)]
pub struct ColumnSourceRank<'a> {
    sources: HashSet<&'a str>,
    projected: Vec<String>,
}

impl<'a> ColumnRanker<'a> for ColumnSourceRank<'a> {
    fn prepare(&mut self, ctx: &Context<'a>) {
        self.sources.clear();
        for p in ctx.resolved_scope().projected() {
            if let Some(source) = p.label.table() {
                self.sources.insert(source);
            }
            self.projected.push(p.label.name().to_string());
        }
    }
    fn score_column(&self, _: &Context<'_>, cand: &Candidate, col: &ColumnCandidate<'_>) -> f32 {
        if self.projected.contains(&col.ident.name().to_string()) {
            return 0.5;
        }
        if let Some(source) = col.scope_alias {
            if self.sources.contains(source) {
                return 1.0;
            }
        }
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::complete::providers::DefaultProviders;
    use crate::test_complete;
    use crate::test_utils::posts_schema;
    use crate::test_utils::users_schema;

    #[test]
    fn ranks_columns_from_the_same_source_higher() {
        test_complete!("SELECT title, ^" => {
            completers: [DefaultProviders, ColumnSourceRank::default()],
            schemas: [users_schema(), posts_schema()],
            in_order: ["posts.id", "users.id"],
            in_order: ["posts.content", "posts.title"],
            in_order: ["posts.title", "users.id"],
        });
    }
}
