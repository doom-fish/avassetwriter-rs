#![allow(clippy::missing_const_for_fn, clippy::redundant_pub_crate)]

use core::ffi::{c_char, c_void};
use std::ffi::CStr;
use std::panic::AssertUnwindSafe;

use crate::writer::{InputPassDescription, SegmentOutput, SegmentReport, SegmentType};

pub(crate) struct ReadyCallbackState {
    pub callback: Box<dyn FnMut() + Send + 'static>,
}

pub(crate) struct PassDescriptionCallbackState {
    pub callback: Box<dyn FnMut(Option<InputPassDescription>) + Send + 'static>,
}

pub(crate) struct SegmentCallbackState {
    pub callback: Box<dyn FnMut(SegmentOutput) + Send + 'static>,
}

/// Catch and log any panic from a user-supplied callback.
///
/// Panics that escape `extern "C"` trampolines are undefined behaviour.
/// This function mirrors `doom_fish_utils::panic_safe::catch_user_panic`
/// without requiring the optional `doom-fish-utils` dependency.
fn catch_callback_panic<F: FnOnce()>(site: &str, f: F) {
    if let Err(payload) = std::panic::catch_unwind(AssertUnwindSafe(f)) {
        let msg = payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("<non-string panic payload>");
        // The eprintln! write is itself protected in case stderr is broken.
        let _ = std::panic::catch_unwind(AssertUnwindSafe(|| {
            eprintln!("avassetwriter: panic in {site} caught at C ABI boundary: {msg}");
        }));
    }
}

pub(crate) unsafe extern "C" fn ready_callback_trampoline(userdata: *mut c_void) {
    // SAFETY: `userdata` is a `Box<ReadyCallbackState>` raw pointer created in
    // `Writer::on_ready_for_more_media_data` and kept alive for the lifetime of
    // the callback registration. The Swift bridge calls this trampoline serially
    // on its own queue, so no aliasing occurs.
    let state = unsafe { &mut *(userdata.cast::<ReadyCallbackState>()) };
    catch_callback_panic("ready_callback_trampoline", || (state.callback)());
}

pub(crate) unsafe extern "C" fn ready_callback_drop(userdata: *mut c_void) {
    if !userdata.is_null() {
        // SAFETY: `userdata` was created via `Box::into_raw` in
        // `Writer::on_ready_for_more_media_data`; this drop trampoline is
        // called exactly once by the Swift bridge when the registration is
        // torn down.
        drop(unsafe { Box::from_raw(userdata.cast::<ReadyCallbackState>()) });
    }
}

pub(crate) unsafe extern "C" fn pass_description_callback_trampoline(
    payload_json: *const c_char,
    userdata: *mut c_void,
) {
    // SAFETY: `userdata` is a `Box<PassDescriptionCallbackState>` raw pointer
    // kept alive for the lifetime of the multipass callback registration.
    // `payload_json` is either null or a valid NUL-terminated C string owned
    // by the Swift bridge for the duration of this call.
    let state = unsafe { &mut *(userdata.cast::<PassDescriptionCallbackState>()) };
    let payload = if payload_json.is_null() {
        None
    } else {
        let json = unsafe { CStr::from_ptr(payload_json) }.to_string_lossy();
        serde_json::from_str::<InputPassDescription>(&json).ok()
    };
    catch_callback_panic("pass_description_callback_trampoline", || {
        (state.callback)(payload);
    });
}

pub(crate) unsafe extern "C" fn pass_description_callback_drop(userdata: *mut c_void) {
    if !userdata.is_null() {
        // SAFETY: `userdata` was created via `Box::into_raw`; this drop
        // trampoline is called exactly once by the Swift bridge.
        drop(unsafe { Box::from_raw(userdata.cast::<PassDescriptionCallbackState>()) });
    }
}

pub(crate) unsafe extern "C" fn segment_callback_trampoline(
    bytes: *const u8,
    byte_len: usize,
    segment_type: i32,
    report_json: *const c_char,
    userdata: *mut c_void,
) {
    // SAFETY: `userdata` is a `Box<SegmentCallbackState>` raw pointer kept
    // alive for the lifetime of the segmented-writer's callback registration.
    // `bytes` (when non-null) points to `byte_len` bytes valid for this call.
    // `report_json` (when non-null) is a valid NUL-terminated C string for
    // the duration of this call.
    let state = unsafe { &mut *(userdata.cast::<SegmentCallbackState>()) };
    let data = if bytes.is_null() {
        Vec::new()
    } else {
        unsafe { std::slice::from_raw_parts(bytes, byte_len) }.to_vec()
    };
    let report = if report_json.is_null() {
        None
    } else {
        let json = unsafe { CStr::from_ptr(report_json) }.to_string_lossy();
        serde_json::from_str::<SegmentReport>(&json).ok()
    };
    catch_callback_panic("segment_callback_trampoline", || {
        (state.callback)(SegmentOutput {
            data,
            segment_type: SegmentType::from_raw(segment_type),
            report,
        });
    });
}

pub(crate) unsafe extern "C" fn segment_callback_drop(userdata: *mut c_void) {
    if !userdata.is_null() {
        // SAFETY: `userdata` was created via `Box::into_raw`; this drop
        // trampoline is called exactly once by the Swift bridge.
        drop(unsafe { Box::from_raw(userdata.cast::<SegmentCallbackState>()) });
    }
}
