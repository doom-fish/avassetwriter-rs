use serde::{Deserialize, Serialize};

/// A serializable representation of `CMTime`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Time {
    /// A numeric time with `value / timescale` seconds.
    Numeric { value: i64, timescale: i32 },
    /// `kCMTimeInvalid`.
    Invalid,
    /// `kCMTimeIndefinite`.
    Indefinite,
    /// Positive infinity.
    PositiveInfinity,
    /// Negative infinity.
    NegativeInfinity,
}

impl Time {
    /// Construct a numeric time.
    #[must_use]
    pub const fn new(value: i64, timescale: i32) -> Self {
        Self::Numeric { value, timescale }
    }

    /// `kCMTimeInvalid`.
    #[must_use]
    pub const fn invalid() -> Self {
        Self::Invalid
    }

    /// `kCMTimeIndefinite`.
    #[must_use]
    pub const fn indefinite() -> Self {
        Self::Indefinite
    }

    /// Positive infinity.
    #[must_use]
    pub const fn positive_infinity() -> Self {
        Self::PositiveInfinity
    }

    /// Negative infinity.
    #[must_use]
    pub const fn negative_infinity() -> Self {
        Self::NegativeInfinity
    }

    /// Return the numeric `(value, timescale)` pair when available.
    #[must_use]
    pub const fn as_numeric(self) -> Option<(i64, i32)> {
        match self {
            Self::Numeric { value, timescale } => Some((value, timescale)),
            Self::Invalid | Self::Indefinite | Self::PositiveInfinity | Self::NegativeInfinity => {
                None
            }
        }
    }
}

impl From<(i64, i32)> for Time {
    fn from(value: (i64, i32)) -> Self {
        Self::new(value.0, value.1)
    }
}

/// A serializable representation of `CMTimeRange`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeRange {
    /// Start time.
    pub start: Time,
    /// Duration.
    pub duration: Time,
}

impl TimeRange {
    /// Construct a time range.
    #[must_use]
    pub const fn new(start: Time, duration: Time) -> Self {
        Self { start, duration }
    }
}
