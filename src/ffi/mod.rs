//! Raw FFI declarations matching `swift-bridge/Sources/AVAssetWriterBridge`.

#![allow(missing_docs)]

mod abi;
mod asset;
mod async_ffi;
mod caption;
mod composition;
mod export_session;
mod extended;
mod media_processing;
mod metadata;
mod output_settings;
pub use abi::*;
pub use asset::*;
pub use async_ffi::*;
pub use caption::*;
pub use composition::*;
pub use export_session::*;
pub use extended::*;
pub use media_processing::*;
pub use metadata::*;
pub use output_settings::*;

use core::ffi::c_void;

extern "C" {
    pub fn avw_payload_free(s: *mut AVWOwnedPayload);

    pub fn av_writer_create(
        path: *const AVWString,
        file_type: *const AVWString,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut c_void;

    pub fn av_writer_release(writer: *mut c_void);

    pub fn av_writer_add_video_input_from_sample(
        writer: *mut c_void,
        sample_buffer: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_writer_add_video_input_from_preset(
        writer: *mut c_void,
        preset: *const AVWString,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_writer_set_should_optimize_for_network_use(
        writer: *mut c_void,
        should_optimize: bool,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_writer_set_movie_fragment_interval_seconds(
        writer: *mut c_void,
        seconds: f64,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_writer_add_input_group(
        writer: *mut c_void,
        ids: *const i32,
        count: usize,
        default_id: i32,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> bool;

    pub fn av_writer_add_audio_input_pcm(
        writer: *mut c_void,
        sample_rate: f64,
        channels: u32,
        bits_per_sample: u32,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_writer_append_audio_pcm(
        writer: *mut c_void,
        input_id: i32,
        pcm_bytes: *const u8,
        pcm_byte_count: usize,
        frame_count: usize,
        pts_value: i64,
        pts_timescale: i32,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_writer_start_session(
        writer: *mut c_void,
        source_time_value: i64,
        source_time_scale: i32,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_writer_append_sample(
        writer: *mut c_void,
        input_id: i32,
        sample_buffer: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_writer_finish(
        writer: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_writer_add_video_input_pixel_buffer(
        writer: *mut c_void,
        width: i32,
        height: i32,
        pixel_format_type: u32,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_writer_append_pixel_buffer(
        writer: *mut c_void,
        input_id: i32,
        pixel_buffer: *mut c_void,
        pts_value: i64,
        pts_timescale: i32,
        out_error_message: *mut *mut AVWOwnedPayload,
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
    pub const START_FAILED: i32 = -7;
}
