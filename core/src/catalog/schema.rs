use std::collections::HashMap;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(optional_fields))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Schema {
    pub name: String,
    pub tables_by_name: HashMap<String, usize>,
    pub tables: Vec<Table>, // keep insertion order for nice listing
    pub functions: Vec<Function>,
    pub table_function_columns: HashMap<String, Vec<Column>>,
    pub columns: HashMap<String, Vec<Column>>,
}

impl Schema {
    pub fn new(name: String) -> Self {
        Self {
            name,
            tables_by_name: HashMap::new(),
            tables: Vec::new(),
            functions: Vec::new(),
            table_function_columns: HashMap::new(),
            columns: HashMap::new(),
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(optional_fields))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Table {
    pub table_schema: String,
    pub table_name: String,
    pub table_type: Option<TableType>,
    pub foreign_keys: Option<Vec<ForeignKey>>,
    pub description: Option<String>,
}

impl Table {
    pub fn new(schema: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            table_schema: schema.into(),
            table_name: name.into(),
            table_type: None,
            foreign_keys: None,
            description: None,
        }
    }
}

#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum TableType {
    #[default]
    Table,
    View,
    MaterializedView,
    System,
    External,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(optional_fields))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Column {
    pub table_schema: String,
    pub table_name: String,
    pub column_name: String,
    pub data_type: Option<SimpleType>,
    pub nullable: Option<bool>,
    pub default: Option<String>,
    pub is_pk: Option<bool>,
    pub generated: Option<bool>,   // computed/identity
    pub collation: Option<String>, // e.g., PostgreSQL collations
    pub comment: Option<String>,
    pub ordinal: Option<u32>,
}

impl Column {
    pub fn new(
        schema: impl Into<String>,
        table: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        Self {
            table_schema: schema.into(),
            table_name: table.into(),
            column_name: name.into(),
            data_type: None,
            nullable: Some(true),
            default: None,
            is_pk: Some(false),
            generated: Some(false),
            collation: None,
            comment: None,
            ordinal: None,
        }
    }
}

#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase", tag = "type")
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum SimpleType {
    Boolean,
    Integer,
    BigInt,
    Float,
    Double,
    Decimal {
        precision: u8,
        scale: u8,
    },
    Text,
    Varchar {
        len: Option<u32>,
    },
    Timestamp,
    Date,
    Time,
    Json,
    Bytes,
    Uuid,
    Other {
        data_type: String,
    },
    #[default]
    Unknown,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(optional_fields))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QualifiedName {
    pub schema: String,
    pub name: String,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(optional_fields))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ColumnRef {
    pub table: QualifiedName,
    pub column: String,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(optional_fields))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForeignKey {
    pub from: ColumnRef,
    pub to: ColumnRef,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(optional_fields))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Function {
    pub name: String,
    pub parameter_types: Option<Vec<SimpleType>>,
    pub function_type: Option<FunctionType>,
    pub description: Option<String>,
    pub return_type: Option<SimpleType>,
}

#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum FunctionType {
    #[default]
    Table,
    Scalar,
    Aggregate,
}
