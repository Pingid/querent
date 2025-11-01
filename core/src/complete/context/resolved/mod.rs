use crate::ast::AstNode;
use crate::ast::{self};
use crate::complete::context::ParsedStatement;
use crate::schema;

mod binding;
mod identifier;
mod resolver;

pub use binding::*;
pub use identifier::*;
use resolver::ScopeGraphBuilder;

impl<'a> From<&ParsedStatement<'a>> for ScopeGraph<'a> {
    fn from(params: &ParsedStatement<'a>) -> Self {
        match ast::Query::find_where_rev(&params.stmt, |node| params.containing_cursor(node.span)) {
            Some(query) => {
                let text: &'a str = params.text;
                let schema: &'a schema::Cache = params.schema;
                ScopeGraphBuilder::build_graph(text, schema, params.spec, query)
            }
            None => Default::default(),
        }
    }
}
