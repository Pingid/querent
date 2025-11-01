use std::collections::HashMap;

use crate::ast::AstNode;
use crate::ast::{self};
use crate::complete::context::ParsedStatement;
use crate::dialect::DialectSpec;
use crate::schema;
use crate::span::Loc;

mod binding;
mod identifier;
mod resolver;

use binding::*;
use identifier::*;
use resolver::ScopeResolver;

#[derive(Debug, Default)]
pub struct ResolvedScope<'a> {
    pub projected: Vec<ColumnBinding<'a>>,
    pub by_name: HashMap<&'a str, BindingId>,
    pub bindings: HashMap<BindingId, Binding<'a>>,
}

impl<'a> ResolvedScope<'a> {
    pub fn build<'ast>(
        text: &'a str, schema: &'a schema::Cache, spec: &'a DialectSpec, ast: &'ast Loc<ast::Query>,
    ) -> Self {
        ScopeResolver::new(text, schema, spec, ast).resolve()
    }

    pub fn projected_columns(&self) -> &Vec<ColumnBinding<'a>> {
        &self.projected
    }

    pub fn available_columns(&self) -> Vec<(Option<&'a str>, &ColumnBinding<'a>)> {
        self.bindings
            .values()
            .flat_map(|b| match &b.kind {
                BindingKind::Base { columns, .. } => {
                    columns.iter().map(|c| (b.alias, c)).collect::<Vec<_>>()
                }
                BindingKind::Cte { name, scope } => scope
                    .projected_columns()
                    .iter()
                    .map(|column| (Some(*name), column))
                    .collect::<Vec<_>>(),
                BindingKind::Sub { scope } => scope
                    .projected_columns()
                    .iter()
                    .map(|column| (b.alias, column))
                    .collect::<Vec<_>>(),
                _ => vec![],
            })
            .collect::<Vec<_>>()
    }

    fn bind(&mut self, alias: Option<&'a str>, kind: BindingKind<'a>) -> BindingId {
        let id = BindingId(self.bindings.len() as u32);
        self.bindings.insert(id, Binding { kind, alias });
        if let Some(alias) = alias {
            self.by_name.insert(alias, id);
        }
        id
    }

    fn next_id(&self) -> BindingId {
        BindingId(self.bindings.len() as u32)
    }
}

impl<'a> From<&ParsedStatement<'a>> for ResolvedScope<'a> {
    fn from(params: &ParsedStatement<'a>) -> Self {
        let cursor = params.cursor;
        match ast::Query::find_where_rev(ast::Node::Statement(&params.stmt), |node| {
            node.span().contains_inclusive(cursor)
        }) {
            Some(query) => {
                let text: &'a str = params.text;
                let schema: &'a schema::Cache = params.schema;
                ScopeResolver::new(text, schema, params.spec, query).resolve()
            }
            None => Default::default(),
        }
    }
}
