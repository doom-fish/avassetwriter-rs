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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_writer_create_failed_includes_message() {
        let err = AVWriterError::WriterCreateFailed("bad path".into());
        assert!(err.to_string().contains("bad path"));
        assert!(err.to_string().contains("create failed"));
    }

    #[test]
    fn display_start_failed_includes_message() {
        let err = AVWriterError::StartFailed("no inputs".into());
        assert!(err.to_string().contains("startWriting failed"));
        assert!(err.to_string().contains("no inputs"));
    }

    #[test]
    fn display_append_failed_includes_message() {
        let err = AVWriterError::AppendFailed("bad buffer".into());
        assert!(err.to_string().contains("append failed"));
    }

    #[test]
    fn display_input_not_ready_has_constant_text() {
        let err = AVWriterError::InputNotReady;
        assert_eq!(err.to_string(), "input not ready for more media data");
    }

    #[test]
    fn display_invalid_argument_includes_message() {
        let err = AVWriterError::InvalidArgument("nul byte".into());
        assert!(err.to_string().contains("invalid argument"));
        assert!(err.to_string().contains("nul byte"));
    }

    #[test]
    fn display_invalid_state_includes_message() {
        let err = AVWriterError::InvalidState("already started".into());
        assert!(err.to_string().contains("invalid state"));
    }

    #[test]
    fn implements_std_error() {
        fn assert_error<E: std::error::Error>(_e: &E) {}
        let err = AVWriterError::InputNotReady;
        assert_error(&err);
    }

    #[test]
    fn equality_compares_messages() {
        let a = AVWriterError::StartFailed("x".into());
        let b = AVWriterError::StartFailed("x".into());
        let c = AVWriterError::StartFailed("y".into());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
