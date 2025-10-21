#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize),
    serde(rename_all = "lowercase")
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataType {
    Boolean,
    Integer,
    BigInt,
    Float,
    Double,
    Decimal,
    Text,
    Varchar,
    Timestamp,
    Date,
    Time,
    Json,
    Bytes,
    Uuid,
    Null,
    #[default]
    Unknown,
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for DataType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let normalized = s.to_lowercase();

        // Check for common type name variations across databases
        Ok(match normalized.as_str() {
            // Boolean types
            "boolean" | "bool" | "bit" => DataType::Boolean,

            // Integer types (32-bit)
            "integer" | "int" | "int4" | "int32" | "mediumint" => DataType::Integer,

            // BigInt types (64-bit)
            "bigint" | "int8" | "int64" | "long" | "bigserial" | "serial8" => DataType::BigInt,

            // Float types (32-bit)
            "float" | "float4" | "real" | "float32" => DataType::Float,

            // Double types (64-bit)
            "double" | "float8" | "double precision" | "float64" => DataType::Double,

            // Decimal/Numeric types
            "decimal" | "numeric" | "number" | "money" => DataType::Decimal,

            // Text types
            "text" | "longtext" | "mediumtext" | "clob" | "ntext" => DataType::Text,

            // Varchar types
            "varchar" | "character varying" | "varchar2" | "nvarchar" | "string" | "char"
            | "character" | "bpchar" => DataType::Varchar,

            // Timestamp types
            "timestamp"
            | "timestamptz"
            | "timestamp with time zone"
            | "timestamp without time zone"
            | "datetime"
            | "datetime2"
            | "smalldatetime" => DataType::Timestamp,

            // Date types
            "date" => DataType::Date,

            // Time types
            "time" | "timetz" | "time with time zone" | "time without time zone" => DataType::Time,

            // JSON types
            "json" | "jsonb" => DataType::Json,

            // Binary types
            "bytes" | "bytea" | "binary" | "varbinary" | "blob" | "longblob" | "mediumblob"
            | "tinyblob" | "image" => DataType::Bytes,

            // UUID types
            "uuid" | "uniqueidentifier" | "guid" => DataType::Uuid,

            // Null
            "null" => DataType::Null,

            // Default to Unknown for unrecognized types
            _ => DataType::Unknown,
        })
    }
}

impl ToString for DataType {
    fn to_string(&self) -> String {
        match self {
            DataType::Boolean => "boolean".to_string(),
            DataType::Integer => "integer".to_string(),
            DataType::BigInt => "bigint".to_string(),
            DataType::Float => "float".to_string(),
            DataType::Double => "double".to_string(),
            DataType::Decimal => "decimal".to_string(),
            DataType::Text => "text".to_string(),
            DataType::Varchar => "varchar".to_string(),
            DataType::Timestamp => "timestamp".to_string(),
            DataType::Date => "date".to_string(),
            DataType::Time => "time".to_string(),
            DataType::Json => "json".to_string(),
            DataType::Bytes => "bytes".to_string(),
            DataType::Uuid => "uuid".to_string(),
            DataType::Null => "null".to_string(),
            DataType::Unknown => "unknown".to_string(),
        }
    }
}
