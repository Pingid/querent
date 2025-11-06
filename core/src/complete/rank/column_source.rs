use std::collections::HashSet;

use crate::complete::candidate::Candidate;
use crate::complete::candidate::ColumnCandidate;
use crate::complete::context::Context;
use crate::complete::rank::ColumnRanker;

/// Prioritize columns from the same source as other projected columns.
/// Slightly prefer columns not already projected.
#[derive(Default)]
pub struct ColumnSourceRank<'a> {
    sources: Option<HashSet<&'a str>>,
    projected: Option<Vec<(&'a str, Option<&'a str>)>>, // (column_name, table_name)
}

impl<'a> ColumnRanker<'a> for ColumnSourceRank<'a> {
    fn prepare(&mut self, ctx: &Context<'a>) {
        let mut sources = HashSet::new();
        let mut projected = Vec::new();
        for p in ctx.scope().projected() {
            // Store the table that this projection came from
            // This could be from p.label.table() (if it's qualified in the SELECT)
            // or inferred from the binding
            if let Some(source) = p.label.table() {
                sources.insert(source);
            }
            projected.push((p.label.name(), p.label.table()));
        }
        self.sources = Some(sources);
        self.projected = Some(projected);
    }
    fn score_column(&self, _: &Context<'_>, _: &Candidate, col: &ColumnCandidate<'_>) -> f32 {
        // Get the table this column belongs to
        // For columns with aliases (like u.name where u is an alias for users),
        // col.label.table() gives us the alias "u"
        let col_table = col.label.table().or(col.ident.table());

        // Check if this exact column (name + table) is already projected
        let is_exact_match_projected =
            self.projected
                .as_ref()
                .unwrap()
                .iter()
                .any(|(name, proj_table)| {
                    *name == col.label.name() &&
            // Check if tables match - either exact match or one is an alias of the other
            // When u.name is projected, proj_table is Some("u")
            // When checking u.name column, col_table is Some("u")
            *proj_table == col_table
                });

        // If this exact qualified column is already projected, give it lower priority
        // but not too low - user might want to reference it again
        if is_exact_match_projected {
            return 0.3;
        }

        // Check if this column name is already projected (but possibly from different table)
        let is_name_projected = self
            .projected
            .as_ref()
            .unwrap()
            .iter()
            .any(|(name, _)| *name == col.label.name());

        // Check if this column comes from a source that's already in use
        let from_used_source = if let Some(source) = col_table {
            self.sources.as_ref().unwrap().contains(source)
        } else {
            false
        };

        match (
            is_name_projected,
            from_used_source,
            self.sources.as_ref().unwrap().is_empty(),
        ) {
            // When no columns are projected yet or no sources identified, neutral score
            (_, _, true) => 0.7,
            // Same name already projected but from the source being used - medium low
            (true, true, false) => 0.4,
            // Same name from different source - medium
            (true, false, false) => 0.5,
            // Not projected, from same source - highest priority
            (false, true, false) => 1.0,
            // Not projected, from different source - still good score
            (false, false, false) => 0.6,
        }
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
        // When "title" is selected (which comes from posts table),
        // other posts columns should be ranked higher than users columns,
        // but "title" itself should be ranked low since it's already selected
        test_complete!("SELECT title, ^" => {
            completers: [DefaultProviders, ColumnSourceRank],
            schemas: [users_schema(), posts_schema()],
            // posts.id and posts.content should come before users columns
            in_order: ["posts.id", "users.id"],
            in_order: ["posts.content", "users.id"],
            // posts.title should be ranked low since it's already selected
            in_order: ["users.id", "posts.title"],
        });
    }
}
