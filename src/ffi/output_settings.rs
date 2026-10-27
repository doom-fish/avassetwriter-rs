use super::abi::{AVWOwnedPayload, AVWString};
use core::ffi::c_void;

extern "C" {
    pub fn av_output_settings_assistant_available_presets_payload() -> *mut AVWOwnedPayload;

    pub fn av_output_settings_assistant_create(
        preset: *const AVWString,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut c_void;

    pub fn av_output_settings_assistant_release(assistant: *mut c_void);

    pub fn av_output_settings_assistant_info_payload(
        assistant: *mut c_void,
    ) -> *mut AVWOwnedPayload;

    pub fn av_output_settings_assistant_source_audio_format(assistant: *mut c_void) -> *mut c_void;

    pub fn av_output_settings_assistant_set_source_audio_format(
        assistant: *mut c_void,
        format: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_output_settings_assistant_source_video_format(assistant: *mut c_void) -> *mut c_void;

    pub fn av_output_settings_assistant_set_source_video_format(
        assistant: *mut c_void,
        format: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_output_settings_assistant_set_source_video_average_frame_duration(
        assistant: *mut c_void,
        value: i64,
        timescale: i32,
        kind: i32,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_output_settings_assistant_set_source_video_min_frame_duration(
        assistant: *mut c_void,
        value: i64,
        timescale: i32,
        kind: i32,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;
}
