#![allow(clippy::doc_markdown)]

use serde::{Deserialize, Serialize};

use crate::time::TimeRange;

/// A metadata value supported by the AVAssetWriter bridge payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
#[non_exhaustive]
pub enum MetadataValue {
    /// UTF-8 string value.
    String(String),
    /// Signed integer value.
    Integer(i64),
    /// Floating-point value.
    Float(f64),
    /// Boolean value.
    Boolean(bool),
    /// Raw bytes.
    Data(Vec<u8>),
}

/// A metadata item payload that can be converted to `AVMutableMetadataItem`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataItem {
    /// Metadata identifier raw value, e.g. `mdta/com.apple.quicktime.title`.
    pub identifier: String,
    /// The metadata value.
    pub value: MetadataValue,
    /// Optional CoreMedia metadata base data type raw value.
    pub data_type: Option<String>,
    /// Optional BCP-47 language tag.
    pub extended_language_tag: Option<String>,
    /// Optional locale identifier.
    pub locale_identifier: Option<String>,
}

impl MetadataItem {
    /// Construct a string-valued metadata item.
    #[must_use]
    pub fn string(identifier: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            identifier: identifier.into(),
            value: MetadataValue::String(value.into()),
            data_type: None,
            extended_language_tag: None,
            locale_identifier: None,
        }
    }
}

/// A CoreMedia metadata specification used to build a metadata-track format hint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataSpecification {
    /// Metadata identifier raw value.
    pub identifier: String,
    /// CoreMedia metadata base data type raw value.
    pub data_type: String,
    /// Optional BCP-47 language tag.
    pub extended_language_tag: Option<String>,
}

/// A timed metadata group payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimedMetadataGroup {
    /// Metadata items active for the supplied time range.
    pub items: Vec<MetadataItem>,
    /// The group time range.
    pub time_range: TimeRange,
}
