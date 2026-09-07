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
//! * Dropping an input-ready stream stops delivery into Rust. Once writing has
//!   started, removal marks that input as finished because the native API has
//!   no independent cancellation operation. Registration therefore requires
//!   [`Writer::start_session`] to have succeeded first.
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
}

impl core::fmt::Debug for InputMediaDataReadyStream {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("InputMediaDataReadyStream")
            .field("buffered", &self.buffered_count())
            .finish_non_exhaustive()
    }
}

// SAFETY: `bridge_ptr` is an opaque Swift bridge handle whose native lifetime
// box serializes callback invocation and teardown. `BoundedAsyncStream` is
// itself `Send`.
unsafe impl Send for InputMediaDataReadyStream {}

impl Drop for InputMediaDataReadyStream {
    fn drop(&mut self) {
        if !self.bridge_ptr.is_null() {
            unsafe { ffi::av_writer_input_ready_stream_unsubscribe(self.bridge_ptr) };
            self.bridge_ptr = ptr::null_mut();
        }
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

unsafe extern "C" fn input_media_data_ready_drop(ctx: *mut c_void) {
    if ctx.is_null() {
        return;
    }
    catch_user_panic("input_media_data_ready_drop", || {
        // SAFETY: native ownership is transferred only after successful
        // registration and the Swift lifetime box invokes this exactly once.
        drop(unsafe { Box::from_raw(ctx.cast::<AsyncStreamSender<InputMediaDataReadyEvent>>()) });
    });
}

impl InputMediaDataReadyStream {
    fn subscribe(
        writer: &Writer,
        input_id: InputId,
        capacity: usize,
    ) -> Result<Self, AVWriterError> {
        let (stream, sender) = BoundedAsyncStream::new(capacity);
        let sender_raw = Box::into_raw(Box::new(sender));
        let mut err_msg: *mut c_char = ptr::null_mut();
        let bridge_ptr = unsafe {
            ffi::av_writer_input_ready_stream_subscribe_owned(
                writer.as_raw_ptr(),
                input_id.raw(),
                input_media_data_ready_cb as ReadyStreamCb,
                sender_raw.cast::<c_void>(),
                Some(input_media_data_ready_drop),
                &mut err_msg,
            )
        };
        if bridge_ptr.is_null() {
            unsafe { input_media_data_ready_drop(sender_raw.cast::<c_void>()) };
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        Ok(Self {
            inner: stream,
            bridge_ptr,
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
    /// Returns [`AVWriterError::InvalidState`] if writing has not started or a
    /// ready callback/stream has already been registered for that input.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    struct ReadyContext {
        calls: Arc<AtomicUsize>,
        drops: Option<Arc<AtomicUsize>>,
    }

    impl Drop for ReadyContext {
        fn drop(&mut self) {
            if let Some(drops) = &self.drops {
                drops.fetch_add(1, Ordering::SeqCst);
            }
        }
    }

    unsafe extern "C" fn ready_callback(ctx: *mut c_void) {
        let Some(context) = (unsafe { ctx.cast::<ReadyContext>().as_ref() }) else {
            return;
        };
        context.calls.fetch_add(1, Ordering::SeqCst);
    }

    unsafe extern "C" fn ready_context_drop(ctx: *mut c_void) {
        if !ctx.is_null() {
            drop(unsafe { Box::from_raw(ctx.cast::<ReadyContext>()) });
        }
    }

    fn artifact_path(name: &str) -> PathBuf {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/test-artifacts");
        std::fs::create_dir_all(&dir).expect("create test artifacts");
        dir.join(format!("{name}-{}.m4a", std::process::id()))
    }

    fn create_audio_writer(name: &str) -> (Writer, InputId, PathBuf) {
        let path = artifact_path(name);
        let _ = std::fs::remove_file(&path);
        let writer = Writer::create(&path, FileType::M4a).expect("create writer");
        let input = writer
            .add_audio_input_pcm(48_000.0, 1, 16)
            .expect("add audio input");
        (writer, input, path)
    }

    fn wait_for_callback(calls: &AtomicUsize) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while calls.load(Ordering::SeqCst) == 0 {
            assert!(
                Instant::now() < deadline,
                "ready callback did not fire before deadline"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    #[test]
    fn legacy_ready_subscribe_writes_the_real_error_slot() {
        let (writer, input, path) = create_audio_writer("legacy-ready-failure");
        let calls = Arc::new(AtomicUsize::new(0));
        let context = Box::into_raw(Box::new(ReadyContext {
            calls: Arc::clone(&calls),
            drops: None,
        }));
        let mut error = ptr::null_mut();

        let handle = unsafe {
            ffi::av_writer_input_ready_stream_subscribe(
                writer.as_raw_ptr(),
                input.raw(),
                ready_callback,
                context.cast(),
                &mut error,
            )
        };

        assert!(handle.is_null());
        let message = crate::bridge_support::take_swift_string(error)
            .expect("legacy subscribe should populate its error pointer");
        assert!(message.contains("requires a writing session"));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        drop(unsafe { Box::from_raw(context) });
        drop(writer);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn legacy_ready_subscribe_keeps_context_caller_owned() {
        let (writer, input, path) = create_audio_writer("legacy-ready-owned-by-caller");
        writer
            .start_session((0, 48_000))
            .expect("start writing session");
        let calls = Arc::new(AtomicUsize::new(0));
        let context = Box::into_raw(Box::new(ReadyContext {
            calls: Arc::clone(&calls),
            drops: None,
        }));
        let mut error = ptr::null_mut();

        let handle = unsafe {
            ffi::av_writer_input_ready_stream_subscribe(
                writer.as_raw_ptr(),
                input.raw(),
                ready_callback,
                context.cast(),
                &mut error,
            )
        };
        assert!(!handle.is_null(), "legacy subscribe failed");
        assert!(error.is_null());
        wait_for_callback(&calls);

        unsafe { ffi::av_writer_input_ready_stream_unsubscribe(handle) };
        let calls_after_unsubscribe = calls.load(Ordering::SeqCst);
        drop(unsafe { Box::from_raw(context) });
        std::thread::sleep(Duration::from_millis(10));
        assert_eq!(calls.load(Ordering::SeqCst), calls_after_unsubscribe);

        writer.finish().expect("finish writer");
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn owned_ready_subscribe_drops_context_once_after_teardown() {
        let (writer, input, path) = create_audio_writer("owned-ready-teardown");
        writer
            .start_session((0, 48_000))
            .expect("start writing session");
        let calls = Arc::new(AtomicUsize::new(0));
        let drops = Arc::new(AtomicUsize::new(0));
        let context = Box::into_raw(Box::new(ReadyContext {
            calls: Arc::clone(&calls),
            drops: Some(Arc::clone(&drops)),
        }));
        let mut error = ptr::null_mut();

        let handle = unsafe {
            ffi::av_writer_input_ready_stream_subscribe_owned(
                writer.as_raw_ptr(),
                input.raw(),
                ready_callback,
                context.cast(),
                Some(ready_context_drop),
                &mut error,
            )
        };
        assert!(!handle.is_null(), "owned subscribe failed");
        assert!(error.is_null());
        wait_for_callback(&calls);

        unsafe { ffi::av_writer_input_ready_stream_unsubscribe(handle) };
        assert_eq!(drops.load(Ordering::SeqCst), 1);

        writer.finish().expect("finish writer");
        assert_eq!(drops.load(Ordering::SeqCst), 1);
        let _ = std::fs::remove_file(path);
    }
}
