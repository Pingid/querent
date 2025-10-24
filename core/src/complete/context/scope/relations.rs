use std::collections::HashMap;

use crate::schema;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Relations<'a> {
    /// By alias or relation name -> RelationId (for quick lookup).
    pub by_name: HashMap<&'a str, BindingId>,
    /// All bound relations.
    pub bindings: HashMap<BindingId, RelationBinding<'a>>,
    /// Output columns of the innermost SELECT list (post-aliasing).
    pub projected: Vec<BoundColumn<'a>>,
    /// Columns referenced in the GROUP BY clause.
    pub grouped: Vec<BoundColumn<'a>>,
    /// Columns referenced in the ORDER BY clause.
    pub ordered: Vec<BoundColumn<'a>>,
}

/// Accessor methods for [`Relations`].
impl Relations<'_> {
    pub fn relation(&self, name: &str) -> Option<BindingId> {
        self.by_name.get(name).copied()
    }
}

/// Builder methods for [`Relations`].
impl<'a> Relations<'a> {
    pub fn insert_relation(&mut self, kind: BindingKind<'a>, alias: Option<&'a str>) -> BindingId {
        let id = BindingId(self.bindings.len() as u32);

        if let BindingKind::Base(path) = &kind
            && let Some(name) = path.0.last()
        {
            self.by_name.insert(*name, id);
        }

        self.bindings.insert(
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

/// Unique handles so aliases & nested bindings are unambiguous.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct BindingId(u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColumnId(pub u32);

#[derive(Debug, Clone, PartialEq)]
pub struct RelationBinding<'a> {
    pub id: BindingId,
    pub kind: BindingKind<'a>,
    pub alias: Option<&'a str>,
    pub columns: Vec<BoundColumn<'a>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BindingKind<'a> {
    Base(NamePath<'a>),           // e.g. schema.table
    Cte(Box<Relations<'a>>),      // CTE with its relations
    Subquery(Box<Relations<'a>>), // keep for lazy expansion or rebuild
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
        relation: BindingId,
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
        relation: Option<BindingId>, // None => unqualified *
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
