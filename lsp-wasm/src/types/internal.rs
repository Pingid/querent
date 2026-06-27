use querent_core::dialect;

// Define Queries here so we can use TS::decl() on it
#[derive(ts_rs::TS)]
#[ts(optional_fields)]
#[allow(dead_code)]
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct IntrospectionQueries {
    pub functions: Option<String>,
    pub tables: Option<String>,
    pub columns: Option<String>,
}

impl From<dialect::DialectKind> for IntrospectionQueries {
    fn from(kind: dialect::DialectKind) -> Self {
        Self {
            functions: kind.introspect_functions().map(|s| s.to_string()),
            tables: kind.introspect_tables().map(|s| s.to_string()),
            columns: kind.introspect_columns().map(|s| s.to_string()),
        }
    }
}
