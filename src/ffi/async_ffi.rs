//! Raw FFI declarations for async (callback-based) Swift bridge thunks.

use super::abi::{AVWOwnedPayload, AVWString};
use core::ffi::c_void;

use super::extended::DropCallback;

pub type AsyncCb = extern "C" fn(result: *const c_void, error: *const AVWString, ctx: *mut c_void);

/// Callback type used by the ready-stream bridge.
pub type ReadyStreamCb = unsafe extern "C" fn(ctx: *mut c_void);

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

    pub fn av_export_session_compatible_file_types_async(
        session: *mut c_void,
        cb: AsyncCb,
        ctx: *mut c_void,
    );

    /// Legacy ready-stream ABI. `ctx` remains owned by the caller.
    pub fn av_writer_input_ready_stream_subscribe(
        writer: *mut c_void,
        input_id: i32,
        cb: ReadyStreamCb,
        ctx: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut c_void;

    /// Owned-context variant used by the safe Rust stream wrapper.
    ///
    /// Native code invokes `drop_userdata(ctx)` after registration removal and
    /// any active callback complete.
    pub fn av_writer_input_ready_stream_subscribe_owned(
        writer: *mut c_void,
        input_id: i32,
        cb: ReadyStreamCb,
        ctx: *mut c_void,
        drop_userdata: Option<DropCallback>,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut c_void;

    /// Tear down a ready-stream bridge created by
    /// either ready-stream subscribe function.
    pub fn av_writer_input_ready_stream_unsubscribe(handle: *mut c_void);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready_stream_subscribe_signatures_are_stable() {
        let _: unsafe extern "C" fn(
            *mut c_void,
            i32,
            ReadyStreamCb,
            *mut c_void,
            *mut *mut AVWOwnedPayload,
        ) -> *mut c_void = av_writer_input_ready_stream_subscribe;
        let _: unsafe extern "C" fn(
            *mut c_void,
            i32,
            ReadyStreamCb,
            *mut c_void,
            Option<DropCallback>,
            *mut *mut AVWOwnedPayload,
        ) -> *mut c_void = av_writer_input_ready_stream_subscribe_owned;
    }
}
