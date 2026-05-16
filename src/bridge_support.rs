use core::ffi::c_char;
use std::ffi::CString;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::error::AVWriterError;
use crate::ffi;
use crate::time::Time;

pub fn serialize_json<T: ?Sized + Serialize>(value: &T) -> Result<String, AVWriterError> {
    serde_json::to_string(value).map_err(|e| {
        AVWriterError::InvalidArgument(format!("failed to serialize json payload: {e}"))
    })
}

pub fn parse_json_ptr<T: for<'de> Deserialize<'de>>(
    ptr: *mut c_char,
    context: &str,
) -> Result<T, AVWriterError> {
    let raw = take_swift_string(ptr).ok_or_else(|| {
        AVWriterError::InvalidState(format!("swift bridge returned no {context} payload"))
    })?;
    serde_json::from_str(&raw).map_err(|e| {
        AVWriterError::InvalidState(format!(
            "failed to decode {context} json from swift bridge: {e}"
        ))
    })
}

pub fn parse_optional_json_string(raw: Option<String>) -> Result<Option<JsonValue>, AVWriterError> {
    raw.map(|value| {
        serde_json::from_str(&value).map_err(|e| {
            AVWriterError::InvalidState(format!("failed to decode nested json payload: {e}"))
        })
    })
    .transpose()
}

pub fn cstring_arg(value: &str, context: &str) -> Result<CString, AVWriterError> {
    CString::new(value)
        .map_err(|e| AVWriterError::InvalidArgument(format!("{context} contained NUL byte: {e}")))
}

pub fn take_swift_string(ptr: *mut c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    unsafe {
        let string = std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned();
        ffi::avw_string_free(ptr);
        Some(string)
    }
}

pub const fn time_kind(time: &Time) -> i32 {
    match time {
        Time::Numeric { .. } => 0,
        Time::Invalid => 1,
        Time::Indefinite => 2,
        Time::PositiveInfinity => 3,
        Time::NegativeInfinity => 4,
    }
}

pub const fn time_value(time: &Time) -> i64 {
    match time {
        Time::Numeric { value, .. } => *value,
        Time::Invalid | Time::Indefinite | Time::PositiveInfinity | Time::NegativeInfinity => 0,
    }
}

pub const fn time_scale(time: &Time) -> i32 {
    match time {
        Time::Numeric { timescale, .. } => *timescale,
        Time::Invalid | Time::Indefinite | Time::PositiveInfinity | Time::NegativeInfinity => 0,
    }
}
