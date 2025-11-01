use crate::complete::context::resolved::ColumnName;
use crate::complete::context::resolved::ResolvedScope;
use crate::complete::context::resolved::TableName;
use crate::complete::context::resolved::identifier::ResolvedFunction;
use crate::schema;

/// Unique handles so aliases & nested bindings are unambiguous.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct BindingId(pub u32);

#[derive(Debug)]
pub struct Binding<'a> {
    pub kind: BindingKind<'a>,
    pub alias: Option<&'a str>,
}

#[derive(Debug)]
pub enum BindingKind<'a> {
    Cte {
        name: &'a str,
        scope: Box<ResolvedScope<'a>>,
    },
    Base {
        name: TableName<'a>,
        table: Option<&'a schema::Table>,
        columns: Vec<ColumnBinding<'a>>,
    },
    Sub {
        scope: Box<ResolvedScope<'a>>,
    },
    Func {
        name: &'a str,
        definition: Option<ResolvedFunction<'a>>,
        columns: Vec<ColumnBinding<'a>>,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct ColumnBinding<'a> {
    pub dt: Option<schema::DataType>,
    pub col: Option<&'a schema::Column>,
    pub name: ColumnName<'a>,
    pub alias: Option<&'a str>,
    pub origin: Option<BindingId>,
}
