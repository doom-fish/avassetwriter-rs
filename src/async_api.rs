//! Async API for `avassetwriter`
//!
//! Provides executor-agnostic [`Future`] wrappers around `AVFoundation`
//! completion-handler APIs.  Enable with the `async` Cargo feature.
//!
//! ## Available types
//!
//! | Type | Apple API wrapped |
//! |------|-------------------|
//! | [`AsyncWriter`] / [`WriterFinishFuture`] | `AVAssetWriter.finishWritingWithCompletionHandler:` |
//! | [`AsyncExportSession`] / [`ExportFuture`] | `AVAssetExportSession.exportAsynchronouslyWithCompletionHandler:` (also covers macOS 26+ `export(to:as:isolation:)`) |
//! | [`AsyncExportSession`] / [`CompatibleFileTypesFuture`] | `AVAssetExportSession.determineCompatibleFileTypesWithCompletionHandler:` |
//!
//! ## Tier-2 deferrals
//!
//! The following APIs are multi-fire or stream-like and belong in a Tier-2
//! `Stream` pattern rather than a one-shot `Future`:
//!
//! * `AVAssetWriterInput.requestMediaDataWhenReady(on:using:)` — fires
//!   repeatedly whenever the input becomes ready for more data; use a
//!   channel/stream (Tier 2).
//!
//! ## Not available in SDK
//!
//! * `AVOutputSettingsAssistant.compatibilityTest(forSourceFormat:completionHandler:)` —
//!   this method does **not** exist in the `AVFoundation` SDK.
//!   `AVOutputSettingsAssistant` is a purely synchronous settings-recommendation
//!   class; no completion-handler surface is exposed.
//!
//! ## Example
//!
//! ```rust,no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use std::path::PathBuf;
//! use avassetwriter::{FileType, Writer};
//! use avassetwriter::async_api::AsyncWriter;
//!
//! pollster::block_on(async {
//!     let out = PathBuf::from("out.mp4");
//!     let writer = Writer::create(&out, FileType::Mp4)?;
//!     // … configure inputs, append samples …
//!     AsyncWriter::finish(writer).await?;
//!     Ok::<_, Box<dyn std::error::Error>>(())
//! })
//! # }
//! ```
//!
//! [`Future`]: std::future::Future

use core::ffi::{c_void, CStr};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use doom_fish_utils::completion::{error_from_cstr, AsyncCompletion, AsyncCompletionFuture};

use crate::error::AVWriterError;
use crate::export_session::ExportSession;
use crate::ffi::{self, AsyncCb};
use crate::writer::{FileType, Writer};

// ============================================================================
// WriterFinishFuture — AVAssetWriter.finishWritingWithCompletionHandler:
// ============================================================================

extern "C" fn writer_finish_cb(result: *const c_void, error: *const i8, ctx: *mut c_void) {
    if !error.is_null() {
        let msg = unsafe { error_from_cstr(error) };
        unsafe { AsyncCompletion::<()>::complete_err(ctx, msg) };
    } else if !result.is_null() {
        unsafe { AsyncCompletion::complete_ok(ctx, ()) };
    } else {
        unsafe { AsyncCompletion::<()>::complete_err(ctx, "finishWriting: no result".into()) };
    }
}

/// Future returned by [`AsyncWriter::finish`].
pub struct WriterFinishFuture {
    // Keeps the writer alive for the duration of the async operation.
    // When this future is dropped (cancelled), the writer is released only
    // after the underlying Swift `finishWriting` callback has fired (Swift
    // ARC keeps the object alive through the closure capture).
    _writer: Writer,
    inner: AsyncCompletionFuture<()>,
}

impl core::fmt::Debug for WriterFinishFuture {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("WriterFinishFuture").finish_non_exhaustive()
    }
}

impl Future for WriterFinishFuture {
    type Output = Result<(), AVWriterError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.inner)
            .poll(cx)
            .map(|r| r.map_err(AVWriterError::FinishFailed))
    }
}

/// Async wrapper for `AVAssetWriter`.
///
/// Provides an async [`finish`](AsyncWriter::finish) method that drives
/// `AVAssetWriter.finishWritingWithCompletionHandler:` as a `Future`.
pub struct AsyncWriter;

impl AsyncWriter {
    /// Asynchronously finish writing and flush the media file.
    ///
    /// Consumes `writer` (marks all inputs as finished first), then resolves
    /// when `AVAssetWriter.finishWritingWithCompletionHandler:` fires.
    ///
    /// # Errors
    ///
    /// Returns [`AVWriterError::FinishFailed`] if the writer ends in a
    /// `.failed` or `.cancelled` state.
    #[must_use]
    pub fn finish(writer: Writer) -> WriterFinishFuture {
        let ptr = writer.as_raw_ptr();
        let (future, ctx) = AsyncCompletion::<()>::create();
        // Safety: `ptr` is valid for the lifetime of `writer`, which is moved
        // into `_writer` below and kept alive for the duration of the future.
        // The Swift closure inside `av_writer_finish_async` additionally holds
        // a strong ARC reference to the underlying Swift object, so even if
        // this future is dropped/cancelled before the callback fires, the
        // Swift object remains alive until the callback completes.
        unsafe {
            ffi::av_writer_finish_async(ptr, writer_finish_cb as AsyncCb, ctx);
        }
        WriterFinishFuture {
            _writer: writer,
            inner: future,
        }
    }
}

// ============================================================================
// ExportFuture — AVAssetExportSession.exportAsynchronouslyWithCompletionHandler:
// ============================================================================

extern "C" fn export_cb(result: *const c_void, error: *const i8, ctx: *mut c_void) {
    if !error.is_null() {
        let msg = unsafe { error_from_cstr(error) };
        unsafe { AsyncCompletion::<()>::complete_err(ctx, msg) };
    } else if !result.is_null() {
        unsafe { AsyncCompletion::complete_ok(ctx, ()) };
    } else {
        unsafe { AsyncCompletion::<()>::complete_err(ctx, "exportAsynchronously: no result".into()) };
    }
}

/// Future returned by [`AsyncExportSession::export`].
pub struct ExportFuture {
    inner: AsyncCompletionFuture<()>,
}

impl core::fmt::Debug for ExportFuture {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ExportFuture").finish_non_exhaustive()
    }
}

impl Future for ExportFuture {
    type Output = Result<(), AVWriterError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.inner)
            .poll(cx)
            .map(|r| r.map_err(|msg| AVWriterError::InvalidState(format!("export: {msg}"))))
    }
}

// ============================================================================
// CompatibleFileTypesFuture
// ============================================================================

extern "C" fn compatible_file_types_cb(
    result: *const c_void,
    error: *const i8,
    ctx: *mut c_void,
) {
    if !error.is_null() {
        let msg = unsafe { error_from_cstr(error) };
        unsafe { AsyncCompletion::<Vec<FileType>>::complete_err(ctx, msg) };
        return;
    }

    if result.is_null() {
        unsafe {
            AsyncCompletion::<Vec<FileType>>::complete_err(
                ctx,
                "determineCompatibleFileTypes: no result".into(),
            );
        };
        return;
    }

    // `result` is a heap-allocated *mut c_char JSON string from `ffiString`.
    // We parse it and then free it via `avw_string_free`.
    let json_ptr = result.cast::<core::ffi::c_char>().cast_mut();
    let parse_result = (|| -> Result<Vec<FileType>, String> {
        let s = unsafe { CStr::from_ptr(json_ptr) }
            .to_str()
            .map_err(|e| e.to_string())?;
        let raw: Vec<String> = serde_json::from_str(s).map_err(|e| e.to_string())?;
        Ok(raw
            .iter()
            .filter_map(|s| FileType::from_raw(s))
            .collect())
    })();
    unsafe { ffi::avw_string_free(json_ptr) };

    match parse_result {
        Ok(types) => unsafe { AsyncCompletion::complete_ok(ctx, types) },
        Err(e) => unsafe {
            AsyncCompletion::<Vec<FileType>>::complete_err(
                ctx,
                format!("failed to decode compatible file types: {e}"),
            );
        },
    }
}

/// Future returned by [`AsyncExportSession::compatible_file_types`].
pub struct CompatibleFileTypesFuture {
    inner: AsyncCompletionFuture<Vec<FileType>>,
}

impl core::fmt::Debug for CompatibleFileTypesFuture {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CompatibleFileTypesFuture")
            .finish_non_exhaustive()
    }
}

impl Future for CompatibleFileTypesFuture {
    type Output = Result<Vec<FileType>, AVWriterError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.inner).poll(cx).map(|r| {
            r.map_err(|msg| {
                AVWriterError::InvalidState(format!("compatible file types: {msg}"))
            })
        })
    }
}

// ============================================================================
// AsyncExportSession — facade for AVAssetExportSession async APIs
// ============================================================================

/// Async wrapper for `AVAssetExportSession`.
///
/// Provides async versions of export and file-type discovery operations.
/// The `session` passed to each method must remain valid (not dropped) for
/// the lifetime of the returned future.
///
/// ## Note on macOS 26+ `export(to:as:isolation:)`
///
/// Apple's Swift concurrency projection of `exportAsynchronouslyWithCompletionHandler:`
/// (`export(to:as:isolation:)`, available on macOS 26.0+) is semantically
/// identical to [`AsyncExportSession::export`] — both drive the same
/// underlying completion handler.  Use [`AsyncExportSession::export`] to
/// support all macOS versions ≥ 10.9.
pub struct AsyncExportSession;

impl AsyncExportSession {
    /// Asynchronously export media to the configured output URL.
    ///
    /// The session must be fully configured (output URL, file type, etc.)
    /// before calling this method.  Wraps
    /// `AVAssetExportSession.exportAsynchronouslyWithCompletionHandler:`.
    ///
    /// # Errors
    ///
    /// Returns an error if the export session ends in `.failed` or
    /// `.cancelled` state.
    #[must_use]
    pub fn export(session: &ExportSession) -> ExportFuture {
        let ptr = session.as_raw_ptr();
        let (future, ctx) = AsyncCompletion::<()>::create();
        unsafe {
            ffi::av_export_session_export_async(ptr, export_cb as AsyncCb, ctx);
        }
        ExportFuture { inner: future }
    }

    /// Asynchronously determine the file types compatible with this session.
    ///
    /// Wraps
    /// `AVAssetExportSession.determineCompatibleFileTypesWithCompletionHandler:`.
    ///
    /// # Errors
    ///
    /// Returns an error if the underlying call fails or the result cannot be
    /// decoded.
    #[must_use]
    pub fn compatible_file_types(session: &ExportSession) -> CompatibleFileTypesFuture {
        let ptr = session.as_raw_ptr();
        let (future, ctx) = AsyncCompletion::<Vec<FileType>>::create();
        unsafe {
            ffi::av_export_session_compatible_file_types_async(
                ptr,
                compatible_file_types_cb as AsyncCb,
                ctx,
            );
        }
        CompatibleFileTypesFuture { inner: future }
    }
}
