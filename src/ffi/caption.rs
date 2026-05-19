use core::ffi::{c_char, c_void};

extern "C" {
    pub fn av_caption_region_predefined_json(
        kind: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_char;

    pub fn av_caption_grouper_create() -> *mut c_void;

    pub fn av_caption_grouper_add_caption_json(
        grouper: *mut c_void,
        caption_json: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_caption_grouper_flush_groups_json(
        grouper: *mut c_void,
        up_to_time_value: i64,
        up_to_time_scale: i32,
        up_to_time_kind: i32,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_char;

    pub fn av_caption_grouper_release(grouper: *mut c_void);

    pub fn av_caption_format_conformer_create(
        settings_json: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_void;

    pub fn av_caption_format_conformer_conforms_captions_to_time_range(
        conformer: *mut c_void,
    ) -> bool;

    pub fn av_caption_format_conformer_set_conforms_captions_to_time_range(
        conformer: *mut c_void,
        conforms: bool,
    );

    pub fn av_caption_format_conformer_conformed_caption_json(
        conformer: *mut c_void,
        caption_json: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_char;

    pub fn av_caption_format_conformer_release(conformer: *mut c_void);

    pub fn av_caption_renderer_create() -> *mut c_void;

    pub fn av_caption_renderer_info_json(renderer: *mut c_void) -> *mut c_char;

    pub fn av_caption_renderer_set_captions_json(
        renderer: *mut c_void,
        captions_json: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_caption_renderer_set_bounds(
        renderer: *mut c_void,
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    );

    pub fn av_caption_renderer_scene_changes_json(
        renderer: *mut c_void,
        start_value: i64,
        start_scale: i32,
        start_kind: i32,
        duration_value: i64,
        duration_scale: i32,
        duration_kind: i32,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_char;

    pub fn av_caption_renderer_release(renderer: *mut c_void);

    pub fn av_caption_conversion_validator_create(
        captions_json: *const c_char,
        start_value: i64,
        start_scale: i32,
        start_kind: i32,
        duration_value: i64,
        duration_scale: i32,
        duration_kind: i32,
        settings_json: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_void;

    pub fn av_caption_conversion_validator_info_json(validator: *mut c_void) -> *mut c_char;

    pub fn av_caption_conversion_validator_validate(
        validator: *mut c_void,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_caption_conversion_validator_stop_validating(validator: *mut c_void);

    pub fn av_caption_conversion_validator_release(validator: *mut c_void);
}
