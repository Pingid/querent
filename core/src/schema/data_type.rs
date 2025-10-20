#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "lowercase", tag = "type")
)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
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
