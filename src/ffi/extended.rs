use core::ffi::{c_char, c_void};

pub use doom_fish_utils::ffi_callbacks::DropCallback;

pub type ReadyCallback = unsafe extern "C" fn(userdata: *mut c_void);
pub type PassDescriptionCallback =
    unsafe extern "C" fn(payload_json: *const c_char, userdata: *mut c_void);
pub type SegmentCallback = unsafe extern "C" fn(
    bytes: *const u8,
    byte_len: usize,
    segment_type: i32,
    report_json: *const c_char,
    userdata: *mut c_void,
);
extern "C" {
    pub fn av_writer_create_segmented(
        file_type: *const c_char,
        profile: *const c_char,
        callback: Option<SegmentCallback>,
        userdata: *mut c_void,
        drop_userdata: Option<DropCallback>,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_void;

    pub fn av_writer_info_json(writer: *mut c_void) -> *mut c_char;

    pub fn av_writer_set_metadata_json(
        writer: *mut c_void,
        metadata_json: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_set_directory_for_temporary_files(
        writer: *mut c_void,
        directory_path: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_can_apply_output_settings_json(
        writer: *mut c_void,
        media_type: *const c_char,
        output_settings_json: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_add_input_json(
        writer: *mut c_void,
        media_type: *const c_char,
        output_settings_json: *const c_char,
        source_format_hint: *mut c_void,
        expects_media_data_in_real_time: bool,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_add_audio_input_from_sample(
        writer: *mut c_void,
        sample_buffer: *mut c_void,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_add_metadata_input_from_specifications_json(
        writer: *mut c_void,
        specifications_json: *const c_char,
        expects_media_data_in_real_time: bool,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_end_session(
        writer: *mut c_void,
        time_value: i64,
        time_scale: i32,
        time_kind: i32,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_cancel(writer: *mut c_void, out_error_message: *mut *mut c_char) -> i32;

    pub fn av_writer_attach_pixel_buffer_adaptor_json(
        writer: *mut c_void,
        input_id: i32,
        attributes_json: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_pixel_buffer_pool(writer: *mut c_void, input_id: i32) -> *mut c_void;

    pub fn av_writer_attach_tagged_pixel_buffer_group_adaptor_json(
        writer: *mut c_void,
        input_id: i32,
        attributes_json: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_tagged_pixel_buffer_pool(writer: *mut c_void, input_id: i32) -> *mut c_void;

    pub fn av_writer_append_tagged_pixel_buffer_group(
        writer: *mut c_void,
        input_id: i32,
        pixel_buffers: *const *mut c_void,
        layer_ids: *const i64,
        count: usize,
        pts_value: i64,
        pts_scale: i32,
        pts_kind: i32,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_attach_metadata_adaptor(
        writer: *mut c_void,
        input_id: i32,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_append_timed_metadata_group_json(
        writer: *mut c_void,
        input_id: i32,
        group_json: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_attach_caption_adaptor(
        writer: *mut c_void,
        input_id: i32,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_append_caption_json(
        writer: *mut c_void,
        input_id: i32,
        caption_json: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_append_caption_group_json(
        writer: *mut c_void,
        input_id: i32,
        caption_group_json: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_input_info_json(writer: *mut c_void, input_id: i32) -> *mut c_char;

    pub fn av_writer_input_source_format_hint(writer: *mut c_void, input_id: i32) -> *mut c_void;

    pub fn av_writer_set_input_metadata_json(
        writer: *mut c_void,
        input_id: i32,
        metadata_json: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_input_set_expects_media_data_in_real_time(
        writer: *mut c_void,
        input_id: i32,
        expects_media_data_in_real_time: bool,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_input_request_media_data_when_ready(
        writer: *mut c_void,
        input_id: i32,
        callback: Option<ReadyCallback>,
        userdata: *mut c_void,
        drop_userdata: Option<DropCallback>,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_input_mark_as_finished(
        writer: *mut c_void,
        input_id: i32,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_input_set_language_code(
        writer: *mut c_void,
        input_id: i32,
        language_code: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_input_set_extended_language_tag(
        writer: *mut c_void,
        input_id: i32,
        extended_language_tag: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_input_set_natural_size(
        writer: *mut c_void,
        input_id: i32,
        width: f64,
        height: f64,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_input_set_transform(
        writer: *mut c_void,
        input_id: i32,
        a: f64,
        b: f64,
        c: f64,
        d: f64,
        tx: f64,
        ty: f64,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_input_set_preferred_volume(
        writer: *mut c_void,
        input_id: i32,
        preferred_volume: f32,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_input_set_marks_output_track_as_enabled(
        writer: *mut c_void,
        input_id: i32,
        enabled: bool,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_input_set_media_time_scale(
        writer: *mut c_void,
        input_id: i32,
        media_time_scale: i32,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_input_set_preferred_media_chunk_duration(
        writer: *mut c_void,
        input_id: i32,
        duration_value: i64,
        duration_scale: i32,
        duration_kind: i32,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_input_set_preferred_media_chunk_alignment(
        writer: *mut c_void,
        input_id: i32,
        alignment: i64,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_input_set_sample_reference_base_url(
        writer: *mut c_void,
        input_id: i32,
        sample_reference_base_url: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_input_set_media_data_location(
        writer: *mut c_void,
        input_id: i32,
        location: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_input_can_add_track_association(
        writer: *mut c_void,
        input_id: i32,
        other_input_id: i32,
        association_type: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_input_add_track_association(
        writer: *mut c_void,
        input_id: i32,
        other_input_id: i32,
        association_type: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_input_set_performs_multi_pass_encoding_if_supported(
        writer: *mut c_void,
        input_id: i32,
        enabled: bool,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_input_respond_to_each_pass_description(
        writer: *mut c_void,
        input_id: i32,
        callback: Option<PassDescriptionCallback>,
        userdata: *mut c_void,
        drop_userdata: Option<DropCallback>,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_input_mark_current_pass_as_finished(
        writer: *mut c_void,
        input_id: i32,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_set_movie_fragment_interval(
        writer: *mut c_void,
        interval_value: i64,
        interval_scale: i32,
        interval_kind: i32,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_set_initial_movie_fragment_interval(
        writer: *mut c_void,
        interval_value: i64,
        interval_scale: i32,
        interval_kind: i32,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_set_initial_movie_fragment_sequence_number(
        writer: *mut c_void,
        sequence_number: i64,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_set_produces_combinable_fragments(
        writer: *mut c_void,
        enabled: bool,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_set_overall_duration_hint(
        writer: *mut c_void,
        hint_value: i64,
        hint_scale: i32,
        hint_kind: i32,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_set_movie_time_scale(
        writer: *mut c_void,
        movie_time_scale: i32,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_set_preferred_output_segment_interval(
        writer: *mut c_void,
        interval_value: i64,
        interval_scale: i32,
        interval_kind: i32,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_set_initial_segment_start_time(
        writer: *mut c_void,
        start_value: i64,
        start_scale: i32,
        start_kind: i32,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_set_output_file_type_profile(
        writer: *mut c_void,
        profile: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_writer_flush_segment(writer: *mut c_void, out_error_message: *mut *mut c_char)
        -> i32;
}
