use core::ffi::{c_char, c_void};

extern "C" {
    pub fn av_metadata_item_filter_for_sharing() -> *mut c_void;

    pub fn av_metadata_item_filter_kind(filter: *mut c_void) -> *mut c_char;

    pub fn av_metadata_item_filter_release(filter: *mut c_void);

    pub fn av_audio_mix_create() -> *mut c_void;

    pub fn av_audio_mix_info_json(mix: *mut c_void) -> *mut c_char;

    pub fn av_audio_mix_input_parameter_count(mix: *mut c_void) -> usize;

    pub fn av_audio_mix_copy_input_parameter_at_index(
        mix: *mut c_void,
        index: usize,
    ) -> *mut c_void;

    pub fn av_audio_mix_set_input_parameters(
        mix: *mut c_void,
        parameters: *const *mut c_void,
        count: usize,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_audio_mix_release(mix: *mut c_void);

    pub fn av_audio_mix_input_parameters_create() -> *mut c_void;

    pub fn av_audio_mix_input_parameters_info_json(parameters: *mut c_void) -> *mut c_char;

    pub fn av_audio_mix_input_parameters_volume_ramp_json(
        parameters: *mut c_void,
        time_value: i64,
        time_scale: i32,
        time_kind: i32,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_char;

    pub fn av_audio_mix_input_parameters_set_track_id(
        parameters: *mut c_void,
        track_id: i32,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_audio_mix_input_parameters_set_audio_time_pitch_algorithm(
        parameters: *mut c_void,
        algorithm: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_audio_mix_input_parameters_set_volume(
        parameters: *mut c_void,
        volume: f32,
        time_value: i64,
        time_scale: i32,
        time_kind: i32,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_audio_mix_input_parameters_set_volume_ramp(
        parameters: *mut c_void,
        start_volume: f32,
        end_volume: f32,
        start_value: i64,
        start_scale: i32,
        start_kind: i32,
        duration_value: i64,
        duration_scale: i32,
        duration_kind: i32,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_audio_mix_input_parameters_release(parameters: *mut c_void);

    pub fn av_video_composition_create_from_asset(path: *const c_char) -> *mut c_void;

    pub fn av_video_composition_create_from_asset_ci_filter_recorder(
        path: *const c_char,
    ) -> *mut c_void;

    pub fn av_video_composition_info_json(composition: *mut c_void) -> *mut c_char;

    pub fn av_video_composition_set_custom_video_compositor_class(
        composition: *mut c_void,
        class: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_video_composition_release(composition: *mut c_void);

    pub fn av_video_compositor_info_json(compositor: *mut c_void) -> *mut c_char;

    pub fn av_take_latest_video_composition_request_json() -> *mut c_char;

    pub fn av_take_latest_ci_image_filtering_request_json() -> *mut c_char;

    pub fn av_video_compositor_release(compositor: *mut c_void);
}
