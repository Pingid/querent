use std::collections::HashMap;

use crate::ast::{self};
use crate::catalog::schema::SimpleType;
use crate::span::Loc;

mod builder;
pub use builder::*;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Scope {
    /// By alias or relation name -> RelationId (for quick lookup).
    pub by_name: HashMap<String, RelationId>,
    /// All bound relations.
    pub relations: HashMap<RelationId, RelationBinding>,
    /// Output columns of the innermost SELECT list (post-aliasing).
    pub projected: Vec<BoundColumn>,
    /// WITH bindings accessible to this query block (name -> RelationId).
    pub ctes: HashMap<String, RelationId>,
    /// Columns referenced in the GROUP BY clause.
    pub grouped: Vec<BoundColumn>,
    /// Columns referenced in the ORDER BY clause.
    pub ordered: Vec<BoundColumn>,
}

/// Accessor methods for Scope.
impl Scope {
    pub fn relation(&self, name: &str) -> Option<RelationId> {
        self.by_name.get(name).copied()
    }
}

/// Builder methods for Scope.
impl Scope {
    pub fn insert_relation(&mut self, kind: RelationKind, alias: Option<String>) -> RelationId {
        let id = RelationId(self.relations.len() as u32);
        self.relations.insert(
            id,
            RelationBinding {
                id,
                kind: kind.clone(),
                alias: alias.clone(),
                columns: Vec::new(),
            },
        );

        if let Some(a) = alias {
            self.by_name.insert(a, id);
        }

        if let RelationKind::Base(path) = &kind
            && let Some(name) = path.0.last()
        {
            self.by_name.insert(name.clone(), id);
        }
        id
    }

    pub fn insert_column(
        &mut self,
        name: String,
        origin: Origin,
        qualifier: Option<String>,
        ty: Option<SimpleType>,
    ) -> ColumnId {
        let id = ColumnId(self.projected.len() as u32);
        self.projected.push(BoundColumn {
            id,
            name,
            ty,
            origin,
            qualifier,
        });
        id
    }
}

/// Unique handles so aliases & nested scopes are unambiguous.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct RelationId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColumnId(u32);

/// One bound relation in scope (FROM item or WITH binding).
#[derive(Debug, Clone, PartialEq)]
pub struct RelationBinding {
    pub id: RelationId,
    pub kind: RelationKind,
    pub alias: Option<String>,
    pub columns: Vec<BoundColumn>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RelationKind {
    Base(NamePath),       // e.g. schema.table
    Cte(Box<Scope>),      // CTE with its scope
    Subquery(Box<Scope>), // keep for lazy expansion or rebuild
}

/// Column visible in a relation binding or projection output.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundColumn {
    pub id: ColumnId,
    pub name: String, // visible name (incl. alias)
    pub ty: Option<SimpleType>,
    pub origin: Origin, // lineage
    /// The qualifier used in the source (e.g., "users" in "users.name")
    pub qualifier: Option<String>,
}

/// Where a column ultimately comes from; enables lineage & re-type.
#[derive(Debug, Clone, PartialEq)]
pub enum Origin {
    UnresolvedIdent(NamePath),
    /// Directly from a base table column.
    BaseColumn {
        relation: RelationId,
        name: String, // base column name
    },
    /// From another bound column (used for SELECT passthrough, CTEs).
    FromColumn(ColumnId),
    /// Computed expression. Keep input lineage for “deep” tracking.
    DerivedExpr {
        expr: Loc<ast::Expr>,
        inputs: Vec<Origin>,
    },
    /// Wildcard expansion marker (resolved to concrete columns upstream).
    Star {
        relation: Option<RelationId>, // None => unqualified *
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamePath(pub Vec<String>);

impl<I, S> From<I> for NamePath
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    fn from(iter: I) -> Self {
        Self(iter.into_iter().map(|s| s.into()).collect())
    }
}
