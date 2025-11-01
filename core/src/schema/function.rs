use crate::schema;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(optional_fields))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub function_name: String,
    pub parameter_types: Vec<schema::DataType>,
    pub return_type: FunctionReturnType,
    pub description: Option<String>,
    pub schema_name: Option<String>,
    pub database_name: Option<String>,
}

/// The return type of a function - either a scalar value or a table
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionReturnType {
    /// Returns a single scalar value (e.g., UPPER(), COUNT())
    Scalar(schema::DataType),

    /// Returns an aggregate value (e.g., SUM(), AVG())
    /// Semantically different from Scalar for completion context
    Aggregate(schema::DataType),

    /// Returns a table with named columns (e.g., UNNEST(), generate_series())
    Table(Vec<TableColumn>),
}

impl FunctionReturnType {
    pub fn data_type(&self) -> Option<schema::DataType> {
        match self {
            FunctionReturnType::Scalar(dt) => Some(*dt),
            FunctionReturnType::Aggregate(dt) => Some(*dt),
            FunctionReturnType::Table(_) => None,
        }
    }
}

/// A column returned by a table-valued function
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableColumn {
    pub column_name: String,
    pub data_type: schema::DataType,
}
