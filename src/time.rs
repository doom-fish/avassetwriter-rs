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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_numeric_constructor_preserves_components() {
        let t = Time::new(600, 30);
        assert_eq!(t.as_numeric(), Some((600, 30)));
    }

    #[test]
    fn time_invalid_is_not_numeric() {
        assert_eq!(Time::invalid().as_numeric(), None);
    }

    #[test]
    fn time_indefinite_is_not_numeric() {
        assert_eq!(Time::indefinite().as_numeric(), None);
    }

    #[test]
    fn time_infinities_are_not_numeric() {
        assert_eq!(Time::positive_infinity().as_numeric(), None);
        assert_eq!(Time::negative_infinity().as_numeric(), None);
    }

    #[test]
    fn time_from_tuple_round_trips() {
        let t: Time = (1000, 24).into();
        assert_eq!(t.as_numeric(), Some((1000, 24)));
    }

    #[test]
    fn time_equality_distinguishes_variants() {
        assert_ne!(Time::invalid(), Time::indefinite());
        assert_ne!(Time::positive_infinity(), Time::negative_infinity());
        assert_eq!(Time::new(1, 1), Time::new(1, 1));
    }

    #[test]
    fn time_range_constructor_preserves_components() {
        let start = Time::new(0, 30);
        let duration = Time::new(300, 30);
        let range = TimeRange::new(start, duration);
        assert_eq!(range.start, start);
        assert_eq!(range.duration, duration);
    }

    #[test]
    fn time_serializes_round_trip() {
        let t = Time::new(42, 600);
        let json = serde_json::to_string(&t).unwrap();
        let back: Time = serde_json::from_str(&json).unwrap();
        assert_eq!(t, back);
    }
}
