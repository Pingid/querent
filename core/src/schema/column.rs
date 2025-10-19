use crate::schema;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS), ts(optional_fields))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    pub column_name: String,
    pub table_name: String,
    pub schema_name: Option<String>,
    pub data_type: schema::DataType,
    pub is_nullable: Option<bool>,
}
