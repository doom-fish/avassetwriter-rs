use core::ffi::{c_char, c_void};

extern "C" {
    pub fn av_composition_create_empty() -> *mut c_void;

    pub fn av_composition_create_from_asset(
        asset: *mut c_void,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_void;

    pub fn av_composition_release(composition: *mut c_void);

    pub fn av_composition_info_json(composition: *mut c_void) -> *mut c_char;

    pub fn av_composition_track_count(composition: *mut c_void) -> usize;

    pub fn av_composition_copy_track_at_index(
        composition: *mut c_void,
        index: usize,
    ) -> *mut c_void;

    pub fn av_composition_track_release(track: *mut c_void);

    pub fn av_composition_track_info_json(track: *mut c_void) -> *mut c_char;

    pub fn av_composition_track_segment_count(track: *mut c_void) -> usize;

    pub fn av_composition_track_copy_segment_at_index(
        track: *mut c_void,
        index: usize,
    ) -> *mut c_void;

    pub fn av_composition_track_segment_for_track_time(
        track: *mut c_void,
        time_value: i64,
        time_scale: i32,
        time_kind: i32,
    ) -> *mut c_void;

    pub fn av_composition_track_has_media_characteristic(
        track: *mut c_void,
        media_characteristic: *const c_char,
    ) -> bool;

    pub fn av_composition_track_sample_presentation_time_json(
        track: *mut c_void,
        time_value: i64,
        time_scale: i32,
        time_kind: i32,
    ) -> *mut c_char;

    pub fn av_composition_track_format_description_count(track: *mut c_void) -> usize;

    pub fn av_composition_track_copy_format_description_at_index(
        track: *mut c_void,
        index: usize,
    ) -> *mut c_void;

    pub fn av_composition_track_format_description_replacement_count(track: *mut c_void) -> usize;

    pub fn av_composition_track_copy_format_description_replacement_at_index(
        track: *mut c_void,
        index: usize,
    ) -> *mut c_void;

    pub fn av_composition_track_segment_create_url(
        url: *const c_char,
        is_file_url: bool,
        track_id: i32,
        source_start_value: i64,
        source_start_scale: i32,
        source_start_kind: i32,
        source_duration_value: i64,
        source_duration_scale: i32,
        source_duration_kind: i32,
        target_start_value: i64,
        target_start_scale: i32,
        target_start_kind: i32,
        target_duration_value: i64,
        target_duration_scale: i32,
        target_duration_kind: i32,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_void;

    pub fn av_composition_track_segment_create_empty(
        start_value: i64,
        start_scale: i32,
        start_kind: i32,
        duration_value: i64,
        duration_scale: i32,
        duration_kind: i32,
    ) -> *mut c_void;

    pub fn av_composition_track_segment_release(segment: *mut c_void);

    pub fn av_composition_track_segment_info_json(segment: *mut c_void) -> *mut c_char;

    pub fn av_composition_track_segment_asset_track_segment(segment: *mut c_void) -> *mut c_void;

    pub fn av_asset_track_segment_release(segment: *mut c_void);

    pub fn av_asset_track_segment_info_json(segment: *mut c_void) -> *mut c_char;

    pub fn av_composition_track_format_description_replacement_release(replacement: *mut c_void);

    pub fn av_composition_track_format_description_replacement_info_json(
        replacement: *mut c_void,
    ) -> *mut c_char;

    pub fn av_composition_track_format_description_replacement_original_format_description(
        replacement: *mut c_void,
    ) -> *mut c_void;

    pub fn av_composition_track_format_description_replacement_replacement_format_description(
        replacement: *mut c_void,
    ) -> *mut c_void;
}
