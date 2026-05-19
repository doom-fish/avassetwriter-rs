#![allow(clippy::missing_errors_doc, clippy::needless_pass_by_value)]

use core::ffi::{c_char, c_void};
use core::ptr;
use std::ops::Deref;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::bridge_support::{
    cstring_arg, parse_json_ptr, parse_optional_json_string, take_swift_string, time_kind,
    time_scale, time_value,
};
use crate::error::{from_swift, AVWriterError};
use crate::ffi;
use crate::time::{Time, TimeRange};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MetadataItemFilterKind {
    Sharing,
    Other(String),
}

impl MetadataItemFilterKind {
    fn from_raw(raw: String) -> Self {
        match raw.as_str() {
            "sharing" => Self::Sharing,
            _ => Self::Other(raw),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum VideoCompositorClass {
    Passthrough,
}

impl VideoCompositorClass {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Passthrough => "passthrough",
        }
    }

    fn from_class_name(name: &str) -> Option<Self> {
        match name.rsplit('.').next() {
            Some("AVWRPassthroughVideoCompositor") => Some(Self::Passthrough),
            _ => None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AudioMixInfoPayload {
    input_parameter_count: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AudioMixInputParametersInfoPayload {
    track_id: i32,
    audio_time_pitch_algorithm: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AudioVolumeRampPayload {
    start_volume: f32,
    end_volume: f32,
    time_range: TimeRange,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SizePayload {
    width: f64,
    height: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VideoCompositionInfoPayload {
    frame_duration: Time,
    render_size: SizePayload,
    render_scale: f32,
    instruction_count: usize,
    custom_video_compositor_class_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct VideoCompositorInfoPayload {
    class_name: String,
    source_pixel_buffer_attributes_json: Option<String>,
    required_pixel_buffer_attributes_for_render_context_json: Option<String>,
    supports_wide_color_source_frames: bool,
    #[serde(rename = "supportsHDRSourceFrames")]
    supports_hdr_source_frames: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RectPayload {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AsynchronousVideoCompositionRequestPayload {
    composition_time: Time,
    source_track_ids: Vec<i32>,
    source_sample_data_track_ids: Vec<i32>,
    render_size: SizePayload,
    render_scale: f32,
    video_composition_instruction_class_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AsynchronousCIImageFilteringRequestPayload {
    composition_time: Time,
    render_size: SizePayload,
    source_image_extent: RectPayload,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Snapshot of an `AVAsynchronousVideoCompositionRequest`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AsynchronousVideoCompositionRequest {
    pub composition_time: Time,
    pub source_track_ids: Vec<i32>,
    pub source_sample_data_track_ids: Vec<i32>,
    pub render_size: (f64, f64),
    pub render_scale: f32,
    pub video_composition_instruction_class_name: Option<String>,
}

/// Snapshot of an `AVAsynchronousCIImageFilteringRequest`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AsynchronousCIImageFilteringRequest {
    pub composition_time: Time,
    pub render_size: (f64, f64),
    pub source_image_extent: ImageRect,
}

/// Safe wrapper around `AVMetadataItemFilter`.
pub struct MetadataItemFilter {
    ptr: *mut c_void,
}

// SAFETY: `MetadataItemFilter` wraps an ARC-retained `AVMetadataItemFilter`
// pointer.  ARC retain/release are atomic; moving across threads is safe.
// Concurrent shared access (`Sync`) is not implemented.
unsafe impl Send for MetadataItemFilter {}

impl MetadataItemFilter {
    /// Return Apple's built-in metadata filter for safe sharing/export.
    pub fn for_sharing() -> Result<Self, AVWriterError> {
        let ptr = unsafe { ffi::av_metadata_item_filter_for_sharing() };
        Self::from_raw(ptr).ok_or_else(|| missing_payload("metadata-item filter"))
    }

    /// Identify which built-in filter this wrapper represents.
    pub fn kind(&self) -> Result<MetadataItemFilterKind, AVWriterError> {
        let raw = take_required_string(
            unsafe { ffi::av_metadata_item_filter_kind(self.ptr) },
            "metadata-item filter kind",
        )?;
        Ok(MetadataItemFilterKind::from_raw(raw))
    }

    pub(crate) const fn as_ptr(&self) -> *mut c_void {
        self.ptr
    }

    pub(crate) fn from_raw(ptr: *mut c_void) -> Option<Self> {
        (!ptr.is_null()).then_some(Self { ptr })
    }
}

impl Drop for MetadataItemFilter {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ffi::av_metadata_item_filter_release(self.ptr) };
        }
    }
}

impl core::fmt::Debug for MetadataItemFilter {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MetadataItemFilter")
            .field("ptr", &self.ptr)
            .finish()
    }
}

/// A readback of one configured audio volume ramp.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AudioVolumeRamp {
    pub start_volume: f32,
    pub end_volume: f32,
    pub time_range: TimeRange,
}

/// Safe wrapper around `AVAudioMix`.
pub struct AudioMix {
    ptr: *mut c_void,
}

// SAFETY: `AudioMix` wraps an ARC-retained `AVAudioMix` pointer.
// ARC retain/release are atomic; moving across threads is safe.
// Concurrent shared access (`Sync`) is not implemented.
unsafe impl Send for AudioMix {}

impl AudioMix {
    /// Create an empty mutable audio mix.
    pub fn new() -> Result<Self, AVWriterError> {
        let ptr = unsafe { ffi::av_audio_mix_create() };
        Self::from_raw(ptr).ok_or_else(|| missing_payload("audio mix"))
    }

    /// Return the number of input-parameter entries currently configured.
    pub fn input_parameter_count(&self) -> Result<usize, AVWriterError> {
        Ok(self.info()?.input_parameter_count)
    }

    /// Return every configured input-parameter entry.
    pub fn input_parameters(&self) -> Result<Vec<AudioMixInputParameters>, AVWriterError> {
        let count = unsafe { ffi::av_audio_mix_input_parameter_count(self.ptr) };
        let mut parameters = Vec::with_capacity(count);
        for index in 0..count {
            let ptr = unsafe { ffi::av_audio_mix_copy_input_parameter_at_index(self.ptr, index) };
            let parameter = AudioMixInputParameters::from_raw(ptr)
                .ok_or_else(|| missing_payload("audio mix input parameters"))?;
            parameters.push(parameter);
        }
        Ok(parameters)
    }

    pub(crate) const fn as_ptr(&self) -> *mut c_void {
        self.ptr
    }

    pub(crate) fn from_raw(ptr: *mut c_void) -> Option<Self> {
        (!ptr.is_null()).then_some(Self { ptr })
    }

    fn info(&self) -> Result<AudioMixInfoPayload, AVWriterError> {
        let ptr = unsafe { ffi::av_audio_mix_info_json(self.ptr) };
        parse_json_ptr(ptr, "audio mix info")
    }
}

impl Drop for AudioMix {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ffi::av_audio_mix_release(self.ptr) };
        }
    }
}

impl core::fmt::Debug for AudioMix {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AudioMix").field("ptr", &self.ptr).finish()
    }
}

/// Safe wrapper around `AVAudioMixInputParameters`.
pub struct AudioMixInputParameters {
    ptr: *mut c_void,
}

// SAFETY: `AudioMixInputParameters` wraps an ARC-retained Objective-C object.
unsafe impl Send for AudioMixInputParameters {}

impl AudioMixInputParameters {
    pub fn track_id(&self) -> Result<i32, AVWriterError> {
        Ok(self.info()?.track_id)
    }

    pub fn audio_time_pitch_algorithm(&self) -> Result<Option<String>, AVWriterError> {
        Ok(self.info()?.audio_time_pitch_algorithm)
    }

    pub fn volume_ramp_for_time(
        &self,
        time: impl Into<Time>,
    ) -> Result<Option<AudioVolumeRamp>, AVWriterError> {
        let time = time.into();
        let mut err_msg: *mut c_char = ptr::null_mut();
        let ptr = unsafe {
            ffi::av_audio_mix_input_parameters_volume_ramp_json(
                self.ptr,
                time_value(&time),
                time_scale(&time),
                time_kind(&time),
                &mut err_msg,
            )
        };
        if ptr.is_null() {
            return if err_msg.is_null() {
                Ok(None)
            } else {
                Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) })
            };
        }
        let payload: AudioVolumeRampPayload = parse_json_ptr(ptr, "audio volume ramp")?;
        Ok(Some(AudioVolumeRamp {
            start_volume: payload.start_volume,
            end_volume: payload.end_volume,
            time_range: payload.time_range,
        }))
    }

    pub(crate) const fn as_ptr(&self) -> *mut c_void {
        self.ptr
    }

    pub(crate) fn from_raw(ptr: *mut c_void) -> Option<Self> {
        (!ptr.is_null()).then_some(Self { ptr })
    }

    fn info(&self) -> Result<AudioMixInputParametersInfoPayload, AVWriterError> {
        let ptr = unsafe { ffi::av_audio_mix_input_parameters_info_json(self.ptr) };
        parse_json_ptr(ptr, "audio mix input parameters info")
    }
}

impl Drop for AudioMixInputParameters {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ffi::av_audio_mix_input_parameters_release(self.ptr) };
        }
    }
}

impl core::fmt::Debug for AudioMixInputParameters {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AudioMixInputParameters")
            .field("ptr", &self.ptr)
            .finish()
    }
}

/// Safe wrapper around `AVMutableAudioMixInputParameters`.
pub struct MutableAudioMixInputParameters {
    parameters: AudioMixInputParameters,
}

// SAFETY: `MutableAudioMixInputParameters` owns an ARC-retained Objective-C object.
unsafe impl Send for MutableAudioMixInputParameters {}

impl Deref for MutableAudioMixInputParameters {
    type Target = AudioMixInputParameters;

    fn deref(&self) -> &Self::Target {
        &self.parameters
    }
}

impl MutableAudioMixInputParameters {
    pub fn new() -> Result<Self, AVWriterError> {
        let ptr = unsafe { ffi::av_audio_mix_input_parameters_create() };
        let parameters = AudioMixInputParameters::from_raw(ptr)
            .ok_or_else(|| missing_payload("mutable audio mix input parameters"))?;
        Ok(Self { parameters })
    }

    #[must_use]
    pub const fn as_input_parameters(&self) -> &AudioMixInputParameters {
        &self.parameters
    }

    pub fn set_track_id(&self, track_id: i32) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_audio_mix_input_parameters_set_track_id(
                self.parameters.ptr,
                track_id,
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn set_audio_time_pitch_algorithm(
        &self,
        algorithm: Option<&str>,
    ) -> Result<(), AVWriterError> {
        let algorithm_c = algorithm
            .map(|value| cstring_arg(value, "audio time-pitch algorithm"))
            .transpose()?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_audio_mix_input_parameters_set_audio_time_pitch_algorithm(
                self.parameters.ptr,
                algorithm_c
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

    pub fn set_volume(&self, volume: f32, time: impl Into<Time>) -> Result<(), AVWriterError> {
        let time = time.into();
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_audio_mix_input_parameters_set_volume(
                self.parameters.ptr,
                volume,
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

    pub fn set_volume_ramp(
        &self,
        start_volume: f32,
        end_volume: f32,
        time_range: TimeRange,
    ) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_audio_mix_input_parameters_set_volume_ramp(
                self.parameters.ptr,
                start_volume,
                end_volume,
                time_value(&time_range.start),
                time_scale(&time_range.start),
                time_kind(&time_range.start),
                time_value(&time_range.duration),
                time_scale(&time_range.duration),
                time_kind(&time_range.duration),
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }
}

/// Safe wrapper around `AVMutableAudioMix`.
pub struct MutableAudioMix {
    mix: AudioMix,
}

// SAFETY: `MutableAudioMix` owns an ARC-retained Objective-C object.
unsafe impl Send for MutableAudioMix {}

impl Deref for MutableAudioMix {
    type Target = AudioMix;

    fn deref(&self) -> &Self::Target {
        &self.mix
    }
}

impl MutableAudioMix {
    pub fn new() -> Result<Self, AVWriterError> {
        AudioMix::new().map(|mix| Self { mix })
    }

    #[must_use]
    pub const fn as_audio_mix(&self) -> &AudioMix {
        &self.mix
    }

    pub fn set_input_parameters(
        &self,
        parameters: &[AudioMixInputParameters],
    ) -> Result<(), AVWriterError> {
        let raw_parameters = parameters
            .iter()
            .map(AudioMixInputParameters::as_ptr)
            .collect::<Vec<_>>();
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_audio_mix_set_input_parameters(
                self.mix.ptr,
                raw_parameters.as_ptr(),
                raw_parameters.len(),
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }
}

/// Safe wrapper around `AVVideoComposition` / `AVMutableVideoComposition`.
pub struct VideoComposition {
    ptr: *mut c_void,
}

// SAFETY: `VideoComposition` wraps an ARC-retained `AVVideoComposition`
// pointer.  ARC retain/release are atomic; moving across threads is safe.
// Concurrent shared access (`Sync`) is not implemented.
unsafe impl Send for VideoComposition {}

impl VideoComposition {
    /// Create a mutable composition seeded from the video tracks in `path`.
    pub fn from_asset(path: impl AsRef<Path>) -> Result<Self, AVWriterError> {
        let path_c = path_arg(path, "video composition asset path")?;
        let ptr = unsafe { ffi::av_video_composition_create_from_asset(path_c.as_ptr()) };
        Self::from_raw(ptr).ok_or_else(|| missing_payload("video composition"))
    }

    /// Create a mutable composition whose `CIFilter` handler records request snapshots
    /// and forwards the source image unchanged.
    pub fn from_asset_with_ci_filter_passthrough(
        path: impl AsRef<Path>,
    ) -> Result<Self, AVWriterError> {
        let path_c = path_arg(path, "video composition asset path")?;
        let ptr = unsafe {
            ffi::av_video_composition_create_from_asset_ci_filter_recorder(path_c.as_ptr())
        };
        Self::from_raw(ptr).ok_or_else(|| missing_payload("video composition"))
    }

    /// Take the most recently recorded CI image filtering request snapshot, if any.
    pub fn take_latest_ci_image_filtering_request(
    ) -> Result<Option<AsynchronousCIImageFilteringRequest>, AVWriterError> {
        parse_optional_bridge_json(
            unsafe { ffi::av_take_latest_ci_image_filtering_request_json() },
            "ci image filtering request",
        )
        .map(|payload| {
            payload.map(|payload: AsynchronousCIImageFilteringRequestPayload| {
                AsynchronousCIImageFilteringRequest {
                    composition_time: payload.composition_time,
                    render_size: (payload.render_size.width, payload.render_size.height),
                    source_image_extent: ImageRect {
                        x: payload.source_image_extent.x,
                        y: payload.source_image_extent.y,
                        width: payload.source_image_extent.width,
                        height: payload.source_image_extent.height,
                    },
                }
            })
        })
    }

    /// Return the composition frame duration.
    pub fn frame_duration(&self) -> Result<Time, AVWriterError> {
        Ok(self.info()?.frame_duration)
    }

    /// Return the render size as `(width, height)`.
    pub fn render_size(&self) -> Result<(f64, f64), AVWriterError> {
        let info = self.info()?;
        Ok((info.render_size.width, info.render_size.height))
    }

    /// Return the render scale.
    pub fn render_scale(&self) -> Result<f32, AVWriterError> {
        Ok(self.info()?.render_scale)
    }

    /// Return the number of composition instructions.
    pub fn instruction_count(&self) -> Result<usize, AVWriterError> {
        Ok(self.info()?.instruction_count)
    }

    /// Return the known built-in custom compositor class, if configured.
    pub fn custom_video_compositor_class(
        &self,
    ) -> Result<Option<VideoCompositorClass>, AVWriterError> {
        Ok(self
            .info()?
            .custom_video_compositor_class_name
            .as_deref()
            .and_then(VideoCompositorClass::from_class_name))
    }

    /// Return the Objective-C class name of the configured custom compositor, if any.
    pub fn custom_video_compositor_class_name(&self) -> Result<Option<String>, AVWriterError> {
        Ok(self.info()?.custom_video_compositor_class_name)
    }

    /// Set or clear the custom compositor class.
    pub fn set_custom_video_compositor_class(
        &self,
        class: Option<VideoCompositorClass>,
    ) -> Result<(), AVWriterError> {
        let class_c = class
            .map(|value| cstring_arg(value.as_str(), "video compositor class"))
            .transpose()?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_video_composition_set_custom_video_compositor_class(
                self.ptr,
                class_c.as_ref().map_or(ptr::null(), |value| value.as_ptr()),
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub(crate) const fn as_ptr(&self) -> *mut c_void {
        self.ptr
    }

    pub(crate) fn from_raw(ptr: *mut c_void) -> Option<Self> {
        (!ptr.is_null()).then_some(Self { ptr })
    }

    fn info(&self) -> Result<VideoCompositionInfoPayload, AVWriterError> {
        let ptr = unsafe { ffi::av_video_composition_info_json(self.ptr) };
        parse_json_ptr(ptr, "video composition info")
    }
}

impl Drop for VideoComposition {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ffi::av_video_composition_release(self.ptr) };
        }
    }
}

impl core::fmt::Debug for VideoComposition {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("VideoComposition")
            .field("ptr", &self.ptr)
            .finish()
    }
}

/// Safe wrapper around an `AVVideoCompositing` instance.
pub struct VideoCompositor {
    ptr: *mut c_void,
}

// SAFETY: `VideoCompositor` wraps an ARC-retained `AVVideoCompositing`
// pointer.  ARC retain/release are atomic; moving across threads is safe.
// Concurrent shared access (`Sync`) is not implemented.
unsafe impl Send for VideoCompositor {}

impl VideoCompositor {
    /// Take the most recently recorded video composition request snapshot, if any.
    pub fn take_latest_video_composition_request(
    ) -> Result<Option<AsynchronousVideoCompositionRequest>, AVWriterError> {
        parse_optional_bridge_json(
            unsafe { ffi::av_take_latest_video_composition_request_json() },
            "video composition request",
        )
        .map(|payload| {
            payload.map(|payload: AsynchronousVideoCompositionRequestPayload| {
                AsynchronousVideoCompositionRequest {
                    composition_time: payload.composition_time,
                    source_track_ids: payload.source_track_ids,
                    source_sample_data_track_ids: payload.source_sample_data_track_ids,
                    render_size: (payload.render_size.width, payload.render_size.height),
                    render_scale: payload.render_scale,
                    video_composition_instruction_class_name: payload
                        .video_composition_instruction_class_name,
                }
            })
        })
    }

    /// Return the Objective-C class name for this compositor instance.
    pub fn class_name(&self) -> Result<String, AVWriterError> {
        Ok(self.info()?.class_name)
    }

    /// Return the known built-in compositor kind, if recognized.
    pub fn kind(&self) -> Result<Option<VideoCompositorClass>, AVWriterError> {
        Ok(VideoCompositorClass::from_class_name(
            &self.info()?.class_name,
        ))
    }

    /// Return the compositor's source pixel-buffer requirements, when present.
    pub fn source_pixel_buffer_attributes(&self) -> Result<Option<JsonValue>, AVWriterError> {
        parse_optional_json_string(self.info()?.source_pixel_buffer_attributes_json)
    }

    /// Return the required render-context pixel-buffer attributes.
    pub fn required_pixel_buffer_attributes_for_render_context(
        &self,
    ) -> Result<Option<JsonValue>, AVWriterError> {
        parse_optional_json_string(
            self.info()?
                .required_pixel_buffer_attributes_for_render_context_json,
        )
    }

    /// Report whether the compositor opts into wide-color source frames.
    pub fn supports_wide_color_source_frames(&self) -> Result<bool, AVWriterError> {
        Ok(self.info()?.supports_wide_color_source_frames)
    }

    /// Report whether the compositor opts into HDR source frames.
    pub fn supports_hdr_source_frames(&self) -> Result<bool, AVWriterError> {
        Ok(self.info()?.supports_hdr_source_frames)
    }

    pub(crate) fn from_raw(ptr: *mut c_void) -> Option<Self> {
        (!ptr.is_null()).then_some(Self { ptr })
    }

    fn info(&self) -> Result<VideoCompositorInfoPayload, AVWriterError> {
        let ptr = unsafe { ffi::av_video_compositor_info_json(self.ptr) };
        parse_json_ptr(ptr, "video compositor info")
    }
}

impl Drop for VideoCompositor {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ffi::av_video_compositor_release(self.ptr) };
        }
    }
}

impl core::fmt::Debug for VideoCompositor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("VideoCompositor")
            .field("ptr", &self.ptr)
            .finish()
    }
}

fn path_arg(path: impl AsRef<Path>, context: &str) -> Result<std::ffi::CString, AVWriterError> {
    let path_str = path
        .as_ref()
        .to_str()
        .ok_or_else(|| AVWriterError::InvalidArgument(format!("{context} is not valid UTF-8")))?;
    cstring_arg(path_str, context)
}

fn missing_payload(context: &str) -> AVWriterError {
    AVWriterError::InvalidState(format!("swift bridge returned no {context} payload"))
}

fn take_required_string(ptr: *mut c_char, context: &str) -> Result<String, AVWriterError> {
    take_swift_string(ptr).ok_or_else(|| missing_payload(context))
}

fn parse_optional_bridge_json<T: for<'de> Deserialize<'de>>(
    ptr: *mut c_char,
    context: &str,
) -> Result<Option<T>, AVWriterError> {
    let Some(raw) = take_swift_string(ptr) else {
        return Ok(None);
    };
    serde_json::from_str(&raw).map(Some).map_err(|e| {
        AVWriterError::InvalidState(format!(
            "failed to decode {context} json from swift bridge: {e}"
        ))
    })
}
