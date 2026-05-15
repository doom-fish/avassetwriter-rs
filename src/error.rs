//! Errors produced by the `AVAssetWriter` bridge.

use core::fmt;

use crate::ffi;

/// Top-level error type returned by all fallible APIs in this crate.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AVWriterError {
    /// `AVAssetWriter` could not be created (file type unrecognised, path
    /// invalid, etc.).
    WriterCreateFailed(String),
    /// `startWriting()` returned false. Usually caused by a configuration
    /// problem with the inputs.
    StartFailed(String),
    /// `AVAssetWriterInput.append()` returned false.
    AppendFailed(String),
    /// `finishWriting` ended in `.failed` or `.cancelled`.
    FinishFailed(String),
    /// `isReadyForMoreMediaData == false` — caller should retry shortly.
    InputNotReady,
    /// Caller passed an invalid argument (NUL byte, unknown file type,
    /// out-of-range input id).
    InvalidArgument(String),
    /// Writer was in an unexpected state (e.g. asked to start twice).
    InvalidState(String),
}

impl fmt::Display for AVWriterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WriterCreateFailed(m) => write!(f, "AVAssetWriter create failed: {m}"),
            Self::StartFailed(m) => write!(f, "startWriting failed: {m}"),
            Self::AppendFailed(m) => write!(f, "append failed: {m}"),
            Self::FinishFailed(m) => write!(f, "finish failed: {m}"),
            Self::InputNotReady => write!(f, "input not ready for more media data"),
            Self::InvalidArgument(m) => write!(f, "invalid argument: {m}"),
            Self::InvalidState(m) => write!(f, "invalid state: {m}"),
        }
    }
}

impl std::error::Error for AVWriterError {}

/// Build an `AVWriterError` from a status code + optional Swift-side message.
///
/// Frees `error_str` after copying its contents.
pub(crate) unsafe fn from_swift(status: i32, error_str: *mut core::ffi::c_char) -> AVWriterError {
    let message = if error_str.is_null() {
        String::new()
    } else {
        let s = core::ffi::CStr::from_ptr(error_str)
            .to_string_lossy()
            .into_owned();
        ffi::avw_string_free(error_str);
        s
    };
    match status {
        ffi::status::WRITER_CREATE_FAILED => AVWriterError::WriterCreateFailed(message),
        ffi::status::APPEND_FAILED => AVWriterError::AppendFailed(message),
        ffi::status::FINISH_FAILED => AVWriterError::FinishFailed(message),
        ffi::status::INPUT_NOT_READY => AVWriterError::InputNotReady,
        ffi::status::INVALID_ARGUMENT => AVWriterError::InvalidArgument(message),
        ffi::status::INVALID_STATE => AVWriterError::InvalidState(message),
        _ => AVWriterError::InvalidState(format!("unknown status {status}: {message}")),
    }
}
