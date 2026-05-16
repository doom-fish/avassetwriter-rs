use core::ffi::{c_char, c_void};

extern "C" {
    pub fn av_export_session_all_presets_json() -> *mut c_char;

    pub fn av_export_session_compatible_presets_json(path: *const c_char) -> *mut c_char;

    pub fn av_export_session_determine_compatibility(
        path: *const c_char,
        preset: *const c_char,
        output_file_type: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_export_session_create(
        path: *const c_char,
        preset: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_void;

    pub fn av_export_session_release(session: *mut c_void);

    pub fn av_export_session_info_json(session: *mut c_void) -> *mut c_char;

    pub fn av_export_session_set_output_file_type(
        session: *mut c_void,
        output_file_type: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_export_session_set_output_path(
        session: *mut c_void,
        output_path: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_export_session_set_should_optimize_for_network_use(
        session: *mut c_void,
        enabled: bool,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_export_session_set_allows_parallelized_export(
        session: *mut c_void,
        enabled: bool,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_export_session_export(
        session: *mut c_void,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_export_session_cancel(
        session: *mut c_void,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_export_session_compatible_file_types_json(session: *mut c_void) -> *mut c_char;

    pub fn av_export_session_set_time_range(
        session: *mut c_void,
        start_value: i64,
        start_timescale: i32,
        start_kind: i32,
        duration_value: i64,
        duration_timescale: i32,
        duration_kind: i32,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_export_session_set_file_length_limit(
        session: *mut c_void,
        limit: i64,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_export_session_estimated_maximum_duration_json(
        session: *mut c_void,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_char;

    pub fn av_export_session_estimated_output_file_length(
        session: *mut c_void,
        out_error_message: *mut *mut c_char,
    ) -> i64;

    pub fn av_export_session_set_metadata_json(
        session: *mut c_void,
        metadata_json: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_export_session_set_can_perform_multiple_passes_over_source_media_data(
        session: *mut c_void,
        enabled: bool,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_export_session_set_directory_for_temporary_files(
        session: *mut c_void,
        path: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_export_session_set_audio_track_group_handling(
        session: *mut c_void,
        handling: u64,
        out_error_message: *mut *mut c_char,
    ) -> i32;
}
