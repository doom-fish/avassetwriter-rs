use serde::{Deserialize, Serialize};

use crate::time::TimeRange;

/// Plain-text caption payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Caption {
    /// Caption text.
    pub text: String,
    /// Caption time range.
    pub time_range: TimeRange,
}

impl Caption {
    /// Construct a caption.
    #[must_use]
    pub fn new(text: impl Into<String>, time_range: TimeRange) -> Self {
        Self {
            text: text.into(),
            time_range,
        }
    }
}

/// A group of captions sharing one enclosing time range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptionGroup {
    /// Group captions.
    pub captions: Vec<Caption>,
    /// Group time range.
    pub time_range: TimeRange,
}

impl CaptionGroup {
    /// Construct a caption group.
    #[must_use]
    pub const fn new(captions: Vec<Caption>, time_range: TimeRange) -> Self {
        Self {
            captions,
            time_range,
        }
    }
}
