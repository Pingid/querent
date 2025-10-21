#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase", tag = "type")
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataType {
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
    Null,
    #[default]
    Unknown,
}

impl ToString for DataType {
    fn to_string(&self) -> String {
        match self {
            DataType::Boolean => "boolean".to_string(),
            DataType::Integer => "integer".to_string(),
            DataType::BigInt => "bigint".to_string(),
            DataType::Float => "float".to_string(),
            DataType::Double => "double".to_string(),
            DataType::Decimal { precision, scale } => format!("decimal({}, {})", precision, scale),
            DataType::Text => "text".to_string(),
            DataType::Varchar { len } => format!("varchar({})", len.unwrap_or(0)),
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
