use super::abi::{AVWArray, AVWOwnedPayload, AVWString};
use core::ffi::c_void;

extern "C" {
    pub fn av_export_session_all_presets_payload() -> *mut AVWOwnedPayload;

    pub fn av_export_session_compatible_presets_payload(
        path: *const AVWString,
    ) -> *mut AVWOwnedPayload;

    pub fn av_export_session_determine_compatibility(
        path: *const AVWString,
        preset: *const AVWString,
        output_file_type: *const AVWString,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_export_session_create(
        path: *const AVWString,
        preset: *const AVWString,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut c_void;

    pub fn av_export_session_release(session: *mut c_void);

    pub fn av_export_session_info_payload(session: *mut c_void) -> *mut AVWOwnedPayload;

    pub fn av_export_session_set_output_file_type(
        session: *mut c_void,
        output_file_type: *const AVWString,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_export_session_set_output_path(
        session: *mut c_void,
        output_path: *const AVWString,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_export_session_set_should_optimize_for_network_use(
        session: *mut c_void,
        enabled: bool,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_export_session_set_allows_parallelized_export(
        session: *mut c_void,
        enabled: bool,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_export_session_export(
        session: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_export_session_cancel(
        session: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_export_session_compatible_file_types_payload(
        session: *mut c_void,
    ) -> *mut AVWOwnedPayload;

    pub fn av_export_session_set_time_range(
        session: *mut c_void,
        start_value: i64,
        start_timescale: i32,
        start_kind: i32,
        duration_value: i64,
        duration_timescale: i32,
        duration_kind: i32,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_export_session_set_file_length_limit(
        session: *mut c_void,
        limit: i64,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_export_session_estimated_maximum_duration_payload(
        session: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut AVWOwnedPayload;

    pub fn av_export_session_estimated_output_file_length(
        session: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i64;

    pub fn av_export_session_set_metadata_payload(
        session: *mut c_void,
        metadata_payload: *const AVWArray,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_export_session_metadata(
        session: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut c_void;

    pub fn av_export_session_set_metadata_items(
        session: *mut c_void,
        items: *const *mut c_void,
        item_count: usize,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_export_session_metadata_item_filter(session: *mut c_void) -> *mut c_void;

    pub fn av_export_session_set_metadata_item_filter(
        session: *mut c_void,
        filter: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_export_session_audio_mix(session: *mut c_void) -> *mut c_void;

    pub fn av_export_session_set_audio_mix(
        session: *mut c_void,
        audio_mix: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_export_session_video_composition(session: *mut c_void) -> *mut c_void;

    pub fn av_export_session_set_video_composition(
        session: *mut c_void,
        video_composition: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_export_session_custom_video_compositor(session: *mut c_void) -> *mut c_void;

    pub fn av_export_session_set_can_perform_multiple_passes_over_source_media_data(
        session: *mut c_void,
        enabled: bool,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_export_session_set_directory_for_temporary_files(
        session: *mut c_void,
        path: *const AVWString,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_export_session_set_audio_track_group_handling(
        session: *mut c_void,
        handling: u64,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;
}
