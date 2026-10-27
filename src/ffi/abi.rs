#![allow(missing_docs)]

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWOwnedPayload {
    pub value: *const core::ffi::c_void,
    pub owner: *mut core::ffi::c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWString {
    pub data: *const u8,
    pub length: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWArray {
    pub data: *const core::ffi::c_void,
    pub length: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWTime {
    pub kind: i32,
    pub value: i64,
    pub timescale: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWMetadataValue {
    pub kind: i32,
    pub string_value: AVWString,
    pub integer_value: i64,
    pub float_value: f64,
    pub boolean_value: u8,
    pub data_value: AVWArray,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWPropertyValue {
    pub kind: i32,
    pub boolean: u8,
    pub signed_value: i64,
    pub unsigned_value: u64,
    pub number: f64,
    pub string: AVWString,
    pub bytes: AVWArray,
    pub values: AVWArray,
    pub entries: AVWArray,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWPropertyEntry {
    pub key: AVWString,
    pub value: AVWPropertyValue,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWSize {
    pub width: f64,
    pub height: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWTransform {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub tx: f64,
    pub ty: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWTimeRange {
    pub start: AVWTime,
    pub duration: AVWTime,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWMetadataItem {
    pub identifier: AVWString,
    pub value: AVWMetadataValue,
    pub data_type: *const AVWString,
    pub extended_language_tag: *const AVWString,
    pub locale_identifier: *const AVWString,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWMetadataItemProperties {
    pub identifier: AVWString,
    pub data_type: *const AVWString,
    pub extended_language_tag: *const AVWString,
    pub locale_identifier: *const AVWString,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWMetadataGroup {
    pub items: AVWArray,
    pub classifying_label: *const AVWString,
    pub unique_id: *const AVWString,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWTimedMetadataGroup {
    pub items: AVWArray,
    pub time_range: AVWTimeRange,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWTimedMetadataGroupInfo {
    pub items: AVWArray,
    pub time_range: AVWTimeRange,
    pub classifying_label: *const AVWString,
    pub unique_id: *const AVWString,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWTimedMetadataGroupProperties {
    pub time_range: AVWTimeRange,
    pub classifying_label: *const AVWString,
    pub unique_id: *const AVWString,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWDateRangeMetadataGroup {
    pub items: AVWArray,
    pub start_date: AVWString,
    pub end_date: *const AVWString,
    pub classifying_label: *const AVWString,
    pub unique_id: *const AVWString,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWDateRangeMetadataGroupProperties {
    pub start_date: AVWString,
    pub end_date: *const AVWString,
    pub classifying_label: *const AVWString,
    pub unique_id: *const AVWString,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWMetadataSpecification {
    pub identifier: AVWString,
    pub data_type: AVWString,
    pub extended_language_tag: *const AVWString,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWKeyLoadStatus {
    pub key: AVWString,
    pub status: i32,
    pub error_message: *const AVWString,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWInputGroup {
    pub inputs: AVWArray,
    pub default_input: *const i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWPassDescription {
    pub source_time_ranges: AVWArray,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWSegmentReportSample {
    pub presentation_time_stamp: AVWTime,
    pub offset: i64,
    pub length: i64,
    pub is_sync_sample: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWSegmentTrackReport {
    pub track_id: i32,
    pub media_type: AVWString,
    pub earliest_presentation_time_stamp: AVWTime,
    pub duration: AVWTime,
    pub first_video_sample_information: *const AVWSegmentReportSample,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWSegmentReport {
    pub segment_type: i32,
    pub track_reports: AVWArray,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWWriterInfo {
    pub output_path: *const AVWString,
    pub output_file_type: *const AVWString,
    pub available_media_types: AVWArray,
    pub status: i32,
    pub error_message: *const AVWString,
    pub should_optimize_for_network_use: u8,
    pub directory_for_temporary_files: *const AVWString,
    pub inputs: AVWArray,
    pub input_groups: AVWArray,
    pub movie_fragment_interval: AVWTime,
    pub initial_movie_fragment_interval: AVWTime,
    pub initial_movie_fragment_sequence_number: i64,
    pub produces_combinable_fragments: u8,
    pub overall_duration_hint: AVWTime,
    pub movie_time_scale: i32,
    pub preferred_output_segment_interval: AVWTime,
    pub initial_segment_start_time: AVWTime,
    pub output_file_type_profile: *const AVWString,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWInputInfo {
    pub media_type: AVWString,
    pub output_settings: *const AVWPropertyValue,
    pub ready_for_more_media_data: u8,
    pub expects_media_data_in_real_time: u8,
    pub language_code: *const AVWString,
    pub extended_language_tag: *const AVWString,
    pub natural_size: AVWSize,
    pub transform: AVWTransform,
    pub preferred_volume: f32,
    pub marks_output_track_as_enabled: u8,
    pub media_time_scale: i32,
    pub preferred_media_chunk_duration: AVWTime,
    pub preferred_media_chunk_alignment: i64,
    pub sample_reference_base_url: *const AVWString,
    pub media_data_location: *const AVWString,
    pub performs_multi_pass_encoding_if_supported: u8,
    pub can_perform_multiple_passes: u8,
    pub current_pass_description: *const AVWPassDescription,
    pub pixel_buffer_source_attributes: *const AVWPropertyValue,
    pub tagged_pixel_buffer_source_attributes: *const AVWPropertyValue,
    pub has_metadata_adaptor: u8,
    pub has_caption_adaptor: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWCaptionDimension {
    pub value: f64,
    pub units: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWCaptionPoint {
    pub x: AVWCaptionDimension,
    pub y: AVWCaptionDimension,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWCaptionSize {
    pub width: AVWCaptionDimension,
    pub height: AVWCaptionDimension,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWCaptionRegion {
    pub identifier: *const AVWString,
    pub origin: AVWCaptionPoint,
    pub size: AVWCaptionSize,
    pub scroll: i32,
    pub display_alignment: i32,
    pub writing_mode: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWCaptionRuby {
    pub text: AVWString,
    pub position: i32,
    pub alignment: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWCaptionRubySpan {
    pub start: usize,
    pub length: usize,
    pub ruby: AVWCaptionRuby,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWCaption {
    pub text: AVWString,
    pub time_range: AVWTimeRange,
    pub region: *const AVWCaptionRegion,
    pub text_alignment: *const i32,
    pub animation: *const i32,
    pub ruby_spans: AVWArray,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWCaptionGroup {
    pub captions: AVWArray,
    pub time_range: AVWTimeRange,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWCaptionBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWCaptionRendererInfo {
    pub captions: AVWArray,
    pub bounds: AVWCaptionBounds,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWCaptionRendererScene {
    pub time_range: AVWTimeRange,
    pub has_active_captions: u8,
    pub needs_periodic_refresh: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWCaptionConversionSettings {
    pub media_subtype: AVWString,
    pub time_code_frame_duration: *const AVWTime,
    pub use_drop_frame_time_code: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWCaptionConversionAdjustment {
    pub adjustment_type: AVWString,
    pub start_time_offset: *const AVWTime,
    pub duration_offset: *const AVWTime,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWCaptionConversionWarning {
    pub warning_type: AVWString,
    pub range_of_captions_start: usize,
    pub range_of_captions_length: usize,
    pub adjustment: *const AVWCaptionConversionAdjustment,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWCaptionConversionValidatorInfo {
    pub status: i32,
    pub captions: AVWArray,
    pub time_range: AVWTimeRange,
    pub warnings: AVWArray,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWOutputSettingsAssistantInfo {
    pub audio_settings: *const AVWPropertyValue,
    pub video_settings: *const AVWPropertyValue,
    pub output_file_type: *const AVWString,
    pub source_video_average_frame_duration: AVWTime,
    pub source_video_min_frame_duration: AVWTime,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWExportSessionInfo {
    pub preset_name: AVWString,
    pub asset_path: *const AVWString,
    pub output_file_type: *const AVWString,
    pub output_path: *const AVWString,
    pub should_optimize_for_network_use: u8,
    pub allows_parallelized_export: u8,
    pub status: i32,
    pub error_message: *const AVWString,
    pub progress: f32,
    pub supported_file_types: AVWArray,
    pub time_range: AVWTimeRange,
    pub file_length_limit: i64,
    pub can_perform_multiple_passes_over_source_media_data: u8,
    pub directory_for_temporary_files: *const AVWString,
    pub audio_track_group_handling: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWCompositionInfo {
    pub duration: AVWTime,
    pub natural_size: AVWSize,
    pub is_playable: u8,
    pub is_exportable: u8,
    pub is_readable: u8,
    pub is_composable: u8,
    pub url_asset_initialization_options: *const AVWPropertyValue,
    pub track_ids: AVWArray,
    pub unused_track_id: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWCompositionTimeMapping {
    pub source: AVWTimeRange,
    pub target: AVWTimeRange,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWAssetTrackSegmentInfo {
    pub time_mapping: AVWCompositionTimeMapping,
    pub is_empty: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWCompositionTrackSegmentInfo {
    pub time_mapping: AVWCompositionTimeMapping,
    pub is_empty: u8,
    pub source_url: *const AVWString,
    pub source_track_id: *const i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWFormatDescriptionSummary {
    pub media_type_raw: u32,
    pub media_type: AVWString,
    pub media_subtype_raw: u32,
    pub media_subtype: AVWString,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWCompositionTrackFormatDescriptionReplacementInfo {
    pub original_format_description: AVWFormatDescriptionSummary,
    pub replacement_format_description: AVWFormatDescriptionSummary,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWCompositionTrackInfo {
    pub track_id: i32,
    pub media_type: AVWString,
    pub is_playable: u8,
    pub is_decodable: u8,
    pub is_enabled: u8,
    pub is_self_contained: u8,
    pub total_sample_data_length: i64,
    pub time_range: AVWTimeRange,
    pub natural_time_scale: i32,
    pub estimated_data_rate: f32,
    pub language_code: *const AVWString,
    pub extended_language_tag: *const AVWString,
    pub natural_size: AVWSize,
    pub preferred_transform: AVWTransform,
    pub preferred_volume: f32,
    pub has_audio_sample_dependencies: u8,
    pub nominal_frame_rate: f32,
    pub min_frame_duration: AVWTime,
    pub requires_frame_reordering: u8,
    pub segment_count: usize,
    pub format_description_count: usize,
    pub format_description_replacement_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWAudioMixInfo {
    pub input_parameter_count: usize,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWAudioMixInputParametersInfo {
    pub track_id: i32,
    pub audio_time_pitch_algorithm: *const AVWString,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWAudioVolumeRamp {
    pub start_volume: f32,
    pub end_volume: f32,
    pub time_range: AVWTimeRange,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWVideoCompositionInfo {
    pub frame_duration: AVWTime,
    pub render_size: AVWSize,
    pub render_scale: f32,
    pub instruction_count: usize,
    pub custom_video_compositor_class_name: *const AVWString,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWVideoCompositorInfo {
    pub class_name: AVWString,
    pub source_pixel_buffer_attributes: *const AVWPropertyValue,
    pub required_pixel_buffer_attributes_for_render_context: *const AVWPropertyValue,
    pub supports_wide_color_source_frames: u8,
    pub supports_hdr_source_frames: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWVideoCompositionRequest {
    pub wire_format_version: u32,
    pub generation: u64,
    pub composition_time: AVWTime,
    pub source_track_ids: AVWArray,
    pub source_sample_data_track_ids: AVWArray,
    pub render_size: AVWSize,
    pub render_scale: f32,
    pub video_composition_instruction_class_name: *const AVWString,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWCIImageFilteringRequest {
    pub wire_format_version: u32,
    pub generation: u64,
    pub composition_time: AVWTime,
    pub render_size: AVWSize,
    pub source_image_extent: AVWRect,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AVWAssetInfo {
    pub url: *const AVWString,
    pub duration: AVWTime,
}
