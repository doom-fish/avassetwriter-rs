#ifndef AVASSETWRITER_ABI_H
#define AVASSETWRITER_ABI_H

#include <stddef.h>
#include <stdint.h>

typedef struct AVWOwnedPayload AVWOwnedPayload;
typedef struct AVWString AVWString;
typedef struct AVWArray AVWArray;
typedef struct AVWTime AVWTime;
typedef struct AVWMetadataValue AVWMetadataValue;
typedef struct AVWPropertyValue AVWPropertyValue;
typedef struct AVWPropertyEntry AVWPropertyEntry;
typedef struct AVWSize AVWSize;
typedef struct AVWTransform AVWTransform;
typedef struct AVWTimeRange AVWTimeRange;
typedef struct AVWMetadataItem AVWMetadataItem;
typedef struct AVWMetadataItemProperties AVWMetadataItemProperties;
typedef struct AVWMetadataGroup AVWMetadataGroup;
typedef struct AVWTimedMetadataGroup AVWTimedMetadataGroup;
typedef struct AVWTimedMetadataGroupInfo AVWTimedMetadataGroupInfo;
typedef struct AVWTimedMetadataGroupProperties AVWTimedMetadataGroupProperties;
typedef struct AVWDateRangeMetadataGroup AVWDateRangeMetadataGroup;
typedef struct AVWDateRangeMetadataGroupProperties AVWDateRangeMetadataGroupProperties;
typedef struct AVWMetadataSpecification AVWMetadataSpecification;
typedef struct AVWKeyLoadStatus AVWKeyLoadStatus;
typedef struct AVWInputGroup AVWInputGroup;
typedef struct AVWPassDescription AVWPassDescription;
typedef struct AVWSegmentReportSample AVWSegmentReportSample;
typedef struct AVWSegmentTrackReport AVWSegmentTrackReport;
typedef struct AVWSegmentReport AVWSegmentReport;
typedef struct AVWWriterInfo AVWWriterInfo;
typedef struct AVWInputInfo AVWInputInfo;
typedef struct AVWCaptionDimension AVWCaptionDimension;
typedef struct AVWCaptionPoint AVWCaptionPoint;
typedef struct AVWCaptionSize AVWCaptionSize;
typedef struct AVWCaptionRegion AVWCaptionRegion;
typedef struct AVWCaptionRuby AVWCaptionRuby;
typedef struct AVWCaptionRubySpan AVWCaptionRubySpan;
typedef struct AVWCaption AVWCaption;
typedef struct AVWCaptionGroup AVWCaptionGroup;
typedef struct AVWCaptionBounds AVWCaptionBounds;
typedef struct AVWCaptionRendererInfo AVWCaptionRendererInfo;
typedef struct AVWCaptionRendererScene AVWCaptionRendererScene;
typedef struct AVWCaptionConversionSettings AVWCaptionConversionSettings;
typedef struct AVWCaptionConversionAdjustment AVWCaptionConversionAdjustment;
typedef struct AVWCaptionConversionWarning AVWCaptionConversionWarning;
typedef struct AVWCaptionConversionValidatorInfo AVWCaptionConversionValidatorInfo;
typedef struct AVWOutputSettingsAssistantInfo AVWOutputSettingsAssistantInfo;
typedef struct AVWExportSessionInfo AVWExportSessionInfo;
typedef struct AVWCompositionInfo AVWCompositionInfo;
typedef struct AVWCompositionTimeMapping AVWCompositionTimeMapping;
typedef struct AVWAssetTrackSegmentInfo AVWAssetTrackSegmentInfo;
typedef struct AVWCompositionTrackSegmentInfo AVWCompositionTrackSegmentInfo;
typedef struct AVWFormatDescriptionSummary AVWFormatDescriptionSummary;
typedef struct AVWCompositionTrackFormatDescriptionReplacementInfo AVWCompositionTrackFormatDescriptionReplacementInfo;
typedef struct AVWCompositionTrackInfo AVWCompositionTrackInfo;
typedef struct AVWAudioMixInfo AVWAudioMixInfo;
typedef struct AVWAudioMixInputParametersInfo AVWAudioMixInputParametersInfo;
typedef struct AVWAudioVolumeRamp AVWAudioVolumeRamp;
typedef struct AVWVideoCompositionInfo AVWVideoCompositionInfo;
typedef struct AVWVideoCompositorInfo AVWVideoCompositorInfo;
typedef struct AVWRect AVWRect;
typedef struct AVWVideoCompositionRequest AVWVideoCompositionRequest;
typedef struct AVWCIImageFilteringRequest AVWCIImageFilteringRequest;
typedef struct AVWAssetInfo AVWAssetInfo;

struct AVWOwnedPayload {
    const void * value;
    void * owner;
};

struct AVWString {
    const uint8_t * data;
    size_t length;
};

struct AVWArray {
    const void * data;
    size_t length;
};

struct AVWTime {
    int32_t kind;
    int64_t value;
    int32_t timescale;
};

struct AVWMetadataValue {
    int32_t kind;
    AVWString string_value;
    int64_t integer_value;
    double float_value;
    uint8_t boolean_value;
    AVWArray data_value;
};

struct AVWPropertyValue {
    int32_t kind;
    uint8_t boolean;
    int64_t signed_value;
    uint64_t unsigned_value;
    double number;
    AVWString string;
    AVWArray bytes;
    AVWArray values;
    AVWArray entries;
};

struct AVWPropertyEntry {
    AVWString key;
    AVWPropertyValue value;
};

struct AVWSize {
    double width;
    double height;
};

struct AVWTransform {
    double a;
    double b;
    double c;
    double d;
    double tx;
    double ty;
};

struct AVWTimeRange {
    AVWTime start;
    AVWTime duration;
};

struct AVWMetadataItem {
    AVWString identifier;
    AVWMetadataValue value;
    const AVWString * data_type;
    const AVWString * extended_language_tag;
    const AVWString * locale_identifier;
};

struct AVWMetadataItemProperties {
    AVWString identifier;
    const AVWString * data_type;
    const AVWString * extended_language_tag;
    const AVWString * locale_identifier;
};

struct AVWMetadataGroup {
    AVWArray items;
    const AVWString * classifying_label;
    const AVWString * unique_id;
};

struct AVWTimedMetadataGroup {
    AVWArray items;
    AVWTimeRange time_range;
};

struct AVWTimedMetadataGroupInfo {
    AVWArray items;
    AVWTimeRange time_range;
    const AVWString * classifying_label;
    const AVWString * unique_id;
};

struct AVWTimedMetadataGroupProperties {
    AVWTimeRange time_range;
    const AVWString * classifying_label;
    const AVWString * unique_id;
};

struct AVWDateRangeMetadataGroup {
    AVWArray items;
    AVWString start_date;
    const AVWString * end_date;
    const AVWString * classifying_label;
    const AVWString * unique_id;
};

struct AVWDateRangeMetadataGroupProperties {
    AVWString start_date;
    const AVWString * end_date;
    const AVWString * classifying_label;
    const AVWString * unique_id;
};

struct AVWMetadataSpecification {
    AVWString identifier;
    AVWString data_type;
    const AVWString * extended_language_tag;
};

struct AVWKeyLoadStatus {
    AVWString key;
    int32_t status;
    const AVWString * error_message;
};

struct AVWInputGroup {
    AVWArray inputs;
    const int32_t * default_input;
};

struct AVWPassDescription {
    AVWArray source_time_ranges;
};

struct AVWSegmentReportSample {
    AVWTime presentation_time_stamp;
    int64_t offset;
    int64_t length;
    uint8_t is_sync_sample;
};

struct AVWSegmentTrackReport {
    int32_t track_id;
    AVWString media_type;
    AVWTime earliest_presentation_time_stamp;
    AVWTime duration;
    const AVWSegmentReportSample * first_video_sample_information;
};

struct AVWSegmentReport {
    int32_t segment_type;
    AVWArray track_reports;
};

struct AVWWriterInfo {
    const AVWString * output_path;
    const AVWString * output_file_type;
    AVWArray available_media_types;
    int32_t status;
    const AVWString * error_message;
    uint8_t should_optimize_for_network_use;
    const AVWString * directory_for_temporary_files;
    AVWArray inputs;
    AVWArray input_groups;
    AVWTime movie_fragment_interval;
    AVWTime initial_movie_fragment_interval;
    int64_t initial_movie_fragment_sequence_number;
    uint8_t produces_combinable_fragments;
    AVWTime overall_duration_hint;
    int32_t movie_time_scale;
    AVWTime preferred_output_segment_interval;
    AVWTime initial_segment_start_time;
    const AVWString * output_file_type_profile;
};

struct AVWInputInfo {
    AVWString media_type;
    const AVWPropertyValue * output_settings;
    uint8_t ready_for_more_media_data;
    uint8_t expects_media_data_in_real_time;
    const AVWString * language_code;
    const AVWString * extended_language_tag;
    AVWSize natural_size;
    AVWTransform transform;
    float preferred_volume;
    uint8_t marks_output_track_as_enabled;
    int32_t media_time_scale;
    AVWTime preferred_media_chunk_duration;
    int64_t preferred_media_chunk_alignment;
    const AVWString * sample_reference_base_url;
    const AVWString * media_data_location;
    uint8_t performs_multi_pass_encoding_if_supported;
    uint8_t can_perform_multiple_passes;
    const AVWPassDescription * current_pass_description;
    const AVWPropertyValue * pixel_buffer_source_attributes;
    const AVWPropertyValue * tagged_pixel_buffer_source_attributes;
    uint8_t has_metadata_adaptor;
    uint8_t has_caption_adaptor;
};

struct AVWCaptionDimension {
    double value;
    int32_t units;
};

struct AVWCaptionPoint {
    AVWCaptionDimension x;
    AVWCaptionDimension y;
};

struct AVWCaptionSize {
    AVWCaptionDimension width;
    AVWCaptionDimension height;
};

struct AVWCaptionRegion {
    const AVWString * identifier;
    AVWCaptionPoint origin;
    AVWCaptionSize size;
    int32_t scroll;
    int32_t display_alignment;
    int32_t writing_mode;
};

struct AVWCaptionRuby {
    AVWString text;
    int32_t position;
    int32_t alignment;
};

struct AVWCaptionRubySpan {
    size_t start;
    size_t length;
    AVWCaptionRuby ruby;
};

struct AVWCaption {
    AVWString text;
    AVWTimeRange time_range;
    const AVWCaptionRegion * region;
    const int32_t * text_alignment;
    const int32_t * animation;
    AVWArray ruby_spans;
};

struct AVWCaptionGroup {
    AVWArray captions;
    AVWTimeRange time_range;
};

struct AVWCaptionBounds {
    double x;
    double y;
    double width;
    double height;
};

struct AVWCaptionRendererInfo {
    AVWArray captions;
    AVWCaptionBounds bounds;
};

struct AVWCaptionRendererScene {
    AVWTimeRange time_range;
    uint8_t has_active_captions;
    uint8_t needs_periodic_refresh;
};

struct AVWCaptionConversionSettings {
    AVWString media_subtype;
    const AVWTime * time_code_frame_duration;
    uint8_t use_drop_frame_time_code;
};

struct AVWCaptionConversionAdjustment {
    AVWString adjustment_type;
    const AVWTime * start_time_offset;
    const AVWTime * duration_offset;
};

struct AVWCaptionConversionWarning {
    AVWString warning_type;
    size_t range_of_captions_start;
    size_t range_of_captions_length;
    const AVWCaptionConversionAdjustment * adjustment;
};

struct AVWCaptionConversionValidatorInfo {
    int32_t status;
    AVWArray captions;
    AVWTimeRange time_range;
    AVWArray warnings;
};

struct AVWOutputSettingsAssistantInfo {
    const AVWPropertyValue * audio_settings;
    const AVWPropertyValue * video_settings;
    const AVWString * output_file_type;
    AVWTime source_video_average_frame_duration;
    AVWTime source_video_min_frame_duration;
};

struct AVWExportSessionInfo {
    AVWString preset_name;
    const AVWString * asset_path;
    const AVWString * output_file_type;
    const AVWString * output_path;
    uint8_t should_optimize_for_network_use;
    uint8_t allows_parallelized_export;
    int32_t status;
    const AVWString * error_message;
    float progress;
    AVWArray supported_file_types;
    AVWTimeRange time_range;
    int64_t file_length_limit;
    uint8_t can_perform_multiple_passes_over_source_media_data;
    const AVWString * directory_for_temporary_files;
    uint64_t audio_track_group_handling;
};

struct AVWCompositionInfo {
    AVWTime duration;
    AVWSize natural_size;
    uint8_t is_playable;
    uint8_t is_exportable;
    uint8_t is_readable;
    uint8_t is_composable;
    const AVWPropertyValue * url_asset_initialization_options;
    AVWArray track_ids;
    int32_t unused_track_id;
};

struct AVWCompositionTimeMapping {
    AVWTimeRange source;
    AVWTimeRange target;
};

struct AVWAssetTrackSegmentInfo {
    AVWCompositionTimeMapping time_mapping;
    uint8_t is_empty;
};

struct AVWCompositionTrackSegmentInfo {
    AVWCompositionTimeMapping time_mapping;
    uint8_t is_empty;
    const AVWString * source_url;
    const int32_t * source_track_id;
};

struct AVWFormatDescriptionSummary {
    uint32_t media_type_raw;
    AVWString media_type;
    uint32_t media_subtype_raw;
    AVWString media_subtype;
};

struct AVWCompositionTrackFormatDescriptionReplacementInfo {
    AVWFormatDescriptionSummary original_format_description;
    AVWFormatDescriptionSummary replacement_format_description;
};

struct AVWCompositionTrackInfo {
    int32_t track_id;
    AVWString media_type;
    uint8_t is_playable;
    uint8_t is_decodable;
    uint8_t is_enabled;
    uint8_t is_self_contained;
    int64_t total_sample_data_length;
    AVWTimeRange time_range;
    int32_t natural_time_scale;
    float estimated_data_rate;
    const AVWString * language_code;
    const AVWString * extended_language_tag;
    AVWSize natural_size;
    AVWTransform preferred_transform;
    float preferred_volume;
    uint8_t has_audio_sample_dependencies;
    float nominal_frame_rate;
    AVWTime min_frame_duration;
    uint8_t requires_frame_reordering;
    size_t segment_count;
    size_t format_description_count;
    size_t format_description_replacement_count;
};

struct AVWAudioMixInfo {
    size_t input_parameter_count;
};

struct AVWAudioMixInputParametersInfo {
    int32_t track_id;
    const AVWString * audio_time_pitch_algorithm;
};

struct AVWAudioVolumeRamp {
    float start_volume;
    float end_volume;
    AVWTimeRange time_range;
};

struct AVWVideoCompositionInfo {
    AVWTime frame_duration;
    AVWSize render_size;
    float render_scale;
    size_t instruction_count;
    const AVWString * custom_video_compositor_class_name;
};

struct AVWVideoCompositorInfo {
    AVWString class_name;
    const AVWPropertyValue * source_pixel_buffer_attributes;
    const AVWPropertyValue * required_pixel_buffer_attributes_for_render_context;
    uint8_t supports_wide_color_source_frames;
    uint8_t supports_hdr_source_frames;
};

struct AVWRect {
    double x;
    double y;
    double width;
    double height;
};

struct AVWVideoCompositionRequest {
    uint32_t wire_format_version;
    uint64_t generation;
    AVWTime composition_time;
    AVWArray source_track_ids;
    AVWArray source_sample_data_track_ids;
    AVWSize render_size;
    float render_scale;
    const AVWString * video_composition_instruction_class_name;
};

struct AVWCIImageFilteringRequest {
    uint32_t wire_format_version;
    uint64_t generation;
    AVWTime composition_time;
    AVWSize render_size;
    AVWRect source_image_extent;
};

struct AVWAssetInfo {
    const AVWString * url;
    AVWTime duration;
};

#endif
