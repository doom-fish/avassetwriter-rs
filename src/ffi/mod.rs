//! Raw FFI declarations matching `swift-bridge/Sources/AVAssetWriterBridge`.

#![allow(missing_docs)]

use core::ffi::{c_char, c_void};

extern "C" {
    pub fn avw_string_free(s: *mut c_char);

    pub fn av_writer_create(
        path: *const c_char,
        file_type: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_void;

    pub fn av_writer_release(writer: *mut c_void);

    pub fn av_writer_add_video_input_from_sample(
        writer: *mut c_void,
        sample_buffer: *mut c_void,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_add_audio_input_pcm(
        writer: *mut c_void,
        sample_rate: f64,
        channels: u32,
        bits_per_sample: u32,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_append_audio_pcm(
        writer: *mut c_void,
        input_id: i32,
        pcm_bytes: *const u8,
        pcm_byte_count: usize,
        frame_count: usize,
        pts_value: i64,
        pts_timescale: i32,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_start_session(
        writer: *mut c_void,
        source_time_value: i64,
        source_time_scale: i32,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_append_sample(
        writer: *mut c_void,
        input_id: i32,
        sample_buffer: *mut c_void,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_finish(writer: *mut c_void, out_error_message: *mut *mut c_char) -> i32;

    pub fn av_writer_add_video_input_pixel_buffer(
        writer: *mut c_void,
        width: i32,
        height: i32,
        pixel_format_type: u32,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_append_pixel_buffer(
        writer: *mut c_void,
        input_id: i32,
        pixel_buffer: *mut c_void,
        pts_value: i64,
        pts_timescale: i32,
        out_error_message: *mut *mut c_char,
    ) -> i32;
}

pub mod status {
    pub const OK: i32 = 0;
    pub const INVALID_ARGUMENT: i32 = -1;
    pub const WRITER_CREATE_FAILED: i32 = -2;
    pub const INPUT_NOT_READY: i32 = -3;
    pub const APPEND_FAILED: i32 = -4;
    pub const FINISH_FAILED: i32 = -5;
    pub const INVALID_STATE: i32 = -6;
}
