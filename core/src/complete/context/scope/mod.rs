use crate::ast;
use crate::schema;

mod builder;
mod resolved;
mod scope;

pub use resolved::*;
pub use scope::*;

pub fn resolve_scope<'txt, 'a>(
    text: &'txt str, position: usize, ast: ast::Node<'a>, schema: &'txt schema::Cache,
) -> ResolvedScope<'txt> {
    let scope = builder::build_scope(text, position, ast);
    ResolvedScope::new(scope, schema)
}
