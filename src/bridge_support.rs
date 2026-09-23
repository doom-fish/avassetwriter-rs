use core::ffi::c_char;
use std::ffi::CString;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::error::AVWriterError;
use crate::ffi;
use apple_cf::cm::CMTime;

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
