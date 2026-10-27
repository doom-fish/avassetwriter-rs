use apple_cf::cm::CMTime;

pub use crate::binary::{payload_arg, string_arg, take_payload, take_swift_string};

pub const fn time_kind(time: &CMTime) -> i32 {
    if time.is_indefinite() {
        2
    } else if time.is_positive_infinity() {
        3
    } else if time.is_negative_infinity() {
        4
    } else if crate::time::is_numeric(time) {
        0
    } else {
        1
    }
}

pub const fn time_value(time: &CMTime) -> i64 {
    if crate::time::is_numeric(time) {
        time.value
    } else {
        0
    }
}

pub const fn time_scale(time: &CMTime) -> i32 {
    if crate::time::is_numeric(time) {
        time.timescale
    } else {
        0
    }
}
