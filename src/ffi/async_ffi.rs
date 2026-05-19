//! Raw FFI declarations for async (callback-based) Swift bridge thunks.

use core::ffi::c_void;

/// Callback type used by all async FFI thunks.
///
/// - `result`: non-null sentinel on success (or a heap JSON `*mut c_char` for
///   `av_export_session_compatible_file_types_async`); null on error.
/// - `error`:  null on success; pointer to a NUL-terminated error message on
///   error (the string is owned by Swift and valid only for the duration of
///   the callback).
/// - `ctx`:    opaque context pointer forwarded verbatim from the call-site.
pub type AsyncCb = extern "C" fn(result: *const c_void, error: *const i8, ctx: *mut c_void);

extern "C" {
    /// Async version of `av_writer_finish`.
    ///
    /// Marks all inputs as finished, starts
    /// `AVAssetWriter.finishWritingWithCompletionHandler:`, and fires `cb`
    /// exactly once when the operation completes.
    pub fn av_writer_finish_async(writer: *mut c_void, cb: AsyncCb, ctx: *mut c_void);

    /// Async version of `av_export_session_export`.
    ///
    /// Calls `AVAssetExportSession.exportAsynchronouslyWithCompletionHandler:`
    /// and fires `cb` exactly once when the operation completes.
    pub fn av_export_session_export_async(session: *mut c_void, cb: AsyncCb, ctx: *mut c_void);

    /// Async version of `av_export_session_compatible_file_types_json`.
    ///
    /// Calls `AVAssetExportSession.determineCompatibleFileTypesWithCompletionHandler:`
    /// and fires `cb` exactly once.  On success `result` is a heap-allocated
    /// JSON `*mut c_char` that must be freed via `avw_string_free`.
    pub fn av_export_session_compatible_file_types_async(
        session: *mut c_void,
        cb: AsyncCb,
        ctx: *mut c_void,
    );
}
