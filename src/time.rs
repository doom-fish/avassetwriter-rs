use apple_cf::cm::{CMTime, CMTimeRange};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum TimePayload {
    Numeric { value: i64, timescale: i32 },
    Invalid,
    Indefinite,
    PositiveInfinity,
    NegativeInfinity,
}

impl From<CMTime> for TimePayload {
    fn from(time: CMTime) -> Self {
        if time.is_indefinite() {
            Self::Indefinite
        } else if time.is_positive_infinity() {
            Self::PositiveInfinity
        } else if time.is_negative_infinity() {
            Self::NegativeInfinity
        } else if is_numeric(&time) {
            Self::Numeric {
                value: time.value,
                timescale: time.timescale,
            }
        } else {
            Self::Invalid
        }
    }
}

impl From<TimePayload> for CMTime {
    fn from(payload: TimePayload) -> Self {
        match payload {
            TimePayload::Numeric { value, timescale } => Self::new(value, timescale),
            TimePayload::Invalid => Self::INVALID,
            TimePayload::Indefinite => Self::indefinite(),
            TimePayload::PositiveInfinity => Self::positive_infinity(),
            TimePayload::NegativeInfinity => Self::negative_infinity(),
        }
    }
}

#[derive(Deserialize)]
pub struct TimeJson(#[serde(with = "self")] pub CMTime);

pub const fn is_numeric(time: &CMTime) -> bool {
    time.is_numeric() && time.timescale > 0
}

pub fn serialize<S: Serializer>(time: &CMTime, serializer: S) -> Result<S::Ok, S::Error> {
    TimePayload::from(*time).serialize(serializer)
}

pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<CMTime, D::Error> {
    TimePayload::deserialize(deserializer).map(CMTime::from)
}

#[derive(Serialize, Deserialize)]
struct RangePayload {
    #[serde(with = "self")]
    start: CMTime,
    #[serde(with = "self")]
    duration: CMTime,
}

impl From<CMTimeRange> for RangePayload {
    fn from(range: CMTimeRange) -> Self {
        Self {
            start: range.start,
            duration: range.duration,
        }
    }
}

impl From<RangePayload> for CMTimeRange {
    fn from(payload: RangePayload) -> Self {
        Self::new(payload.start, payload.duration)
    }
}

pub mod range {
    use apple_cf::cm::CMTimeRange;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::RangePayload;

    pub fn serialize<S: Serializer>(range: &CMTimeRange, serializer: S) -> Result<S::Ok, S::Error> {
        RangePayload::from(*range).serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<CMTimeRange, D::Error> {
        RangePayload::deserialize(deserializer).map(CMTimeRange::from)
    }
}

pub mod ranges {
    use apple_cf::cm::CMTimeRange;
    use serde::{Deserialize, Deserializer, Serializer};

    use super::RangePayload;

    pub fn serialize<S: Serializer>(
        ranges: &[CMTimeRange],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_seq(ranges.iter().copied().map(RangePayload::from))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Vec<CMTimeRange>, D::Error> {
        Vec::<RangePayload>::deserialize(deserializer)
            .map(|ranges| ranges.into_iter().map(CMTimeRange::from).collect())
    }
}

pub mod optional {
    use apple_cf::cm::CMTime;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    use super::TimePayload;

    #[allow(clippy::ref_option)]
    pub fn serialize<S: Serializer>(
        time: &Option<CMTime>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        time.map(TimePayload::from).serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<CMTime>, D::Error> {
        Option::<TimePayload>::deserialize(deserializer).map(|time| time.map(CMTime::from))
    }
}

#[cfg(test)]
mod tests {
    use apple_cf::cm::{CMTime, CMTimeRange};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Probe {
        #[serde(with = "super")]
        time: CMTime,
        #[serde(with = "super::range")]
        range: CMTimeRange,
        #[serde(with = "super::ranges")]
        ranges: Vec<CMTimeRange>,
        #[serde(with = "super::optional", default)]
        optional: Option<CMTime>,
    }

    fn round_trip(time: CMTime) -> CMTime {
        let probe = Probe {
            time,
            range: CMTimeRange::new(time, CMTime::new(1, 30)),
            ranges: vec![CMTimeRange::new(CMTime::ZERO, time)],
            optional: Some(time),
        };
        let json = serde_json::to_string(&probe).expect("serialize");
        let back: Probe = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.range.start, back.time);
        assert_eq!(back.ranges[0].duration, back.time);
        assert_eq!(back.optional, Some(back.time));
        back.time
    }

    #[test]
    fn numeric_times_keep_value_and_timescale() {
        assert_eq!(round_trip(CMTime::new(42, 600)), CMTime::new(42, 600));
    }

    #[test]
    fn special_times_survive_the_bridge_format() {
        for time in [
            CMTime::INVALID,
            CMTime::indefinite(),
            CMTime::positive_infinity(),
            CMTime::negative_infinity(),
        ] {
            assert_eq!(round_trip(time), time);
        }
    }

    #[test]
    fn a_zero_timescale_is_sent_as_invalid() {
        let malformed = CMTime {
            value: 1,
            timescale: 0,
            flags: CMTime::new(1, 1).flags,
            epoch: 0,
        };
        assert!(!super::is_numeric(&malformed));
        assert_eq!(round_trip(malformed), CMTime::INVALID);
    }

    #[test]
    fn the_wire_format_matches_the_swift_payload() {
        let json = serde_json::to_value(Probe {
            time: CMTime::new(3, 30),
            range: CMTimeRange::new(CMTime::ZERO, CMTime::indefinite()),
            ranges: Vec::new(),
            optional: None,
        })
        .expect("serialize");

        assert_eq!(
            json["time"],
            serde_json::json!({"kind": "numeric", "value": 3, "timescale": 30})
        );
        assert_eq!(
            json["range"]["duration"],
            serde_json::json!({"kind": "indefinite"})
        );
        assert_eq!(json["optional"], serde_json::Value::Null);
    }
}
