#![allow(
    clippy::doc_markdown,
    clippy::missing_errors_doc,
    clippy::needless_pass_by_value,
    clippy::struct_excessive_bools,
    clippy::too_many_lines
)]

use core::ffi::{c_char, c_void};
use core::ptr;
use std::ffi::CString;
use std::path::Path;

use apple_cf::cm::{CMFormatDescription, CMSampleBuffer};
use apple_cf::cv::{CVPixelBuffer, CVPixelBufferPool};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use super::{FileType, InputId, Writer};
use crate::callbacks::{
    pass_description_callback_drop, pass_description_callback_trampoline, ready_callback_drop,
    ready_callback_trampoline, segment_callback_drop, segment_callback_trampoline,
    PassDescriptionCallbackState, ReadyCallbackState, SegmentCallbackState,
};
use crate::caption::{Caption, CaptionGroup};
use crate::error::{from_swift, AVWriterError};
use crate::ffi;
use crate::metadata::{MetadataItem, MetadataSpecification, TimedMetadataGroup};
use crate::time::{Time, TimeRange};

/// `AVFileTypeProfile`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FileTypeProfile {
    AppleHls,
    CmafCompliant,
    Other(String),
}

impl FileTypeProfile {
    pub(crate) fn as_str(&self) -> &str {
        match self {
            Self::AppleHls => "apple_hls",
            Self::CmafCompliant => "cmaf_compliant",
            Self::Other(value) => value.as_str(),
        }
    }

    pub(crate) fn from_raw(raw: &str) -> Self {
        match raw {
            "apple_hls" => Self::AppleHls,
            "cmaf_compliant" => Self::CmafCompliant,
            other => Self::Other(other.to_string()),
        }
    }
}

/// `AVMediaType`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum MediaType {
    Video,
    Audio,
    Text,
    ClosedCaption,
    Subtitle,
    Timecode,
    Metadata,
    Muxed,
    Haptic,
    DepthData,
    AuxiliaryPicture,
    Other(String),
}

impl MediaType {
    pub(crate) fn as_str(&self) -> &str {
        match self {
            Self::Video => "video",
            Self::Audio => "audio",
            Self::Text => "text",
            Self::ClosedCaption => "closed_caption",
            Self::Subtitle => "subtitle",
            Self::Timecode => "timecode",
            Self::Metadata => "metadata",
            Self::Muxed => "muxed",
            Self::Haptic => "haptic",
            Self::DepthData => "depth_data",
            Self::AuxiliaryPicture => "auxiliary_picture",
            Self::Other(value) => value.as_str(),
        }
    }

    pub(crate) fn from_raw(raw: &str) -> Self {
        match raw {
            "video" => Self::Video,
            "audio" => Self::Audio,
            "text" => Self::Text,
            "closed_caption" => Self::ClosedCaption,
            "subtitle" => Self::Subtitle,
            "timecode" => Self::Timecode,
            "metadata" => Self::Metadata,
            "muxed" => Self::Muxed,
            "haptic" => Self::Haptic,
            "depth_data" => Self::DepthData,
            "auxiliary_picture" => Self::AuxiliaryPicture,
            other => Self::Other(other.to_string()),
        }
    }
}

/// `AVAssetWriterStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum WriterStatus {
    Unknown,
    Writing,
    Completed,
    Failed,
    Cancelled,
    Other(i32),
}

impl WriterStatus {
    #[must_use]
    pub const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Unknown,
            1 => Self::Writing,
            2 => Self::Completed,
            3 => Self::Failed,
            4 => Self::Cancelled,
            other => Self::Other(other),
        }
    }
}

/// `AVAssetWriterInputMediaDataLocation`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum InputMediaDataLocation {
    InterleavedWithMainMediaData,
    BeforeMainMediaDataNotInterleaved,
    SparselyInterleavedWithMainMediaData,
    Other(String),
}

impl InputMediaDataLocation {
    fn as_str(&self) -> &str {
        match self {
            Self::InterleavedWithMainMediaData => "interleaved",
            Self::BeforeMainMediaDataNotInterleaved => "before_main_not_interleaved",
            Self::SparselyInterleavedWithMainMediaData => "sparse_interleaved",
            Self::Other(value) => value.as_str(),
        }
    }

    fn from_raw(raw: &str) -> Self {
        match raw {
            "interleaved" => Self::InterleavedWithMainMediaData,
            "before_main_not_interleaved" => Self::BeforeMainMediaDataNotInterleaved,
            "sparse_interleaved" => Self::SparselyInterleavedWithMainMediaData,
            other => Self::Other(other.to_string()),
        }
    }
}

/// `AVTrackAssociationType`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TrackAssociationType {
    AudioFallback,
    ChapterList,
    ForcedSubtitlesOnly,
    SelectionFollower,
    Timecode,
    MetadataReferent,
    RenderMetadataSource,
    Other(String),
}

impl TrackAssociationType {
    fn as_str(&self) -> &str {
        match self {
            Self::AudioFallback => "audio_fallback",
            Self::ChapterList => "chapter_list",
            Self::ForcedSubtitlesOnly => "forced_subtitles_only",
            Self::SelectionFollower => "selection_follower",
            Self::Timecode => "timecode",
            Self::MetadataReferent => "metadata_referent",
            Self::RenderMetadataSource => "render_metadata_source",
            Self::Other(value) => value.as_str(),
        }
    }
}

/// `AVAssetWriterInputPassDescription`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputPassDescription {
    pub source_time_ranges: Vec<TimeRange>,
}

/// `AVAssetSegmentType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SegmentType {
    Initialization,
    Separable,
    Other(i32),
}

impl SegmentType {
    #[must_use]
    pub const fn from_raw(raw: i32) -> Self {
        match raw {
            1 => Self::Initialization,
            2 => Self::Separable,
            other => Self::Other(other),
        }
    }
}

/// `AVAssetSegmentReportSampleInformation`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentReportSampleInfo {
    pub presentation_time_stamp: Time,
    pub offset: i64,
    pub length: i64,
    pub is_sync_sample: bool,
}

/// `AVAssetSegmentTrackReport`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentTrackReport {
    pub track_id: i32,
    pub media_type: MediaType,
    pub earliest_presentation_time_stamp: Time,
    pub duration: Time,
    pub first_video_sample_information: Option<SegmentReportSampleInfo>,
}

/// `AVAssetSegmentReport`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentReport {
    pub segment_type: SegmentType,
    pub track_reports: Vec<SegmentTrackReport>,
}

/// A segment callback payload for segmented output writers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentOutput {
    pub data: Vec<u8>,
    pub segment_type: SegmentType,
    pub report: Option<SegmentReport>,
}

/// Readback for `AVAssetWriterInputGroup`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputGroupInfo {
    pub inputs: Vec<InputId>,
    pub default_input: Option<InputId>,
}

/// One pixel buffer inside a tagged pixel-buffer group append.
#[derive(Debug, Clone, Copy)]
pub struct TaggedPixelBuffer<'a> {
    pub pixel_buffer: &'a CVPixelBuffer,
    pub layer_id: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InputGroupPayload {
    inputs: Vec<i32>,
    default_input: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WriterInfoPayload {
    output_path: Option<String>,
    output_file_type: Option<String>,
    available_media_types: Vec<String>,
    status: i32,
    error_message: Option<String>,
    metadata: Vec<MetadataItem>,
    should_optimize_for_network_use: bool,
    directory_for_temporary_files: Option<String>,
    inputs: Vec<i32>,
    input_groups: Vec<InputGroupPayload>,
    movie_fragment_interval: Time,
    initial_movie_fragment_interval: Time,
    initial_movie_fragment_sequence_number: i64,
    produces_combinable_fragments: bool,
    overall_duration_hint: Time,
    movie_time_scale: i32,
    preferred_output_segment_interval: Time,
    initial_segment_start_time: Time,
    output_file_type_profile: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SizePayload {
    width: f64,
    height: f64,
}

#[derive(Debug, Deserialize)]
struct TransformPayload {
    a: f64,
    b: f64,
    c: f64,
    d: f64,
    tx: f64,
    ty: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InputInfoPayload {
    media_type: String,
    output_settings_json: Option<String>,
    metadata: Vec<MetadataItem>,
    ready_for_more_media_data: bool,
    expects_media_data_in_real_time: bool,
    language_code: Option<String>,
    extended_language_tag: Option<String>,
    natural_size: SizePayload,
    transform: TransformPayload,
    preferred_volume: f32,
    marks_output_track_as_enabled: bool,
    media_time_scale: i32,
    preferred_media_chunk_duration: Time,
    preferred_media_chunk_alignment: i64,
    sample_reference_base_url: Option<String>,
    media_data_location: Option<String>,
    performs_multi_pass_encoding_if_supported: bool,
    can_perform_multiple_passes: bool,
    current_pass_description: Option<InputPassDescription>,
    pixel_buffer_source_attributes_json: Option<String>,
    tagged_pixel_buffer_source_attributes_json: Option<String>,
    has_metadata_adaptor: bool,
    has_caption_adaptor: bool,
}

impl InputId {
    #[must_use]
    pub const fn raw(self) -> i32 {
        self.0
    }
}

impl Writer {
    pub fn create_segmented(
        file_type: FileType,
        profile: Option<FileTypeProfile>,
        on_segment: impl FnMut(SegmentOutput) + Send + 'static,
    ) -> Result<Self, AVWriterError> {
        let file_type_c = cstring_arg(file_type.as_str(), "file type")?;
        let profile_string = profile
            .as_ref()
            .map(|value| cstring_arg(value.as_str(), "file type profile"))
            .transpose()?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let state = Box::new(SegmentCallbackState {
            callback: Box::new(on_segment),
        });
        let userdata = Box::into_raw(state).cast::<c_void>();
        let ptr = unsafe {
            ffi::av_writer_create_segmented(
                file_type_c.as_ptr(),
                profile_string
                    .as_ref()
                    .map_or(ptr::null(), |value| value.as_ptr()),
                Some(segment_callback_trampoline),
                userdata,
                Some(segment_callback_drop),
                &mut err_msg,
            )
        };
        if ptr.is_null() {
            unsafe { segment_callback_drop(userdata) };
            return Err(unsafe { from_swift(ffi::status::WRITER_CREATE_FAILED, err_msg) });
        }
        Ok(Self { ptr })
    }

    pub fn status(&self) -> Result<WriterStatus, AVWriterError> {
        Ok(WriterStatus::from_raw(self.writer_info()?.status))
    }

    pub fn error_message(&self) -> Result<Option<String>, AVWriterError> {
        Ok(self.writer_info()?.error_message)
    }

    pub fn output_path(&self) -> Result<Option<String>, AVWriterError> {
        Ok(self.writer_info()?.output_path)
    }

    pub fn output_file_type(&self) -> Result<Option<FileType>, AVWriterError> {
        Ok(self
            .writer_info()?
            .output_file_type
            .as_deref()
            .and_then(FileType::from_raw))
    }

    pub fn available_media_types(&self) -> Result<Vec<MediaType>, AVWriterError> {
        Ok(self
            .writer_info()?
            .available_media_types
            .iter()
            .map(|value| MediaType::from_raw(value))
            .collect())
    }

    pub fn metadata(&self) -> Result<Vec<MetadataItem>, AVWriterError> {
        Ok(self.writer_info()?.metadata)
    }

    pub fn set_metadata(&self, metadata: &[MetadataItem]) -> Result<(), AVWriterError> {
        let payload = serialize_json(metadata)?;
        let payload_c = cstring_arg(&payload, "metadata json")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status =
            unsafe { ffi::av_writer_set_metadata_json(self.ptr, payload_c.as_ptr(), &mut err_msg) };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn should_optimize_for_network_use(&self) -> Result<bool, AVWriterError> {
        Ok(self.writer_info()?.should_optimize_for_network_use)
    }

    pub fn directory_for_temporary_files(&self) -> Result<Option<String>, AVWriterError> {
        Ok(self.writer_info()?.directory_for_temporary_files)
    }

    pub fn set_directory_for_temporary_files(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<(), AVWriterError> {
        let path_str = path
            .as_ref()
            .to_str()
            .ok_or_else(|| AVWriterError::InvalidArgument("path is not valid UTF-8".into()))?;
        let path_c = cstring_arg(path_str, "directory path")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_set_directory_for_temporary_files(
                self.ptr,
                path_c.as_ptr(),
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn inputs(&self) -> Result<Vec<InputId>, AVWriterError> {
        Ok(self
            .writer_info()?
            .inputs
            .into_iter()
            .map(InputId)
            .collect())
    }

    pub fn input_groups(&self) -> Result<Vec<InputGroupInfo>, AVWriterError> {
        Ok(self
            .writer_info()?
            .input_groups
            .into_iter()
            .map(|group| InputGroupInfo {
                inputs: group.inputs.into_iter().map(InputId).collect(),
                default_input: group.default_input.map(InputId),
            })
            .collect())
    }

    pub fn can_apply_output_settings(
        &self,
        media_type: &MediaType,
        output_settings: Option<&JsonValue>,
    ) -> Result<bool, AVWriterError> {
        let media_type_c = cstring_arg(media_type.as_str(), "media type")?;
        let settings_json = output_settings.map(serialize_json).transpose()?;
        let settings_c = settings_json
            .as_ref()
            .map(|value| cstring_arg(value, "output settings json"))
            .transpose()?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let result = unsafe {
            ffi::av_writer_can_apply_output_settings_json(
                self.ptr,
                media_type_c.as_ptr(),
                settings_c
                    .as_ref()
                    .map_or(ptr::null(), |value| value.as_ptr()),
                &mut err_msg,
            )
        };
        match result {
            0 => Ok(false),
            1 => Ok(true),
            other => Err(unsafe { from_swift(other, err_msg) }),
        }
    }

    pub fn add_input(
        &self,
        media_type: &MediaType,
        output_settings: Option<&JsonValue>,
        source_format_hint: Option<&CMFormatDescription>,
        expects_media_data_in_real_time: bool,
    ) -> Result<InputId, AVWriterError> {
        let media_type_c = cstring_arg(media_type.as_str(), "media type")?;
        let settings_json = output_settings.map(serialize_json).transpose()?;
        let settings_c = settings_json
            .as_ref()
            .map(|value| cstring_arg(value, "output settings json"))
            .transpose()?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let result = unsafe {
            ffi::av_writer_add_input_json(
                self.ptr,
                media_type_c.as_ptr(),
                settings_c
                    .as_ref()
                    .map_or(ptr::null(), |value| value.as_ptr()),
                source_format_hint.map_or(ptr::null_mut(), CMFormatDescription::as_ptr),
                expects_media_data_in_real_time,
                &mut err_msg,
            )
        };
        if result < 0 {
            return Err(unsafe { from_swift(result, err_msg) });
        }
        Ok(InputId(result))
    }

    pub fn add_audio_input_from_sample(
        &self,
        sample_buffer: &CMSampleBuffer,
    ) -> Result<InputId, AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let result = unsafe {
            ffi::av_writer_add_audio_input_from_sample(
                self.ptr,
                sample_buffer.as_ptr(),
                &mut err_msg,
            )
        };
        if result < 0 {
            return Err(unsafe { from_swift(result, err_msg) });
        }
        Ok(InputId(result))
    }

    pub fn add_metadata_input(
        &self,
        specifications: &[MetadataSpecification],
        expects_media_data_in_real_time: bool,
    ) -> Result<InputId, AVWriterError> {
        let payload = serialize_json(specifications)?;
        let payload_c = cstring_arg(&payload, "metadata specifications json")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let result = unsafe {
            ffi::av_writer_add_metadata_input_from_specifications_json(
                self.ptr,
                payload_c.as_ptr(),
                expects_media_data_in_real_time,
                &mut err_msg,
            )
        };
        if result < 0 {
            return Err(unsafe { from_swift(result, err_msg) });
        }
        Ok(InputId(result))
    }

    pub fn add_caption_input(
        &self,
        media_type: &MediaType,
        expects_media_data_in_real_time: bool,
    ) -> Result<InputId, AVWriterError> {
        match media_type {
            MediaType::Text | MediaType::ClosedCaption | MediaType::Subtitle => {}
            _ => {
                return Err(AVWriterError::InvalidArgument(
                    "caption inputs must use text, closed_caption, or subtitle media types".into(),
                ))
            }
        }
        let input_id = self.add_input(media_type, None, None, expects_media_data_in_real_time)?;
        self.attach_caption_adaptor(input_id)?;
        Ok(input_id)
    }

    pub fn end_session(&self, end_time: impl Into<Time>) -> Result<(), AVWriterError> {
        let time = end_time.into();
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_end_session(
                self.ptr,
                time_value(&time),
                time_scale(&time),
                time_kind(&time),
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn cancel(self) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe { ffi::av_writer_cancel(self.ptr, &mut err_msg) };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn attach_pixel_buffer_adaptor(
        &self,
        input_id: InputId,
        source_pixel_buffer_attributes: Option<&JsonValue>,
    ) -> Result<(), AVWriterError> {
        let attrs_json = source_pixel_buffer_attributes
            .map(serialize_json)
            .transpose()?;
        let attrs_c = attrs_json
            .as_ref()
            .map(|value| cstring_arg(value, "pixel buffer attributes json"))
            .transpose()?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_attach_pixel_buffer_adaptor_json(
                self.ptr,
                input_id.0,
                attrs_c.as_ref().map_or(ptr::null(), |value| value.as_ptr()),
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn pixel_buffer_source_attributes(
        &self,
        input_id: InputId,
    ) -> Result<Option<JsonValue>, AVWriterError> {
        parse_optional_json_string(
            self.input_info(input_id)?
                .pixel_buffer_source_attributes_json,
        )
    }

    pub fn pixel_buffer_pool(
        &self,
        input_id: InputId,
    ) -> Result<Option<CVPixelBufferPool>, AVWriterError> {
        let ptr = unsafe { ffi::av_writer_pixel_buffer_pool(self.ptr, input_id.0) };
        Ok(CVPixelBufferPool::from_raw(ptr))
    }

    pub fn attach_tagged_pixel_buffer_group_adaptor(
        &self,
        input_id: InputId,
        source_pixel_buffer_attributes: Option<&JsonValue>,
    ) -> Result<(), AVWriterError> {
        let attrs_json = source_pixel_buffer_attributes
            .map(serialize_json)
            .transpose()?;
        let attrs_c = attrs_json
            .as_ref()
            .map(|value| cstring_arg(value, "tagged pixel buffer attributes json"))
            .transpose()?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_attach_tagged_pixel_buffer_group_adaptor_json(
                self.ptr,
                input_id.0,
                attrs_c.as_ref().map_or(ptr::null(), |value| value.as_ptr()),
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn tagged_pixel_buffer_source_attributes(
        &self,
        input_id: InputId,
    ) -> Result<Option<JsonValue>, AVWriterError> {
        parse_optional_json_string(
            self.input_info(input_id)?
                .tagged_pixel_buffer_source_attributes_json,
        )
    }

    pub fn tagged_pixel_buffer_pool(
        &self,
        input_id: InputId,
    ) -> Result<Option<CVPixelBufferPool>, AVWriterError> {
        let ptr = unsafe { ffi::av_writer_tagged_pixel_buffer_pool(self.ptr, input_id.0) };
        Ok(CVPixelBufferPool::from_raw(ptr))
    }

    pub fn append_tagged_pixel_buffer_group(
        &self,
        input_id: InputId,
        buffers: &[TaggedPixelBuffer<'_>],
        presentation_time: impl Into<Time>,
    ) -> Result<(), AVWriterError> {
        let time = presentation_time.into();
        let pixel_buffers: Vec<*mut c_void> = buffers
            .iter()
            .map(|entry| entry.pixel_buffer.as_ptr())
            .collect();
        let layer_ids: Vec<i64> = buffers.iter().map(|entry| entry.layer_id).collect();
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_append_tagged_pixel_buffer_group(
                self.ptr,
                input_id.0,
                pixel_buffers.as_ptr(),
                layer_ids.as_ptr(),
                buffers.len(),
                time_value(&time),
                time_scale(&time),
                time_kind(&time),
                &mut err_msg,
            )
        };
        match status {
            ffi::status::OK => Ok(()),
            ffi::status::INPUT_NOT_READY => Err(AVWriterError::InputNotReady),
            other => Err(unsafe { from_swift(other, err_msg) }),
        }
    }

    pub fn attach_metadata_adaptor(&self, input_id: InputId) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status =
            unsafe { ffi::av_writer_attach_metadata_adaptor(self.ptr, input_id.0, &mut err_msg) };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn input_has_metadata_adaptor(&self, input_id: InputId) -> Result<bool, AVWriterError> {
        Ok(self.input_info(input_id)?.has_metadata_adaptor)
    }

    pub fn append_timed_metadata_group(
        &self,
        input_id: InputId,
        group: &TimedMetadataGroup,
    ) -> Result<(), AVWriterError> {
        let payload = serialize_json(group)?;
        let payload_c = cstring_arg(&payload, "timed metadata group json")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_append_timed_metadata_group_json(
                self.ptr,
                input_id.0,
                payload_c.as_ptr(),
                &mut err_msg,
            )
        };
        match status {
            ffi::status::OK => Ok(()),
            ffi::status::INPUT_NOT_READY => Err(AVWriterError::InputNotReady),
            other => Err(unsafe { from_swift(other, err_msg) }),
        }
    }

    pub fn attach_caption_adaptor(&self, input_id: InputId) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status =
            unsafe { ffi::av_writer_attach_caption_adaptor(self.ptr, input_id.0, &mut err_msg) };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn input_has_caption_adaptor(&self, input_id: InputId) -> Result<bool, AVWriterError> {
        Ok(self.input_info(input_id)?.has_caption_adaptor)
    }

    pub fn append_caption(
        &self,
        input_id: InputId,
        caption: &Caption,
    ) -> Result<(), AVWriterError> {
        let payload = serialize_json(caption)?;
        let payload_c = cstring_arg(&payload, "caption json")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_append_caption_json(
                self.ptr,
                input_id.0,
                payload_c.as_ptr(),
                &mut err_msg,
            )
        };
        match status {
            ffi::status::OK => Ok(()),
            ffi::status::INPUT_NOT_READY => Err(AVWriterError::InputNotReady),
            other => Err(unsafe { from_swift(other, err_msg) }),
        }
    }

    pub fn append_caption_group(
        &self,
        input_id: InputId,
        caption_group: &CaptionGroup,
    ) -> Result<(), AVWriterError> {
        let payload = serialize_json(caption_group)?;
        let payload_c = cstring_arg(&payload, "caption group json")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_append_caption_group_json(
                self.ptr,
                input_id.0,
                payload_c.as_ptr(),
                &mut err_msg,
            )
        };
        match status {
            ffi::status::OK => Ok(()),
            ffi::status::INPUT_NOT_READY => Err(AVWriterError::InputNotReady),
            other => Err(unsafe { from_swift(other, err_msg) }),
        }
    }

    pub fn input_media_type(&self, input_id: InputId) -> Result<MediaType, AVWriterError> {
        Ok(MediaType::from_raw(&self.input_info(input_id)?.media_type))
    }

    pub fn input_output_settings(
        &self,
        input_id: InputId,
    ) -> Result<Option<JsonValue>, AVWriterError> {
        parse_optional_json_string(self.input_info(input_id)?.output_settings_json)
    }

    pub fn input_source_format_hint(
        &self,
        input_id: InputId,
    ) -> Result<Option<CMFormatDescription>, AVWriterError> {
        let ptr = unsafe { ffi::av_writer_input_source_format_hint(self.ptr, input_id.0) };
        Ok(CMFormatDescription::from_raw(ptr))
    }

    pub fn input_metadata(&self, input_id: InputId) -> Result<Vec<MetadataItem>, AVWriterError> {
        Ok(self.input_info(input_id)?.metadata)
    }

    pub fn set_input_metadata(
        &self,
        input_id: InputId,
        metadata: &[MetadataItem],
    ) -> Result<(), AVWriterError> {
        let payload = serialize_json(metadata)?;
        let payload_c = cstring_arg(&payload, "input metadata json")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_set_input_metadata_json(
                self.ptr,
                input_id.0,
                payload_c.as_ptr(),
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn input_ready_for_more_media_data(
        &self,
        input_id: InputId,
    ) -> Result<bool, AVWriterError> {
        Ok(self.input_info(input_id)?.ready_for_more_media_data)
    }

    pub fn input_expects_media_data_in_real_time(
        &self,
        input_id: InputId,
    ) -> Result<bool, AVWriterError> {
        Ok(self.input_info(input_id)?.expects_media_data_in_real_time)
    }

    pub fn set_input_expects_media_data_in_real_time(
        &self,
        input_id: InputId,
        expects_media_data_in_real_time: bool,
    ) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_input_set_expects_media_data_in_real_time(
                self.ptr,
                input_id.0,
                expects_media_data_in_real_time,
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn request_media_data_when_ready(
        &self,
        input_id: InputId,
        callback: impl FnMut() + Send + 'static,
    ) -> Result<(), AVWriterError> {
        let state = Box::new(ReadyCallbackState {
            callback: Box::new(callback),
        });
        let userdata = Box::into_raw(state).cast::<c_void>();
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_input_request_media_data_when_ready(
                self.ptr,
                input_id.0,
                Some(ready_callback_trampoline),
                userdata,
                Some(ready_callback_drop),
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            unsafe { ready_callback_drop(userdata) };
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn mark_input_as_finished(&self, input_id: InputId) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status =
            unsafe { ffi::av_writer_input_mark_as_finished(self.ptr, input_id.0, &mut err_msg) };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn input_language_code(&self, input_id: InputId) -> Result<Option<String>, AVWriterError> {
        Ok(self.input_info(input_id)?.language_code)
    }

    pub fn set_input_language_code(
        &self,
        input_id: InputId,
        language_code: Option<&str>,
    ) -> Result<(), AVWriterError> {
        self.set_optional_input_string(
            input_id,
            language_code,
            ffi::av_writer_input_set_language_code,
            "language code",
        )
    }

    pub fn input_extended_language_tag(
        &self,
        input_id: InputId,
    ) -> Result<Option<String>, AVWriterError> {
        Ok(self.input_info(input_id)?.extended_language_tag)
    }

    pub fn set_input_extended_language_tag(
        &self,
        input_id: InputId,
        extended_language_tag: Option<&str>,
    ) -> Result<(), AVWriterError> {
        self.set_optional_input_string(
            input_id,
            extended_language_tag,
            ffi::av_writer_input_set_extended_language_tag,
            "extended language tag",
        )
    }

    pub fn input_natural_size(&self, input_id: InputId) -> Result<(f64, f64), AVWriterError> {
        let info = self.input_info(input_id)?;
        Ok((info.natural_size.width, info.natural_size.height))
    }

    pub fn set_input_natural_size(
        &self,
        input_id: InputId,
        width: f64,
        height: f64,
    ) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_input_set_natural_size(self.ptr, input_id.0, width, height, &mut err_msg)
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn input_transform(&self, input_id: InputId) -> Result<[f64; 6], AVWriterError> {
        let info = self.input_info(input_id)?;
        Ok([
            info.transform.a,
            info.transform.b,
            info.transform.c,
            info.transform.d,
            info.transform.tx,
            info.transform.ty,
        ])
    }

    pub fn set_input_transform(
        &self,
        input_id: InputId,
        transform: [f64; 6],
    ) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_input_set_transform(
                self.ptr,
                input_id.0,
                transform[0],
                transform[1],
                transform[2],
                transform[3],
                transform[4],
                transform[5],
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn input_preferred_volume(&self, input_id: InputId) -> Result<f32, AVWriterError> {
        Ok(self.input_info(input_id)?.preferred_volume)
    }

    pub fn set_input_preferred_volume(
        &self,
        input_id: InputId,
        preferred_volume: f32,
    ) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_input_set_preferred_volume(
                self.ptr,
                input_id.0,
                preferred_volume,
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn input_marks_output_track_as_enabled(
        &self,
        input_id: InputId,
    ) -> Result<bool, AVWriterError> {
        Ok(self.input_info(input_id)?.marks_output_track_as_enabled)
    }

    pub fn set_input_marks_output_track_as_enabled(
        &self,
        input_id: InputId,
        enabled: bool,
    ) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_input_set_marks_output_track_as_enabled(
                self.ptr,
                input_id.0,
                enabled,
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn input_media_time_scale(&self, input_id: InputId) -> Result<i32, AVWriterError> {
        Ok(self.input_info(input_id)?.media_time_scale)
    }

    pub fn set_input_media_time_scale(
        &self,
        input_id: InputId,
        media_time_scale: i32,
    ) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_input_set_media_time_scale(
                self.ptr,
                input_id.0,
                media_time_scale,
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn input_preferred_media_chunk_duration(
        &self,
        input_id: InputId,
    ) -> Result<Time, AVWriterError> {
        Ok(self.input_info(input_id)?.preferred_media_chunk_duration)
    }

    pub fn set_input_preferred_media_chunk_duration(
        &self,
        input_id: InputId,
        duration: impl Into<Time>,
    ) -> Result<(), AVWriterError> {
        let duration = duration.into();
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_input_set_preferred_media_chunk_duration(
                self.ptr,
                input_id.0,
                time_value(&duration),
                time_scale(&duration),
                time_kind(&duration),
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn input_preferred_media_chunk_alignment(
        &self,
        input_id: InputId,
    ) -> Result<i64, AVWriterError> {
        Ok(self.input_info(input_id)?.preferred_media_chunk_alignment)
    }

    pub fn set_input_preferred_media_chunk_alignment(
        &self,
        input_id: InputId,
        alignment: i64,
    ) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_input_set_preferred_media_chunk_alignment(
                self.ptr,
                input_id.0,
                alignment,
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn input_sample_reference_base_url(
        &self,
        input_id: InputId,
    ) -> Result<Option<String>, AVWriterError> {
        Ok(self.input_info(input_id)?.sample_reference_base_url)
    }

    pub fn set_input_sample_reference_base_url(
        &self,
        input_id: InputId,
        url: Option<&str>,
    ) -> Result<(), AVWriterError> {
        self.set_optional_input_string(
            input_id,
            url,
            ffi::av_writer_input_set_sample_reference_base_url,
            "sample reference base URL",
        )
    }

    pub fn input_media_data_location(
        &self,
        input_id: InputId,
    ) -> Result<Option<InputMediaDataLocation>, AVWriterError> {
        Ok(self
            .input_info(input_id)?
            .media_data_location
            .as_deref()
            .map(InputMediaDataLocation::from_raw))
    }

    pub fn set_input_media_data_location(
        &self,
        input_id: InputId,
        location: InputMediaDataLocation,
    ) -> Result<(), AVWriterError> {
        let location_c = cstring_arg(location.as_str(), "media data location")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_input_set_media_data_location(
                self.ptr,
                input_id.0,
                location_c.as_ptr(),
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn can_add_track_association(
        &self,
        input_id: InputId,
        other_input_id: InputId,
        association_type: &TrackAssociationType,
    ) -> Result<bool, AVWriterError> {
        let association_type_c = cstring_arg(association_type.as_str(), "track association type")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_input_can_add_track_association(
                self.ptr,
                input_id.0,
                other_input_id.0,
                association_type_c.as_ptr(),
                &mut err_msg,
            )
        };
        match status {
            0 => Ok(false),
            1 => Ok(true),
            other => Err(unsafe { from_swift(other, err_msg) }),
        }
    }

    pub fn add_track_association(
        &self,
        input_id: InputId,
        other_input_id: InputId,
        association_type: &TrackAssociationType,
    ) -> Result<(), AVWriterError> {
        let association_type_c = cstring_arg(association_type.as_str(), "track association type")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_input_add_track_association(
                self.ptr,
                input_id.0,
                other_input_id.0,
                association_type_c.as_ptr(),
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn input_performs_multi_pass_encoding_if_supported(
        &self,
        input_id: InputId,
    ) -> Result<bool, AVWriterError> {
        Ok(self
            .input_info(input_id)?
            .performs_multi_pass_encoding_if_supported)
    }

    pub fn set_input_performs_multi_pass_encoding_if_supported(
        &self,
        input_id: InputId,
        enabled: bool,
    ) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_input_set_performs_multi_pass_encoding_if_supported(
                self.ptr,
                input_id.0,
                enabled,
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn input_can_perform_multiple_passes(
        &self,
        input_id: InputId,
    ) -> Result<bool, AVWriterError> {
        Ok(self.input_info(input_id)?.can_perform_multiple_passes)
    }

    pub fn input_current_pass_description(
        &self,
        input_id: InputId,
    ) -> Result<Option<InputPassDescription>, AVWriterError> {
        Ok(self.input_info(input_id)?.current_pass_description)
    }

    pub fn respond_to_each_pass_description(
        &self,
        input_id: InputId,
        callback: impl FnMut(Option<InputPassDescription>) + Send + 'static,
    ) -> Result<(), AVWriterError> {
        let state = Box::new(PassDescriptionCallbackState {
            callback: Box::new(callback),
        });
        let userdata = Box::into_raw(state).cast::<c_void>();
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_input_respond_to_each_pass_description(
                self.ptr,
                input_id.0,
                Some(pass_description_callback_trampoline),
                userdata,
                Some(pass_description_callback_drop),
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            unsafe { pass_description_callback_drop(userdata) };
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn mark_current_pass_as_finished(&self, input_id: InputId) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_input_mark_current_pass_as_finished(self.ptr, input_id.0, &mut err_msg)
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn movie_fragment_interval(&self) -> Result<Time, AVWriterError> {
        Ok(self.writer_info()?.movie_fragment_interval)
    }

    pub fn set_movie_fragment_interval(
        &self,
        interval: impl Into<Time>,
    ) -> Result<(), AVWriterError> {
        self.set_writer_time(ffi::av_writer_set_movie_fragment_interval, interval.into())
    }

    pub fn initial_movie_fragment_interval(&self) -> Result<Time, AVWriterError> {
        Ok(self.writer_info()?.initial_movie_fragment_interval)
    }

    pub fn set_initial_movie_fragment_interval(
        &self,
        interval: impl Into<Time>,
    ) -> Result<(), AVWriterError> {
        self.set_writer_time(
            ffi::av_writer_set_initial_movie_fragment_interval,
            interval.into(),
        )
    }

    pub fn initial_movie_fragment_sequence_number(&self) -> Result<i64, AVWriterError> {
        Ok(self.writer_info()?.initial_movie_fragment_sequence_number)
    }

    pub fn set_initial_movie_fragment_sequence_number(
        &self,
        sequence_number: i64,
    ) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_set_initial_movie_fragment_sequence_number(
                self.ptr,
                sequence_number,
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn produces_combinable_fragments(&self) -> Result<bool, AVWriterError> {
        Ok(self.writer_info()?.produces_combinable_fragments)
    }

    pub fn set_produces_combinable_fragments(&self, enabled: bool) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_set_produces_combinable_fragments(self.ptr, enabled, &mut err_msg)
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn overall_duration_hint(&self) -> Result<Time, AVWriterError> {
        Ok(self.writer_info()?.overall_duration_hint)
    }

    pub fn set_overall_duration_hint(&self, hint: impl Into<Time>) -> Result<(), AVWriterError> {
        self.set_writer_time(ffi::av_writer_set_overall_duration_hint, hint.into())
    }

    pub fn movie_time_scale(&self) -> Result<i32, AVWriterError> {
        Ok(self.writer_info()?.movie_time_scale)
    }

    pub fn set_movie_time_scale(&self, movie_time_scale: i32) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_set_movie_time_scale(self.ptr, movie_time_scale, &mut err_msg)
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn preferred_output_segment_interval(&self) -> Result<Time, AVWriterError> {
        Ok(self.writer_info()?.preferred_output_segment_interval)
    }

    pub fn set_preferred_output_segment_interval(
        &self,
        interval: impl Into<Time>,
    ) -> Result<(), AVWriterError> {
        self.set_writer_time(
            ffi::av_writer_set_preferred_output_segment_interval,
            interval.into(),
        )
    }

    pub fn initial_segment_start_time(&self) -> Result<Time, AVWriterError> {
        Ok(self.writer_info()?.initial_segment_start_time)
    }

    pub fn set_initial_segment_start_time(
        &self,
        start_time: impl Into<Time>,
    ) -> Result<(), AVWriterError> {
        self.set_writer_time(
            ffi::av_writer_set_initial_segment_start_time,
            start_time.into(),
        )
    }

    pub fn output_file_type_profile(&self) -> Result<Option<FileTypeProfile>, AVWriterError> {
        Ok(self
            .writer_info()?
            .output_file_type_profile
            .as_deref()
            .map(FileTypeProfile::from_raw))
    }

    pub fn set_output_file_type_profile(
        &self,
        profile: Option<FileTypeProfile>,
    ) -> Result<(), AVWriterError> {
        let profile_c = profile
            .as_ref()
            .map(|value| cstring_arg(value.as_str(), "file type profile"))
            .transpose()?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_set_output_file_type_profile(
                self.ptr,
                profile_c
                    .as_ref()
                    .map_or(ptr::null(), |value| value.as_ptr()),
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn flush_segment(&self) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe { ffi::av_writer_flush_segment(self.ptr, &mut err_msg) };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    fn writer_info(&self) -> Result<WriterInfoPayload, AVWriterError> {
        let ptr = unsafe { ffi::av_writer_info_json(self.ptr) };
        parse_json_ptr(ptr, "writer info")
    }

    fn input_info(&self, input_id: InputId) -> Result<InputInfoPayload, AVWriterError> {
        let ptr = unsafe { ffi::av_writer_input_info_json(self.ptr, input_id.0) };
        parse_json_ptr(ptr, "input info")
    }

    fn set_writer_time(
        &self,
        func: unsafe extern "C" fn(*mut c_void, i64, i32, i32, *mut *mut c_char) -> i32,
        time: Time,
    ) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            func(
                self.ptr,
                time_value(&time),
                time_scale(&time),
                time_kind(&time),
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    fn set_optional_input_string(
        &self,
        input_id: InputId,
        value: Option<&str>,
        func: unsafe extern "C" fn(*mut c_void, i32, *const c_char, *mut *mut c_char) -> i32,
        context: &str,
    ) -> Result<(), AVWriterError> {
        let value_c = value.map(|raw| cstring_arg(raw, context)).transpose()?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            func(
                self.ptr,
                input_id.0,
                value_c.as_ref().map_or(ptr::null(), |raw| raw.as_ptr()),
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }
}

fn serialize_json<T: ?Sized + Serialize>(value: &T) -> Result<String, AVWriterError> {
    serde_json::to_string(value).map_err(|e| {
        AVWriterError::InvalidArgument(format!("failed to serialize json payload: {e}"))
    })
}

fn parse_json_ptr<T: for<'de> Deserialize<'de>>(
    ptr: *mut c_char,
    context: &str,
) -> Result<T, AVWriterError> {
    let raw = take_swift_string(ptr).ok_or_else(|| {
        AVWriterError::InvalidState(format!("swift bridge returned no {context} payload"))
    })?;
    serde_json::from_str(&raw).map_err(|e| {
        AVWriterError::InvalidState(format!(
            "failed to decode {context} json from swift bridge: {e}"
        ))
    })
}

fn parse_optional_json_string(raw: Option<String>) -> Result<Option<JsonValue>, AVWriterError> {
    raw.map(|value| {
        serde_json::from_str(&value).map_err(|e| {
            AVWriterError::InvalidState(format!("failed to decode nested json payload: {e}"))
        })
    })
    .transpose()
}

fn cstring_arg(value: &str, context: &str) -> Result<CString, AVWriterError> {
    CString::new(value)
        .map_err(|e| AVWriterError::InvalidArgument(format!("{context} contained NUL byte: {e}")))
}

fn take_swift_string(ptr: *mut c_char) -> Option<String> {
    if ptr.is_null() {
        return None;
    }
    unsafe {
        let string = std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned();
        ffi::avw_string_free(ptr);
        Some(string)
    }
}

const fn time_kind(time: &Time) -> i32 {
    match time {
        Time::Numeric { .. } => 0,
        Time::Invalid => 1,
        Time::Indefinite => 2,
        Time::PositiveInfinity => 3,
        Time::NegativeInfinity => 4,
    }
}

const fn time_value(time: &Time) -> i64 {
    match time {
        Time::Numeric { value, .. } => *value,
        Time::Invalid | Time::Indefinite | Time::PositiveInfinity | Time::NegativeInfinity => 0,
    }
}

const fn time_scale(time: &Time) -> i32 {
    match time {
        Time::Numeric { timescale, .. } => *timescale,
        Time::Invalid | Time::Indefinite | Time::PositiveInfinity | Time::NegativeInfinity => 0,
    }
}
