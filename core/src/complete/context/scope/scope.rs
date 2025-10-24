use std::collections::HashMap;

use crate::schema;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Scope<'a> {
    /// By alias or relation name -> RelationId (for quick lookup).
    pub by_name: HashMap<&'a str, RelationId>,
    /// All bound relations.
    pub relations: HashMap<RelationId, RelationBinding<'a>>,
    /// Output columns of the innermost SELECT list (post-aliasing).
    pub projected: Vec<BoundColumn<'a>>,
    /// Columns referenced in the GROUP BY clause.
    pub grouped: Vec<BoundColumn<'a>>,
    /// Columns referenced in the ORDER BY clause.
    pub ordered: Vec<BoundColumn<'a>>,
}

/// Accessor methods for Scope.
impl Scope<'_> {
    pub fn relation(&self, name: &str) -> Option<RelationId> {
        self.by_name.get(name).copied()
    }
}

/// Builder methods for Scope.
impl<'a> Scope<'a> {
    pub fn insert_relation(
        &mut self, kind: RelationKind<'a>, alias: Option<&'a str>,
    ) -> RelationId {
        let id = RelationId(self.relations.len() as u32);

        if let RelationKind::Base(path) = &kind
            && let Some(name) = path.0.last()
        {
            self.by_name.insert(*name, id);
        }

        self.relations.insert(
            id,
            RelationBinding {
                id,
                kind,
                alias,
                columns: Vec::new(),
            },
        );

        if let Some(a) = alias {
            self.by_name.insert(a, id);
        }

        id
    }

    pub fn insert_column(
        &mut self, name: &'a str, origin: Origin<'a>, qualifier: Qualifier<'a>,
    ) -> ColumnId {
        let id = ColumnId(self.projected.len() as u32);
        self.projected.push(BoundColumn {
            id,
            name,
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
pub struct ColumnId(pub u32);

/// One bound relation in scope (FROM item or WITH binding).
#[derive(Debug, Clone, PartialEq)]
pub struct RelationBinding<'a> {
    pub id: RelationId,
    pub kind: RelationKind<'a>,
    pub alias: Option<&'a str>,
    pub columns: Vec<BoundColumn<'a>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RelationKind<'a> {
    Base(NamePath<'a>),       // e.g. schema.table
    Cte(Box<Scope<'a>>),      // CTE with its scope
    Subquery(Box<Scope<'a>>), // keep for lazy expansion or rebuild
}

/// Column visible in a relation binding or projection output.
#[derive(Debug, Clone, PartialEq)]
pub struct BoundColumn<'a> {
    pub id: ColumnId,
    pub name: &'a str,      // visible name (incl. alias)
    pub origin: Origin<'a>, // lineage
    pub qualifier: Qualifier<'a>,
}

/// Where a column ultimately comes from; enables lineage & re-type.
#[derive(Debug, Clone, PartialEq)]
pub enum Origin<'a> {
    UnresolvedIdent(NamePath<'a>),
    Constant(Literal),
    /// Directly from a base table column.
    BaseColumn {
        relation: RelationId,
        name: &'a str, // base column name
    },
    /// From another bound column (used for SELECT passthrough, CTEs).
    // FromColumn(ColumnId),
    // /// Computed expression. Keep input lineage for “deep” tracking.
    // DerivedExpr {
    //     expr: Loc<ast::Expr>,
    //     inputs: Vec<Origin>,
    // },
    /// Wildcard expansion marker (resolved to concrete columns upstream).
    Star {
        relation: Option<RelationId>, // None => unqualified *
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Number,
    Float,
    String,
    Boolean,
    Null,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NamePath<'a>(pub Vec<&'a str>);

impl<'a> From<Vec<&'a str>> for NamePath<'a> {
    fn from(path: Vec<&'a str>) -> Self {
        NamePath(path)
    }
}

/// The qualifier used in the source (e.g., "users" in "users.name")
#[derive(Debug, Default, Clone, PartialEq, Hash, Eq)]
pub struct Qualifier<'a> {
    pub schema: Option<&'a str>,
    pub table: Option<&'a str>,
}

impl Qualifier<'_> {
    pub fn is_empty(&self) -> bool {
        self.schema.is_none() && self.table.is_none()
    }
}

impl<'a> From<Vec<&'a str>> for Qualifier<'a> {
    fn from(qualifier: Vec<&'a str>) -> Self {
        let mut q = qualifier;
        let table = q.pop();
        let schema = q.pop();
        Self { schema, table }
    }
}

impl<'a> From<&'a schema::Column> for Qualifier<'a> {
    fn from(col: &'a schema::Column) -> Self {
        Self {
            schema: col.schema_name.as_ref().map(|x| x.as_str()),
            table: col.table_name.as_ref().map(|x| x.as_str()),
        }
    }
}

impl<'a> From<&Vec<&'a str>> for Qualifier<'a> {
    fn from(qualifier: &Vec<&'a str>) -> Self {
        Qualifier::from(qualifier.clone())
    }
}

impl ToString for Qualifier<'_> {
    fn to_string(&self) -> String {
        match (self.schema.as_ref(), self.table.as_ref()) {
            (Some(schema), Some(table)) => format!("{}.{}", schema, table),
            (Some(schema), None) => schema.to_string(),
            (None, Some(table)) => table.to_string(),
            (None, None) => String::new(),
        }
    }
}
