use crate::schema;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(optional_fields))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub function_name: String,
    pub parameters: Vec<schema::DataType>,
    pub description: Option<String>,
    pub function_type: FunctionType,
}

#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase")
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum FunctionType {
    #[default]
    Table,
    Scalar,
    Aggregate,
}
