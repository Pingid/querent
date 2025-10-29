//! Determines which tables and columns are visible and accessible for code
//! completion at a specific cursor position in a SQL query.
//!
//! The scope system analyzes the SQL AST to understand the query context and
//! provides completion candidates that are syntactically valid for that
//! position. For example, it knows which tables are in scope after a FROM
//! clause, which columns are accessible in a SELECT list based on joined
//! tables, and which aliases refer to which sources.
//!
//! ## Architecture
//!
//! Scope resolution happens in two phases:
//!
//! 1. **Relation Building** ([`builder::RelationsBuilder`]): Traverses the AST
//!    to extract all table references, joins, CTEs, and subqueries that are
//!    syntactically visible at the cursor position. This produces a
//!    [`Relations`] structure containing the raw relational context.
//!
//! 2. **Scope Resolution** ([`Scope`]): Takes the relations and
//!    [`schema::Cache`] to resolve qualified names, expand wildcards, and
//!    provide the final set of accessible tables and their columns for
//!    completion.

use crate::ast;
use crate::schema;

mod graph;
mod scope;

pub use graph::*;
pub use scope::*;

pub fn resolve_scope<'txt, 'a>(
    text: &'txt str, position: usize, ast: ast::Node<'_>, schema: &'txt schema::Cache,
) -> Scope<'txt> {
    Scope::new(ScopeGraph::build(text, position, ast), schema)
}
