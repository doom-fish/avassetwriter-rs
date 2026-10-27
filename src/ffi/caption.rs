use super::abi::{AVWArray, AVWCaption, AVWCaptionConversionSettings, AVWOwnedPayload, AVWString};
use core::ffi::c_void;

extern "C" {
    pub fn av_caption_region_predefined_payload(
        kind: *const AVWString,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut AVWOwnedPayload;

    pub fn av_caption_grouper_create() -> *mut c_void;

    pub fn av_caption_grouper_add_caption_payload(
        grouper: *mut c_void,
        caption_payload: *const AVWCaption,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_caption_grouper_flush_groups_payload(
        grouper: *mut c_void,
        up_to_time_value: i64,
        up_to_time_scale: i32,
        up_to_time_kind: i32,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut AVWOwnedPayload;

    pub fn av_caption_grouper_release(grouper: *mut c_void);

    pub fn av_caption_format_conformer_create(
        settings_payload: *const AVWCaptionConversionSettings,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut c_void;

    pub fn av_caption_format_conformer_conforms_captions_to_time_range(
        conformer: *mut c_void,
    ) -> bool;

    pub fn av_caption_format_conformer_set_conforms_captions_to_time_range(
        conformer: *mut c_void,
        conforms: bool,
    );

    pub fn av_caption_format_conformer_conformed_caption_payload(
        conformer: *mut c_void,
        caption_payload: *const AVWCaption,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut AVWOwnedPayload;

    pub fn av_caption_format_conformer_release(conformer: *mut c_void);

    pub fn av_caption_renderer_create() -> *mut c_void;

    pub fn av_caption_renderer_info_payload(renderer: *mut c_void) -> *mut AVWOwnedPayload;

    pub fn av_caption_renderer_set_captions_payload(
        renderer: *mut c_void,
        captions_payload: *const AVWArray,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_caption_renderer_set_bounds(
        renderer: *mut c_void,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    );

    pub fn av_caption_renderer_scene_changes_payload(
        renderer: *mut c_void,
        start_value: i64,
        start_scale: i32,
        start_kind: i32,
        duration_value: i64,
        duration_scale: i32,
        duration_kind: i32,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut AVWOwnedPayload;

    pub fn av_caption_renderer_release(renderer: *mut c_void);

    pub fn av_caption_conversion_validator_create(
        captions_payload: *const AVWArray,
        start_value: i64,
        start_scale: i32,
        start_kind: i32,
        duration_value: i64,
        duration_scale: i32,
        duration_kind: i32,
        settings_payload: *const AVWCaptionConversionSettings,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut c_void;

    pub fn av_caption_conversion_validator_info_payload(
        validator: *mut c_void,
    ) -> *mut AVWOwnedPayload;

    pub fn av_caption_conversion_validator_validate(
        validator: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_caption_conversion_validator_stop_validating(validator: *mut c_void);

    pub fn av_caption_conversion_validator_release(validator: *mut c_void);
}
