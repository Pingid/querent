use std::collections::HashMap;

use crate::complete::context::IdentKind;
use crate::complete::context::scope::identifier::QualifiedIdent;
use crate::dialect::SpecFunction;
use crate::schema;


#[derive(Debug, Default)]
pub struct Scope<'a> {
    pub projected: Vec<Projection<'a>>,
    pub bindings: Vec<(BindId, Bind<'a>)>,
    pub by_name: HashMap<&'a str, BindId>,
    pub order_by: Vec<QualifiedIdent<'a>>,
    pub group_by: Vec<QualifiedIdent<'a>>,
    pub where_focus: Vec<&'a str>,
}

impl<'a> Scope<'a> {
    pub fn bind(
        &mut self, alias: Option<&'a str>, kind: BindKind<'a>, available: Vec<Projection<'a>>,
    ) -> BindId {
        let id = self.new_bind_id();
        self.bindings.push((
            id,
            Bind {
                kind,
                alias,
                available,
            },
        ));
        if let Some(alias) = alias {
            self.by_name.insert(alias, id);
        }
        id
    }

    pub fn get_bind(&self, id: BindId) -> Option<&(BindId, Bind<'a>)> {
        self.bindings.get(*id)
    }

    pub fn new_bind_id(&self) -> BindId {
        BindId(self.bindings.len())
    }

    pub fn get_bind_by_alias(&self, alias: &'a str) -> Option<&(BindId, Bind<'a>)> {
        self.by_name.get(alias).and_then(|id| self.get_bind(*id))
    }

    pub fn available(&self) -> impl Iterator<Item = &Projection<'a>> {
        self.bindings
            .iter()
            .flat_map(|(_, bind)| bind.available.iter())
    }

    pub fn projected(&self) -> &[Projection<'a>] {
        &self.projected
    }

    pub fn order_by(&self) -> &[QualifiedIdent<'a>] {
        &self.order_by
    }

    pub fn group_by(&self) -> &[QualifiedIdent<'a>] {
        &self.group_by
    }

    pub fn where_focus(&self) -> &[&'a str] {
        &self.where_focus
    }

    pub fn ctes(&self) -> impl Iterator<Item = &'a str> {
        self.bindings
            .iter()
            .filter_map(|(_, bind)| match &bind.kind {
                BindKind::Cte { name, .. } => Some(*name),
                _ => None,
            })
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct BindId(pub usize);
impl std::ops::Deref for BindId {
    type Target = usize;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug)]
pub struct Bind<'a> {
    pub kind: BindKind<'a>,
    pub alias: Option<&'a str>,
    pub available: Vec<Projection<'a>>,
}

#[derive(Debug)]
pub enum BindKind<'a> {
    Cte {
        name: &'a str,
        scope: Box<Scope<'a>>,
    },
    Base {
        name: QualifiedIdent<'a>,
        table: Option<&'a schema::Table>,
    },
    Sub {
        scope: Box<Scope<'a>>,
    },
    Func {
        name: &'a str,
        definition: Option<FunctionRef<'a>>,
        columns: Vec<&'a schema::TableColumn>,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct Projection<'a> {
    /// what SELECT exposes (alias or expression text)
    pub label: QualifiedIdent<'a>,

    // /// Optional alias of the source (e.g., table alias)
    // pub source_alias: Option<&'a str>,
    /// The specific kind of projection with its metadata
    pub kind: ProjectionKind<'a>,
}

impl<'a> Projection<'a> {
    pub fn is_referenceable(&self) -> bool {
        match &self.kind {
            ProjectionKind::Column { .. } => true,
            ProjectionKind::TableFunction { .. } => true,
            ProjectionKind::ScalarFunction { alias, .. } => alias.is_some(),
            ProjectionKind::Literal { alias, .. } => alias.is_some(),
            ProjectionKind::Expression { alias, .. } => alias.is_some(),
            ProjectionKind::Unknown => true,
        }
    }

    pub fn project(&self, label: QualifiedIdent<'a>, alias: Option<&'a str>) -> Option<Self> {
        let mut projection = *self;
        projection.label = match alias {
            Some(alias) => QualifiedIdent::from_slice(IdentKind::Column, &[alias]).unwrap(),
            None => match label.is_wildcard() {
                true => projection.label,
                false => label,
            },
        };
        Some(projection)
    }

    pub fn propagate(&self, source_alias: Option<&'a str>) -> Self {
        let mut projection = *self;
        if let Some(source_alias) = source_alias {
            projection.label =
                QualifiedIdent::from_slice(IdentKind::Column, &[source_alias, self.label.name()])
                    .unwrap();
        }
        projection
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ProjectionKind<'a> {
    /// Column from a table/view/subquery
    Column {
        /// The binding this column comes from (table, subquery, etc.)
        source: Option<BindId>,
        /// Schema information if available
        schema_column: Option<&'a schema::Column>,
    },

    /// Column from a table-valued function
    TableFunction {
        /// The function binding
        source: BindId,
        /// Function's column definition
        column: &'a schema::TableColumn,
        /// Function's schema information if available
        definition: Option<FunctionRef<'a>>,
    },

    /// Result of a scalar function
    ScalarFunction {
        /// Function name
        name: &'a str,
        /// The alias of the function
        alias: Option<&'a str>,
        /// Return type if known
        return_type: Option<schema::DataType>,
    },

    /// Literal value
    Literal {
        /// The literal text as it appears in the query
        text: &'a str,
        /// The alias of the literal
        alias: Option<&'a str>,
        /// Inferred or explicit type
        data_type: Option<schema::DataType>,
    },

    /// Computed expression (e.g., col1 + col2, CASE statements)
    Expression {
        /// The alias of the expression
        alias: Option<&'a str>,
        /// Result type of the expression
        data_type: Option<schema::DataType>,
    },

    /// Unknown identifier (e.g., col1, col2, etc.)
    Unknown,
}

impl<'a> Projection<'a> {
    pub fn data_type(&self) -> Option<schema::DataType> {
        match &self.kind {
            ProjectionKind::Column { schema_column, .. } => schema_column.map(|c| c.data_type),
            ProjectionKind::TableFunction { column, .. } => Some(column.data_type),
            ProjectionKind::ScalarFunction { return_type, .. } => return_type.clone(),
            ProjectionKind::Literal { data_type, .. } => data_type.clone(),
            ProjectionKind::Expression { data_type, .. } => data_type.clone(),
            ProjectionKind::Unknown => None,
        }
    }
    pub fn schema_column(&self) -> Option<&'a schema::Column> {
        match &self.kind {
            ProjectionKind::Column { schema_column, .. } => *schema_column,
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FunctionRef<'schema> {
    Spec(&'schema SpecFunction),
    Schema(&'schema schema::Function),
}

impl<'schema> FunctionRef<'schema> {
    pub fn return_type(&self) -> &'schema schema::FuncReturnType {
        match self {
            FunctionRef::Spec(func) => &func.return_type,
            FunctionRef::Schema(func) => &func.return_type,
        }
    }
    pub fn function_name(&self) -> &str {
        match self {
            FunctionRef::Spec(func) => func.function_name,
            FunctionRef::Schema(func) => func.function_name.as_str(),
        }
    }
    pub fn parameter_types(&self) -> &'schema [schema::DataType] {
        match self {
            FunctionRef::Spec(func) => &func.parameter_types,
            FunctionRef::Schema(func) => &func.parameter_types,
        }
    }
    pub fn description(&self) -> Option<&str> {
        match self {
            FunctionRef::Spec(func) => Some(func.description),
            FunctionRef::Schema(func) => func.description.as_deref(),
        }
    }
}
