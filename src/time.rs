use apple_cf::cm::{CMTime, CMTimeRange};

use crate::binary::{Arena, FromAbi, ToAbi};
use crate::bridge_support::{time_kind, time_scale, time_value};
use crate::error::AVWriterError;
use crate::ffi::{AVWTime, AVWTimeRange};

pub struct TimeValue(pub CMTime);

pub const fn is_numeric(time: &CMTime) -> bool {
    time.is_numeric() && time.timescale > 0
}

impl ToAbi for CMTime {
    type Abi = AVWTime;

    fn to_abi(&self, _: &mut Arena) -> Self::Abi {
        AVWTime {
            kind: time_kind(self),
            value: time_value(self),
            timescale: time_scale(self),
        }
    }
}

impl FromAbi for CMTime {
    fn from_abi(raw: &Self::Abi) -> Result<Self, AVWriterError> {
        match raw.kind {
            0 if raw.timescale > 0 => Ok(Self::new(raw.value, raw.timescale)),
            0 | 1 => Ok(Self::INVALID),
            2 => Ok(Self::indefinite()),
            3 => Ok(Self::positive_infinity()),
            4 => Ok(Self::negative_infinity()),
            other => Err(AVWriterError::InvalidState(format!(
                "unknown time kind {other}"
            ))),
        }
    }
}

impl ToAbi for TimeValue {
    type Abi = AVWTime;

    fn to_abi(&self, _: &mut Arena) -> Self::Abi {
        AVWTime {
            kind: time_kind(&self.0),
            value: time_value(&self.0),
            timescale: time_scale(&self.0),
        }
    }
}

impl FromAbi for TimeValue {
    fn from_abi(raw: &Self::Abi) -> Result<Self, AVWriterError> {
        Ok(Self(CMTime::from_abi(raw)?))
    }
}

impl ToAbi for CMTimeRange {
    type Abi = AVWTimeRange;

    fn to_abi(&self, arena: &mut Arena) -> Self::Abi {
        AVWTimeRange {
            start: self.start.to_abi(arena),
            duration: self.duration.to_abi(arena),
        }
    }
}

impl FromAbi for CMTimeRange {
    fn from_abi(raw: &Self::Abi) -> Result<Self, AVWriterError> {
        Ok(Self::new(
            CMTime::from_abi(&raw.start)?,
            CMTime::from_abi(&raw.duration)?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(time: CMTime) -> CMTime {
        let mut arena = Arena::default();
        let range = CMTimeRange::new(time, CMTime::new(1, 30));
        let back = CMTime::from_abi(&time.to_abi(&mut arena)).expect("decode time");
        let range = CMTimeRange::from_abi(&range.to_abi(&mut arena)).expect("decode range");
        let ranges = vec![CMTimeRange::new(CMTime::ZERO, time)];
        let ranges =
            Vec::<CMTimeRange>::from_abi(&ranges.to_abi(&mut arena)).expect("decode ranges");
        let optional = Some(time);
        let optional =
            Option::<CMTime>::from_abi(&optional.to_abi(&mut arena)).expect("decode optional");
        assert_eq!(range.start, back);
        assert_eq!(ranges[0].duration, back);
        assert_eq!(optional, Some(back));
        back
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
        assert!(!is_numeric(&malformed));
        assert_eq!(round_trip(malformed), CMTime::INVALID);
    }

    #[test]
    fn the_wire_format_matches_the_swift_payload() {
        let mut arena = Arena::default();
        let time = CMTime::new(3, 30).to_abi(&mut arena);
        let range = CMTimeRange::new(CMTime::ZERO, CMTime::indefinite()).to_abi(&mut arena);
        let optional = None::<CMTime>.to_abi(&mut arena);
        assert_eq!(time.kind, 0);
        assert_eq!(time.value, 3);
        assert_eq!(time.timescale, 30);
        assert_eq!(range.duration.kind, 2);
        assert!(optional.is_null());
    }
}
