use super::abi::{AVWOwnedPayload, AVWString};
use core::ffi::c_void;

pub type VideoCompositionRequestCallback =
    unsafe extern "C" fn(request: *mut c_void, userdata: *mut c_void);
pub type VideoCompositionDropCallback = unsafe extern "C" fn(userdata: *mut c_void);

extern "C" {
    pub fn av_metadata_item_filter_for_sharing() -> *mut c_void;

    pub fn av_metadata_item_filter_kind(filter: *mut c_void) -> *mut AVWOwnedPayload;

    pub fn av_metadata_item_filter_release(filter: *mut c_void);

    pub fn av_audio_mix_create() -> *mut c_void;

    pub fn av_audio_mix_info_payload(mix: *mut c_void) -> *mut AVWOwnedPayload;

    pub fn av_audio_mix_input_parameter_count(mix: *mut c_void) -> usize;

    pub fn av_audio_mix_copy_input_parameter_at_index(
        mix: *mut c_void,
        index: usize,
    ) -> *mut c_void;

    pub fn av_audio_mix_set_input_parameters(
        mix: *mut c_void,
        parameters: *const *mut c_void,
        count: usize,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_audio_mix_release(mix: *mut c_void);

    pub fn av_audio_mix_input_parameters_create() -> *mut c_void;

    pub fn av_audio_mix_input_parameters_info_payload(
        parameters: *mut c_void,
    ) -> *mut AVWOwnedPayload;

    pub fn av_audio_mix_input_parameters_volume_ramp_payload(
        parameters: *mut c_void,
        time_value: i64,
        time_scale: i32,
        time_kind: i32,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut AVWOwnedPayload;

    pub fn av_audio_mix_input_parameters_set_track_id(
        parameters: *mut c_void,
        track_id: i32,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_audio_mix_input_parameters_set_audio_time_pitch_algorithm(
        parameters: *mut c_void,
        algorithm: *const AVWString,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_audio_mix_input_parameters_set_volume(
        parameters: *mut c_void,
        volume: f32,
        time_value: i64,
        time_scale: i32,
        time_kind: i32,
        out_error_message: *mut *mut AVWOwnedPayload,
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
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_audio_mix_input_parameters_release(parameters: *mut c_void);

    pub fn av_video_composition_create_from_asset(path: *const AVWString) -> *mut c_void;

    pub fn av_video_composition_create_from_asset_ci_filter_recorder(
        path: *const AVWString,
    ) -> *mut c_void;

    pub fn av_video_composition_info_payload(composition: *mut c_void) -> *mut AVWOwnedPayload;

    pub fn av_video_composition_set_custom_video_compositor_class(
        composition: *mut c_void,
        class: *const AVWString,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_video_composition_install_rust_compositor(
        composition: *mut c_void,
        callback: Option<VideoCompositionRequestCallback>,
        userdata: *mut c_void,
        drop_userdata: Option<VideoCompositionDropCallback>,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_video_composition_clear_rust_compositor(
        composition: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_video_composition_release(composition: *mut c_void);

    pub fn av_video_compositor_info_payload(compositor: *mut c_void) -> *mut AVWOwnedPayload;

    pub fn av_copy_latest_video_composition_request_payload(
        out_generation: *mut u64,
    ) -> *mut AVWOwnedPayload;

    pub fn av_ack_latest_video_composition_request(generation: u64) -> bool;

    pub fn av_take_latest_video_composition_request_payload() -> *mut AVWOwnedPayload;

    pub fn av_copy_latest_ci_image_filtering_request_payload(
        out_generation: *mut u64,
    ) -> *mut AVWOwnedPayload;

    pub fn av_ack_latest_ci_image_filtering_request(generation: u64) -> bool;

    pub fn av_take_latest_ci_image_filtering_request_payload() -> *mut AVWOwnedPayload;

    pub fn av_video_composition_request_release(request: *mut c_void);

    pub fn av_video_composition_request_composition_time(
        request: *mut c_void,
        out_value: *mut i64,
        out_scale: *mut i32,
        out_kind: *mut i32,
    );

    pub fn av_video_composition_request_render_info(
        request: *mut c_void,
        out_width: *mut f64,
        out_height: *mut f64,
        out_scale: *mut f32,
    );

    pub fn av_video_composition_request_source_track_id_count(request: *mut c_void) -> usize;

    pub fn av_video_composition_request_source_track_id_at(
        request: *mut c_void,
        index: usize,
        out_track_id: *mut i32,
    ) -> bool;

    pub fn av_video_composition_request_source_sample_data_track_id_count(
        request: *mut c_void,
    ) -> usize;

    pub fn av_video_composition_request_source_sample_data_track_id_at(
        request: *mut c_void,
        index: usize,
        out_track_id: *mut i32,
    ) -> bool;

    pub fn av_video_composition_request_copy_source_frame(
        request: *mut c_void,
        track_id: i32,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut c_void;

    pub fn av_video_composition_request_copy_source_sample_buffer(
        request: *mut c_void,
        track_id: i32,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut c_void;

    pub fn av_video_composition_request_new_output_pixel_buffer(
        request: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut c_void;

    pub fn av_video_composition_request_finish_with_frame(
        request: *mut c_void,
        frame: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_video_composition_request_finish_with_error(
        request: *mut c_void,
        message: *const AVWString,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_video_composition_request_cancel(
        request: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_video_compositor_release(compositor: *mut c_void);
}
