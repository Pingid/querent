use std::collections::HashMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Schema {
    pub name: String,
    pub tables_by_name: HashMap<String, usize>,
    pub tables: Vec<Table>, // keep insertion order for nice listing
    pub functions: Vec<Function>,
    pub table_function_columns: HashMap<String, Vec<Column>>,
}

impl Schema {
    pub fn new(name: String) -> Self {
        Self {
            name,
            tables_by_name: HashMap::new(),
            tables: Vec::new(),
            functions: Vec::new(),
            table_function_columns: HashMap::new(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Table {
    pub name: String,
    pub kind: TableKind,
    pub columns: Vec<Column>,
    pub foreign_keys: Vec<ForeignKey>,
    pub description: Option<String>,
}

impl Table {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: TableKind::Table,
            columns: Vec::new(),
            foreign_keys: Vec::new(),
            description: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum TableKind {
    #[default]
    Table,
    View,
    MaterializedView,
    System,
    External,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Column {
    pub column_name: String,
    pub data_type: Option<SimpleType>,
    pub nullable: bool,
    pub default: Option<String>,
    pub is_pk: bool,
    pub generated: bool,           // computed/identity
    pub collation: Option<String>, // e.g., PostgreSQL collations
    pub comment: Option<String>,
    pub ordinal: Option<u32>,
}

impl Column {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            column_name: name.into(),
            data_type: None,
            nullable: true,
            default: None,
            is_pk: false,
            generated: false,
            collation: None,
            comment: None,
            ordinal: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SimpleType {
    Boolean,
    Integer,
    BigInt,
    Float,
    Double,
    Decimal { precision: u8, scale: u8 },
    Text,
    Varchar { len: Option<u32> },
    Timestamp,
    Date,
    Time,
    Json,
    Bytes,
    Uuid,
    Other(String),
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QualifiedName {
    pub schema: String,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ColumnRef {
    pub table: QualifiedName,
    pub column: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForeignKey {
    pub from: ColumnRef,
    pub to: ColumnRef,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Function {
    pub name: String,
    pub parameter_types: Vec<SimpleType>,
    pub function_type: FunctionType,
    pub description: Option<String>,
    pub return_type: Option<SimpleType>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FunctionType {
    Table,
    Scalar,
    Aggregate,
}
