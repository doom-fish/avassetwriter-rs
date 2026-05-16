#![allow(clippy::missing_const_for_fn, clippy::redundant_pub_crate)]

use core::ffi::{c_char, c_void};
use std::ffi::CStr;

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

pub(crate) unsafe extern "C" fn ready_callback_trampoline(userdata: *mut c_void) {
    let state = &mut *(userdata.cast::<ReadyCallbackState>());
    (state.callback)();
}

pub(crate) unsafe extern "C" fn ready_callback_drop(userdata: *mut c_void) {
    if !userdata.is_null() {
        drop(Box::from_raw(userdata.cast::<ReadyCallbackState>()));
    }
}

pub(crate) unsafe extern "C" fn pass_description_callback_trampoline(
    payload_json: *const c_char,
    userdata: *mut c_void,
) {
    let state = &mut *(userdata.cast::<PassDescriptionCallbackState>());
    let payload = if payload_json.is_null() {
        None
    } else {
        let json = CStr::from_ptr(payload_json).to_string_lossy();
        serde_json::from_str::<InputPassDescription>(&json).ok()
    };
    (state.callback)(payload);
}

pub(crate) unsafe extern "C" fn pass_description_callback_drop(userdata: *mut c_void) {
    if !userdata.is_null() {
        drop(Box::from_raw(
            userdata.cast::<PassDescriptionCallbackState>(),
        ));
    }
}

pub(crate) unsafe extern "C" fn segment_callback_trampoline(
    bytes: *const u8,
    byte_len: usize,
    segment_type: i32,
    report_json: *const c_char,
    userdata: *mut c_void,
) {
    let state = &mut *(userdata.cast::<SegmentCallbackState>());
    let data = if bytes.is_null() {
        Vec::new()
    } else {
        std::slice::from_raw_parts(bytes, byte_len).to_vec()
    };
    let report = if report_json.is_null() {
        None
    } else {
        let json = CStr::from_ptr(report_json).to_string_lossy();
        serde_json::from_str::<SegmentReport>(&json).ok()
    };
    (state.callback)(SegmentOutput {
        data,
        segment_type: SegmentType::from_raw(segment_type),
        report,
    });
}

pub(crate) unsafe extern "C" fn segment_callback_drop(userdata: *mut c_void) {
    if !userdata.is_null() {
        drop(Box::from_raw(userdata.cast::<SegmentCallbackState>()));
    }
}
