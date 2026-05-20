//! Async API for `avassetwriter`
//!
//! Provides executor-agnostic [`Future`] and bounded-stream wrappers around
//! `AVFoundation` completion-handler and input-readiness APIs. Enable with the
//! `async` Cargo feature.
//!
//! ## Available types
//!
//! | Type | Apple API wrapped |
//! |------|-------------------|
//! | [`AsyncWriter`] / [`WriterFinishFuture`] | `AVAssetWriter.finishWritingWithCompletionHandler:` |
//! | [`AsyncWriterInput`] / [`InputMediaDataReadyStream`] | `AVAssetWriterInput.requestMediaDataWhenReady(on:using:)` |
//! | [`AsyncExportSession`] / [`ExportFuture`] | `AVAssetExportSession.exportAsynchronouslyWithCompletionHandler:` (also covers macOS 26+ `export(to:as:isolation:)`) |
//! | [`AsyncExportSession`] / [`CompatibleFileTypesFuture`] | `AVAssetExportSession.determineCompatibleFileTypesWithCompletionHandler:` |
//!
//! `InputMediaDataReadyStream` uses `doom-fish-utils::stream::BoundedAsyncStream`.
//! If the consumer falls behind, the oldest queued ready event is dropped so
//! the writer input's callback queue can keep making forward progress.
//!
//! ## Notes
//!
//! * Like the underlying `requestMediaDataWhenReady(on:using:)` registration,
//!   an input can only be wrapped once for the lifetime of that writer/input.
//!   Dropping the Rust stream stops delivery into Rust, but re-registering the
//!   same input still returns `InvalidState`.
//! * `AVOutputSettingsAssistant.compatibilityTest(forSourceFormat:completionHandler:)` —
//!   this method does **not** exist in the `AVFoundation` SDK.
//!   `AVOutputSettingsAssistant` is a purely synchronous settings-recommendation
//!   class; no completion-handler surface is exposed.
//!
//! ## Example
//!
//! ```rust,no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! use avassetwriter::{FileType, Writer};
//! use avassetwriter::async_api::{AsyncWriter, AsyncWriterInput};
//!
//! pollster::block_on(async {
//!     let writer = Writer::create("out.m4a", FileType::M4a)?;
//!     let input = writer.add_audio_input_pcm(48_000.0, 1, 16)?;
//!     writer.start_session((0, 48_000))?;
//!     let ready = AsyncWriterInput::request_media_data_when_ready(&writer, input, 4)?;
//!     let _ = ready.next().await;
//!     let silence = vec![0_u8; 48_000 * 2];
//!     writer.append_audio_pcm(input, &silence, 48_000, (0, 48_000))?;
//!     drop(ready);
//!     AsyncWriter::finish(writer).await?;
//!     Ok::<_, Box<dyn std::error::Error>>(())
//! })
//! # }
//! ```
//!
//! [`Future`]: std::future::Future

use core::ffi::{c_char, c_void, CStr};
use core::ptr;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use doom_fish_utils::completion::{error_from_cstr, AsyncCompletion, AsyncCompletionFuture};
use doom_fish_utils::panic_safe::catch_user_panic;
use doom_fish_utils::stream::{AsyncStreamSender, BoundedAsyncStream, NextItem};

use crate::error::{from_swift, AVWriterError};
use crate::export_session::ExportSession;
use crate::ffi::{self, AsyncCb, ReadyStreamCb};
use crate::writer::{FileType, InputId, Writer};

fn drop_boxed_ptr<T>(raw: &mut *mut T) {
    if !(*raw).is_null() {
        // SAFETY: `*raw` was created by `Box::into_raw` during stream
        // subscription and this path runs at most once before nulling it out.
        unsafe { drop(Box::from_raw(*raw)) };
        *raw = ptr::null_mut();
    }
}

// ============================================================================
// WriterFinishFuture — AVAssetWriter.finishWritingWithCompletionHandler:
// ============================================================================

extern "C" fn writer_finish_cb(result: *const c_void, error: *const i8, ctx: *mut c_void) {
    catch_user_panic("writer_finish_cb", || {
        if !error.is_null() {
            // SAFETY: `error` is a valid NUL-terminated C string owned by the
            // Swift bridge for the duration of this callback.
            let msg = unsafe { error_from_cstr(error) };
            // SAFETY: `ctx` is a valid `AsyncCompletion` context pointer
            // created by `AsyncCompletion::create` and consumed at most once.
            unsafe { AsyncCompletion::<()>::complete_err(ctx, msg) };
        } else if !result.is_null() {
            // SAFETY: same ctx invariant as above.
            unsafe { AsyncCompletion::complete_ok(ctx, ()) };
        } else {
            // SAFETY: same ctx invariant as above.
            unsafe { AsyncCompletion::<()>::complete_err(ctx, "finishWriting: no result".into()) };
        }
    });
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
// InputMediaDataReadyStream — AVAssetWriterInput.requestMediaDataWhenReady
// ============================================================================

/// Event emitted whenever an input becomes ready for more media data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputMediaDataReadyEvent;

/// Async stream of `AVAssetWriterInput.requestMediaDataWhenReady(on:using:)`
/// callbacks for a single writer input.
pub struct InputMediaDataReadyStream {
    inner: BoundedAsyncStream<InputMediaDataReadyEvent>,
    bridge_ptr: *mut c_void,
    sender_raw: *mut AsyncStreamSender<InputMediaDataReadyEvent>,
}

impl core::fmt::Debug for InputMediaDataReadyStream {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("InputMediaDataReadyStream")
            .field("buffered", &self.buffered_count())
            .finish_non_exhaustive()
    }
}

// SAFETY: `bridge_ptr` is an opaque Swift bridge handle. Its teardown waits
// for the dedicated callback queue before `sender_raw` is reclaimed, and
// `BoundedAsyncStream` is itself `Send`.
unsafe impl Send for InputMediaDataReadyStream {}

impl Drop for InputMediaDataReadyStream {
    fn drop(&mut self) {
        if !self.bridge_ptr.is_null() {
            unsafe { ffi::av_writer_input_ready_stream_unsubscribe(self.bridge_ptr) };
            self.bridge_ptr = ptr::null_mut();
        }
        drop_boxed_ptr(&mut self.sender_raw);
    }
}

unsafe extern "C" fn input_media_data_ready_cb(ctx: *mut c_void) {
    catch_user_panic("input_media_data_ready_cb", || {
        let Some(sender) = ctx
            .cast::<AsyncStreamSender<InputMediaDataReadyEvent>>()
            .as_ref()
        else {
            return;
        };
        sender.push(InputMediaDataReadyEvent);
    });
}

impl InputMediaDataReadyStream {
    fn subscribe(
        writer: &Writer,
        input_id: InputId,
        capacity: usize,
    ) -> Result<Self, AVWriterError> {
        let _ = writer.input_ready_for_more_media_data(input_id)?;
        let (stream, sender) = BoundedAsyncStream::new(capacity);
        let mut sender_raw = Box::into_raw(Box::new(sender));
        let mut err_msg: *mut c_char = ptr::null_mut();
        let bridge_ptr = unsafe {
            ffi::av_writer_input_ready_stream_subscribe(
                writer.as_raw_ptr(),
                input_id.raw(),
                input_media_data_ready_cb as ReadyStreamCb,
                sender_raw.cast::<c_void>(),
                &mut err_msg,
            )
        };
        if bridge_ptr.is_null() {
            drop_boxed_ptr(&mut sender_raw);
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        Ok(Self {
            inner: stream,
            bridge_ptr,
            sender_raw,
        })
    }

    /// Asynchronously wait for the next ready notification.
    #[must_use]
    pub const fn next(&self) -> NextItem<'_, InputMediaDataReadyEvent> {
        self.inner.next()
    }

    /// Try to read a buffered ready notification without blocking.
    #[must_use]
    pub fn try_next(&self) -> Option<InputMediaDataReadyEvent> {
        self.inner.try_next()
    }

    /// Number of ready notifications currently buffered.
    #[must_use]
    pub fn buffered_count(&self) -> usize {
        self.inner.buffered_count()
    }
}

/// Async entry points for `AVAssetWriterInput`-style operations routed through
/// [`Writer`] + [`InputId`].
pub struct AsyncWriterInput;

impl AsyncWriterInput {
    /// Subscribe to `AVAssetWriterInput.requestMediaDataWhenReady(on:using:)`
    /// for the specified writer input.
    ///
    /// # Errors
    ///
    /// Returns [`AVWriterError::InvalidState`] if a ready callback/stream has
    /// already been registered for that input.
    pub fn request_media_data_when_ready(
        writer: &Writer,
        input_id: InputId,
        capacity: usize,
    ) -> Result<InputMediaDataReadyStream, AVWriterError> {
        InputMediaDataReadyStream::subscribe(writer, input_id, capacity)
    }
}

// ============================================================================
// ExportFuture — AVAssetExportSession.exportAsynchronouslyWithCompletionHandler:
// ============================================================================

extern "C" fn export_cb(result: *const c_void, error: *const i8, ctx: *mut c_void) {
    catch_user_panic("export_cb", || {
        if !error.is_null() {
            // SAFETY: `error` is a valid NUL-terminated C string owned by the
            // Swift bridge for the duration of this callback.
            let msg = unsafe { error_from_cstr(error) };
            // SAFETY: `ctx` is a valid `AsyncCompletion` context pointer
            // created by `AsyncCompletion::create` and consumed at most once.
            unsafe { AsyncCompletion::<()>::complete_err(ctx, msg) };
        } else if !result.is_null() {
            // SAFETY: same ctx invariant as above.
            unsafe { AsyncCompletion::complete_ok(ctx, ()) };
        } else {
            // SAFETY: same ctx invariant as above.
            unsafe {
                AsyncCompletion::<()>::complete_err(ctx, "exportAsynchronously: no result".into());
            };
        }
    });
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

extern "C" fn compatible_file_types_cb(result: *const c_void, error: *const i8, ctx: *mut c_void) {
    catch_user_panic("compatible_file_types_cb", || {
        if !error.is_null() {
            // SAFETY: `error` is a valid NUL-terminated C string owned by the
            // Swift bridge for the duration of this callback.
            let msg = unsafe { error_from_cstr(error) };
            // SAFETY: `ctx` is a valid `AsyncCompletion` context pointer
            // created by `AsyncCompletion::create` and consumed at most once.
            unsafe { AsyncCompletion::<Vec<FileType>>::complete_err(ctx, msg) };
            return;
        }

        if result.is_null() {
            // SAFETY: same ctx invariant as above.
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
            // SAFETY: `json_ptr` is a valid NUL-terminated heap string produced
            // by the Swift bridge's `ffiString` helper.  We free it below after
            // copying its contents.
            let s = unsafe { CStr::from_ptr(json_ptr) }
                .to_str()
                .map_err(|e| e.to_string())?;
            let raw: Vec<String> = serde_json::from_str(s).map_err(|e| e.to_string())?;
            Ok(raw.iter().filter_map(|s| FileType::from_raw(s)).collect())
        })();
        // SAFETY: `json_ptr` was produced by the Swift `ffiString` helper and
        // must be freed exactly once via `avw_string_free`.
        unsafe { ffi::avw_string_free(json_ptr) };

        match parse_result {
            // SAFETY: same ctx invariant as above.
            Ok(types) => unsafe { AsyncCompletion::complete_ok(ctx, types) },
            Err(e) => unsafe {
                AsyncCompletion::<Vec<FileType>>::complete_err(
                    ctx,
                    format!("failed to decode compatible file types: {e}"),
                );
            },
        }
    });
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
            r.map_err(|msg| AVWriterError::InvalidState(format!("compatible file types: {msg}")))
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
        // SAFETY: `ptr` is a valid ARC-retained `AVAssetExportSession` pointer
        // for the lifetime of `session`.  The Swift bridge holds a strong ARC
        // reference to the session through its closure capture, so the object
        // remains alive until the completion callback fires even if `session`
        // is later dropped.
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
        // SAFETY: `ptr` is a valid ARC-retained `AVAssetExportSession` pointer
        // for the lifetime of `session`.  The Swift bridge holds a strong ARC
        // reference through its closure capture so the object stays alive until
        // the completion callback fires.
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
