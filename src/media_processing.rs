#![allow(clippy::missing_errors_doc, clippy::needless_pass_by_value)]

use core::ffi::{c_char, c_void};
use core::ptr;
use std::ops::Deref;
use std::path::Path;
use std::sync::Mutex;

use apple_cf::cm::CMSampleBuffer;
use apple_cf::cv::CVPixelBuffer;
use doom_fish_utils::panic_safe::catch_user_panic;
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
    /// Bundled diagnostic compositor that immediately returns a source frame.
    Passthrough,
    /// Bundled live compositor installed through
    /// [`VideoComposition::install_rust_video_compositor`].
    RustCallback,
}

impl VideoCompositorClass {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Passthrough => "passthrough",
            Self::RustCallback => "rustCallback",
        }
    }

    fn from_class_name(name: &str) -> Option<Self> {
        match name.rsplit('.').next() {
            Some("AVWRPassthroughVideoCompositor") => Some(Self::Passthrough),
            Some("AVWRRustVideoCompositor") => Some(Self::RustCallback),
            _ => None,
        }
    }
}

const REQUEST_SNAPSHOT_WIRE_FORMAT_VERSION: u32 = 1;
static VIDEO_COMPOSITION_SNAPSHOT_TAKE_LOCK: Mutex<()> = Mutex::new(());
static CI_IMAGE_FILTERING_SNAPSHOT_TAKE_LOCK: Mutex<()> = Mutex::new(());

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
    #[serde(default)]
    wire_format_version: u32,
    #[serde(default)]
    generation: u64,
    composition_time: Time,
    #[serde(rename = "sourceTrackIDs", alias = "sourceTrackIds")]
    source_track_ids: Vec<i32>,
    #[serde(
        rename = "sourceSampleDataTrackIDs",
        alias = "sourceSampleDataTrackIds"
    )]
    source_sample_data_track_ids: Vec<i32>,
    render_size: SizePayload,
    render_scale: f32,
    video_composition_instruction_class_name: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AsynchronousCIImageFilteringRequestPayload {
    #[serde(default)]
    wire_format_version: u32,
    #[serde(default)]
    generation: u64,
    composition_time: Time,
    render_size: SizePayload,
    source_image_extent: RectPayload,
}

trait VersionedRequestSnapshot {
    fn wire_format_version(&self) -> u32;
    fn generation(&self) -> u64;
}

impl VersionedRequestSnapshot for AsynchronousVideoCompositionRequestPayload {
    fn wire_format_version(&self) -> u32 {
        self.wire_format_version
    }

    fn generation(&self) -> u64 {
        self.generation
    }
}

impl VersionedRequestSnapshot for AsynchronousCIImageFilteringRequestPayload {
    fn wire_format_version(&self) -> u32 {
        self.wire_format_version
    }

    fn generation(&self) -> u64 {
        self.generation
    }
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
    #[serde(rename = "sourceTrackIDs", alias = "sourceTrackIds")]
    pub source_track_ids: Vec<i32>,
    #[serde(
        rename = "sourceSampleDataTrackIDs",
        alias = "sourceSampleDataTrackIds"
    )]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Operations supported by the Rust callback compositor.
#[allow(clippy::struct_excessive_bools)]
pub struct RustVideoCompositorCapabilities {
    /// Requests can return retained source `CVPixelBuffer` values.
    pub source_pixel_buffers: bool,
    /// Requests can return retained source `CMSampleBuffer` values.
    pub source_sample_buffers: bool,
    /// Requests can allocate an output buffer from the native render context.
    pub output_pixel_buffer_allocation: bool,
    /// Tagged-buffer composition is supported.
    pub tagged_buffers: bool,
    /// Wide-color source frames are supported.
    pub wide_color_source_frames: bool,
    /// HDR source frames are supported.
    pub hdr_source_frames: bool,
    /// Rust handlers can process `CIImage` values directly.
    pub ci_images: bool,
}

impl RustVideoCompositorCapabilities {
    pub const CURRENT: Self = Self {
        source_pixel_buffers: true,
        source_sample_buffers: true,
        output_pixel_buffer_allocation: true,
        tagged_buffers: false,
        wide_color_source_frames: false,
        hdr_source_frames: false,
        ci_images: false,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum RequestCompletionState {
    #[default]
    Pending,
    TerminalActionStarted,
}

impl RequestCompletionState {
    fn begin_terminal_action(&mut self) -> bool {
        if *self != Self::Pending {
            return false;
        }
        *self = Self::TerminalActionStarted;
        true
    }

    fn begin_drop_cancel(&mut self) -> bool {
        self.begin_terminal_action()
    }
}

/// Retained native request passed to a Rust custom video-compositor handler.
///
/// Dropping it before a terminal method is called cancels the request.
pub struct VideoCompositionRequest {
    ptr: *mut c_void,
    completion_state: RequestCompletionState,
}

// SAFETY: the native request is retained for this wrapper's lifetime and the
// custom compositor contract permits asynchronous completion from another queue.
unsafe impl Send for VideoCompositionRequest {}

impl VideoCompositionRequest {
    /// Return the requested composition time.
    pub fn composition_time(&self) -> Result<Time, AVWriterError> {
        let mut value = 0_i64;
        let mut scale = 0_i32;
        let mut kind = 0_i32;
        unsafe {
            ffi::av_video_composition_request_composition_time(
                self.ptr,
                &raw mut value,
                &raw mut scale,
                &raw mut kind,
            );
        }
        time_from_ffi_parts(value, scale, kind)
    }

    /// Return the native render-context size.
    #[must_use]
    pub fn render_size(&self) -> (f64, f64) {
        let mut width = 0.0;
        let mut height = 0.0;
        let mut scale = 0.0;
        unsafe {
            ffi::av_video_composition_request_render_info(
                self.ptr,
                &raw mut width,
                &raw mut height,
                &raw mut scale,
            );
        }
        (width, height)
    }

    /// Return the native render scale.
    #[must_use]
    pub fn render_scale(&self) -> f32 {
        let mut width = 0.0;
        let mut height = 0.0;
        let mut scale = 0.0;
        unsafe {
            ffi::av_video_composition_request_render_info(
                self.ptr,
                &raw mut width,
                &raw mut height,
                &raw mut scale,
            );
        }
        scale
    }

    /// Return track IDs whose source pixel buffers may be requested.
    pub fn source_track_ids(&self) -> Result<Vec<i32>, AVWriterError> {
        let count = unsafe { ffi::av_video_composition_request_source_track_id_count(self.ptr) };
        let mut track_ids = Vec::with_capacity(count);
        for index in 0..count {
            let mut track_id = 0;
            let found = unsafe {
                ffi::av_video_composition_request_source_track_id_at(
                    self.ptr,
                    index,
                    &raw mut track_id,
                )
            };
            if !found {
                return Err(AVWriterError::InvalidState(
                    "video composition request source track list changed unexpectedly".into(),
                ));
            }
            track_ids.push(track_id);
        }
        Ok(track_ids)
    }

    /// Return track IDs whose source sample buffers may be requested.
    pub fn source_sample_data_track_ids(&self) -> Result<Vec<i32>, AVWriterError> {
        let count = unsafe {
            ffi::av_video_composition_request_source_sample_data_track_id_count(self.ptr)
        };
        let mut track_ids = Vec::with_capacity(count);
        for index in 0..count {
            let mut track_id = 0;
            let found = unsafe {
                ffi::av_video_composition_request_source_sample_data_track_id_at(
                    self.ptr,
                    index,
                    &raw mut track_id,
                )
            };
            if !found {
                return Err(AVWriterError::InvalidState(
                    "video composition request sample-data track list changed unexpectedly".into(),
                ));
            }
            track_ids.push(track_id);
        }
        Ok(track_ids)
    }

    /// Copy a retained source pixel buffer for `track_id`.
    pub fn source_frame(&self, track_id: i32) -> Result<Option<CVPixelBuffer>, AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let frame = unsafe {
            ffi::av_video_composition_request_copy_source_frame(
                self.ptr,
                track_id,
                &raw mut err_msg,
            )
        };
        if frame.is_null() {
            return if err_msg.is_null() {
                Ok(None)
            } else {
                Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) })
            };
        }
        // SAFETY: the bridge transfers a +1 retained CVPixelBuffer.
        unsafe { CVPixelBuffer::from_raw(frame) }
            .map(Some)
            .ok_or_else(|| missing_payload("video composition source pixel buffer"))
    }

    /// Copy a retained source sample buffer for `track_id`.
    pub fn source_sample_buffer(
        &self,
        track_id: i32,
    ) -> Result<Option<CMSampleBuffer>, AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let sample = unsafe {
            ffi::av_video_composition_request_copy_source_sample_buffer(
                self.ptr,
                track_id,
                &raw mut err_msg,
            )
        };
        if sample.is_null() {
            return if err_msg.is_null() {
                Ok(None)
            } else {
                Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) })
            };
        }
        // SAFETY: the bridge transfers a +1 retained CMSampleBuffer.
        unsafe { CMSampleBuffer::from_raw(sample) }
            .map(Some)
            .ok_or_else(|| missing_payload("video composition source sample buffer"))
    }

    /// Allocate a retained output pixel buffer from the native render context.
    pub fn new_output_pixel_buffer(&self) -> Result<CVPixelBuffer, AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let frame = unsafe {
            ffi::av_video_composition_request_new_output_pixel_buffer(self.ptr, &raw mut err_msg)
        };
        if frame.is_null() {
            return Err(if err_msg.is_null() {
                missing_payload("video composition output pixel buffer")
            } else {
                unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) }
            });
        }
        // SAFETY: the bridge transfers a +1 retained CVPixelBuffer.
        unsafe { CVPixelBuffer::from_raw(frame) }
            .ok_or_else(|| missing_payload("video composition output pixel buffer"))
    }

    /// Complete this request with a composed pixel buffer.
    pub fn finish_with_frame(mut self, frame: &CVPixelBuffer) -> Result<(), AVWriterError> {
        self.begin_terminal_action()?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_video_composition_request_finish_with_frame(
                self.ptr,
                frame.as_ptr(),
                &raw mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    /// Complete this request with an error reported to the export operation.
    pub fn finish_with_error(mut self, message: &str) -> Result<(), AVWriterError> {
        let message_c = cstring_arg(message, "video composition request error")?;
        self.begin_terminal_action()?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_video_composition_request_finish_with_error(
                self.ptr,
                message_c.as_ptr(),
                &raw mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    /// Cancel this request.
    pub fn cancel(mut self) -> Result<(), AVWriterError> {
        self.begin_terminal_action()?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status =
            unsafe { ffi::av_video_composition_request_cancel(self.ptr, &raw mut err_msg) };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    fn begin_terminal_action(&mut self) -> Result<(), AVWriterError> {
        if self.completion_state.begin_terminal_action() {
            Ok(())
        } else {
            Err(AVWriterError::InvalidState(
                "video composition request completion was already attempted".into(),
            ))
        }
    }

    fn from_raw(ptr: *mut c_void) -> Option<Self> {
        (!ptr.is_null()).then_some(Self {
            ptr,
            completion_state: RequestCompletionState::Pending,
        })
    }
}

impl Drop for VideoCompositionRequest {
    fn drop(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        if self.completion_state.begin_drop_cancel() {
            let mut err_msg: *mut c_char = ptr::null_mut();
            unsafe {
                ffi::av_video_composition_request_cancel(self.ptr, &raw mut err_msg);
            }
            let _ = take_swift_string(err_msg);
        }
        unsafe { ffi::av_video_composition_request_release(self.ptr) };
        self.ptr = ptr::null_mut();
    }
}

impl core::fmt::Debug for VideoCompositionRequest {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("VideoCompositionRequest")
            .field("ptr", &self.ptr)
            .field("completion_state", &self.completion_state)
            .finish()
    }
}

struct VideoCompositionHandlerState {
    callback: Box<dyn Fn(VideoCompositionRequest) + Send + Sync + 'static>,
}

unsafe extern "C" fn video_composition_request_trampoline(
    request: *mut c_void,
    userdata: *mut c_void,
) {
    let Some(request) = VideoCompositionRequest::from_raw(request) else {
        return;
    };
    if userdata.is_null() {
        drop(request);
        return;
    }
    catch_user_panic("video_composition_request_trampoline", move || unsafe {
        let state = &*userdata.cast::<VideoCompositionHandlerState>();
        (state.callback)(request);
    });
}

unsafe extern "C" fn video_composition_handler_drop(userdata: *mut c_void) {
    if userdata.is_null() {
        return;
    }
    catch_user_panic("video_composition_handler_drop", || unsafe {
        drop(Box::from_raw(
            userdata.cast::<VideoCompositionHandlerState>(),
        ));
    });
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
                &raw mut err_msg,
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
                &raw mut err_msg,
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
                &raw mut err_msg,
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
                &raw mut err_msg,
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
                &raw mut err_msg,
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
                &raw mut err_msg,
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

    /// Take the most recently recorded CI image filtering request snapshot.
    ///
    /// Calls are serialized across the process so one generation is returned
    /// to at most one Rust caller. Transfer or decode failures leave it
    /// available for a later retry.
    pub fn take_latest_ci_image_filtering_request(
    ) -> Result<Option<AsynchronousCIImageFilteringRequest>, AVWriterError> {
        let payload: Option<AsynchronousCIImageFilteringRequestPayload> = take_latest_snapshot(
            &CI_IMAGE_FILTERING_SNAPSHOT_TAKE_LOCK,
            "ci image filtering request",
            || {
                copy_snapshot_json("ci image filtering request", |generation| unsafe {
                    ffi::av_copy_latest_ci_image_filtering_request_json(generation)
                })
            },
            |generation| unsafe { ffi::av_ack_latest_ci_image_filtering_request(generation) },
        )?;
        let Some(payload) = payload else {
            return Ok(None);
        };
        let snapshot = AsynchronousCIImageFilteringRequest {
            composition_time: payload.composition_time,
            render_size: (payload.render_size.width, payload.render_size.height),
            source_image_extent: ImageRect {
                x: payload.source_image_extent.x,
                y: payload.source_image_extent.y,
                width: payload.source_image_extent.width,
                height: payload.source_image_extent.height,
            },
        };
        Ok(Some(snapshot))
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
        if class == Some(VideoCompositorClass::RustCallback) {
            return Err(AVWriterError::InvalidArgument(
                "the Rust callback compositor must be installed with a request handler".into(),
            ));
        }
        let class_c = class
            .map(|value| cstring_arg(value.as_str(), "video compositor class"))
            .transpose()?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_video_composition_set_custom_video_compositor_class(
                self.ptr,
                class_c.as_ref().map_or(ptr::null(), |value| value.as_ptr()),
                &raw mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    /// Install a retained Rust request handler and route every instruction through it.
    ///
    /// `AVFoundation` may invoke the handler concurrently. Each request can be
    /// completed synchronously or moved to a worker thread. Dropping a pending
    /// request cancels it exactly once. The current implementation accepts
    /// 32-bit BGRA SDR frames; query [`Self::rust_video_compositor_capabilities`]
    /// for explicit unsupported paths.
    pub fn install_rust_video_compositor<F>(&self, handler: F) -> Result<(), AVWriterError>
    where
        F: Fn(VideoCompositionRequest) + Send + Sync + 'static,
    {
        let state = Box::new(VideoCompositionHandlerState {
            callback: Box::new(handler),
        });
        let userdata = Box::into_raw(state).cast::<c_void>();
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_video_composition_install_rust_compositor(
                self.ptr,
                Some(video_composition_request_trampoline),
                userdata,
                Some(video_composition_handler_drop),
                &raw mut err_msg,
            )
        };
        if status != ffi::status::OK {
            unsafe { video_composition_handler_drop(userdata) };
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    /// Restore the original instructions.
    ///
    /// Requests already handed to Rust retain their handler until they finish,
    /// fail, or are cancelled.
    pub fn clear_rust_video_compositor(&self) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status =
            unsafe { ffi::av_video_composition_clear_rust_compositor(self.ptr, &raw mut err_msg) };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    #[must_use]
    /// Return the capabilities of the bundled Rust callback compositor.
    pub const fn rust_video_compositor_capabilities() -> RustVideoCompositorCapabilities {
        RustVideoCompositorCapabilities::CURRENT
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
    /// Take the most recently recorded video composition request snapshot.
    ///
    /// The copy/decode/ack transaction is serialized across the process, so a
    /// generation is returned to at most one Rust caller. Failed transfer or
    /// decoding leaves the same generation available for recovery; a lost
    /// acknowledgment is reported instead of returning a duplicate.
    pub fn take_latest_video_composition_request(
    ) -> Result<Option<AsynchronousVideoCompositionRequest>, AVWriterError> {
        let payload: Option<AsynchronousVideoCompositionRequestPayload> = take_latest_snapshot(
            &VIDEO_COMPOSITION_SNAPSHOT_TAKE_LOCK,
            "video composition request",
            || {
                copy_snapshot_json("video composition request", |generation| unsafe {
                    ffi::av_copy_latest_video_composition_request_json(generation)
                })
            },
            |generation| unsafe { ffi::av_ack_latest_video_composition_request(generation) },
        )?;
        let Some(payload) = payload else {
            return Ok(None);
        };
        let snapshot = AsynchronousVideoCompositionRequest {
            composition_time: payload.composition_time,
            source_track_ids: payload.source_track_ids,
            source_sample_data_track_ids: payload.source_sample_data_track_ids,
            render_size: (payload.render_size.width, payload.render_size.height),
            render_scale: payload.render_scale,
            video_composition_instruction_class_name: payload
                .video_composition_instruction_class_name,
        };
        Ok(Some(snapshot))
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

fn copy_snapshot_json(
    context: &str,
    copy: impl FnOnce(&mut u64) -> *mut c_char,
) -> Result<Option<(u64, String)>, AVWriterError> {
    let mut generation = 0;
    let ptr = copy(&mut generation);
    let Some(raw) = take_swift_string(ptr) else {
        return if generation == 0 {
            Ok(None)
        } else {
            Err(AVWriterError::InvalidState(format!(
                "swift bridge could not transfer {context} generation {generation}"
            )))
        };
    };
    Ok(Some((generation, raw)))
}

fn take_latest_snapshot<T>(
    take_lock: &Mutex<()>,
    context: &str,
    copy: impl FnOnce() -> Result<Option<(u64, String)>, AVWriterError>,
    acknowledge: impl FnOnce(u64) -> bool,
) -> Result<Option<T>, AVWriterError>
where
    T: for<'de> Deserialize<'de> + VersionedRequestSnapshot,
{
    let _take_guard = match take_lock.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    let Some((generation, raw)) = copy()? else {
        return Ok(None);
    };
    let payload: T = serde_json::from_str(&raw).map_err(|e| {
        AVWriterError::InvalidState(format!(
            "failed to decode {context} json from swift bridge: {e}"
        ))
    })?;
    validate_snapshot_header(
        payload.wire_format_version(),
        payload.generation(),
        generation,
        context,
    )?;
    if !acknowledge(generation) {
        return Err(AVWriterError::InvalidState(format!(
            "{context} generation {generation} was consumed before acknowledgment"
        )));
    }
    Ok(Some(payload))
}

fn validate_snapshot_header(
    wire_format_version: u32,
    payload_generation: u64,
    copied_generation: u64,
    context: &str,
) -> Result<(), AVWriterError> {
    if wire_format_version != REQUEST_SNAPSHOT_WIRE_FORMAT_VERSION {
        return Err(AVWriterError::InvalidState(format!(
            "unsupported {context} wire format version {wire_format_version}"
        )));
    }
    if payload_generation != copied_generation {
        return Err(AVWriterError::InvalidState(format!(
            "{context} generation mismatch: copied {copied_generation}, payload {payload_generation}"
        )));
    }
    Ok(())
}

fn time_from_ffi_parts(value: i64, scale: i32, kind: i32) -> Result<Time, AVWriterError> {
    match kind {
        0 => Ok(Time::new(value, scale)),
        1 => Ok(Time::invalid()),
        2 => Ok(Time::indefinite()),
        3 => Ok(Time::positive_infinity()),
        4 => Ok(Time::negative_infinity()),
        other => Err(AVWriterError::InvalidState(format!(
            "swift bridge returned unknown time kind {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Barrier};

    const SNAPSHOT_BODY: &str = r#"
        "compositionTime":{"kind":"numeric","value":3,"timescale":30},
        "renderSize":{"width":1920.0,"height":1080.0},
        "renderScale":1.0,
        "videoCompositionInstructionClassName":"AVWRRustVideoCompositionInstruction"
    "#;

    #[derive(Default)]
    struct MockSnapshotStore {
        entry: Mutex<Option<(u64, String)>>,
        acknowledgments: AtomicUsize,
    }

    impl MockSnapshotStore {
        fn with_entry(generation: u64, json: String) -> Self {
            Self {
                entry: Mutex::new(Some((generation, json))),
                acknowledgments: AtomicUsize::new(0),
            }
        }

        fn copy(&self) -> Option<(u64, String)> {
            self.entry
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone()
        }

        fn acknowledge(&self, generation: u64) -> bool {
            let mut entry = self
                .entry
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if entry.as_ref().map(|value| value.0) != Some(generation) {
                return false;
            }
            *entry = None;
            drop(entry);
            self.acknowledgments.fetch_add(1, Ordering::SeqCst);
            true
        }
    }

    fn video_snapshot_json(generation: u64) -> String {
        format!(
            r#"{{"wireFormatVersion":1,"generation":{generation},"sourceTrackIDs":[2],"sourceSampleDataTrackIDs":[9],{SNAPSHOT_BODY}}}"#
        )
    }

    fn ci_snapshot_json(generation: u64) -> String {
        format!(
            r#"{{"wireFormatVersion":1,"generation":{generation},"compositionTime":{{"kind":"numeric","value":3,"timescale":30}},"renderSize":{{"width":1920.0,"height":1080.0}},"sourceImageExtent":{{"x":0.0,"y":0.0,"width":1920.0,"height":1080.0}}}}"#
        )
    }

    #[test]
    fn video_request_wire_uses_exact_id_keys_and_version() {
        let json = format!(
            r#"{{"wireFormatVersion":1,"generation":7,"sourceTrackIDs":[2],"sourceSampleDataTrackIDs":[9],{SNAPSHOT_BODY}}}"#
        );
        let payload: AsynchronousVideoCompositionRequestPayload =
            serde_json::from_str(&json).unwrap();
        validate_snapshot_header(
            payload.wire_format_version,
            payload.generation,
            7,
            "video composition request",
        )
        .unwrap();
        assert_eq!(payload.source_track_ids, vec![2]);
        assert_eq!(payload.source_sample_data_track_ids, vec![9]);
    }

    #[test]
    fn video_request_wire_accepts_legacy_id_aliases() {
        let json = format!(
            r#"{{"wireFormatVersion":1,"generation":8,"sourceTrackIds":[4],"sourceSampleDataTrackIds":[6],{SNAPSHOT_BODY}}}"#
        );
        let payload: AsynchronousVideoCompositionRequestPayload =
            serde_json::from_str(&json).unwrap();
        assert_eq!(payload.source_track_ids, vec![4]);
        assert_eq!(payload.source_sample_data_track_ids, vec![6]);
    }

    #[test]
    fn public_snapshot_serializes_exact_id_keys() {
        let snapshot = AsynchronousVideoCompositionRequest {
            composition_time: Time::new(0, 30),
            source_track_ids: vec![1],
            source_sample_data_track_ids: vec![2],
            render_size: (640.0, 480.0),
            render_scale: 1.0,
            video_composition_instruction_class_name: None,
        };
        let value = serde_json::to_value(snapshot).unwrap();
        assert_eq!(value["sourceTrackIDs"], serde_json::json!([1]));
        assert_eq!(value["sourceSampleDataTrackIDs"], serde_json::json!([2]));
        assert!(value.get("sourceTrackIds").is_none());
        assert!(value.get("sourceSampleDataTrackIds").is_none());
    }

    #[test]
    fn snapshot_header_rejects_unknown_version() {
        let error = validate_snapshot_header(99, 5, 5, "video composition request").unwrap_err();
        assert!(error.to_string().contains("wire format version 99"));
    }

    #[test]
    fn snapshot_header_rejects_generation_mismatch() {
        let error = validate_snapshot_header(1, 4, 5, "video composition request").unwrap_err();
        assert!(error.to_string().contains("generation mismatch"));
    }

    #[test]
    fn snapshot_transfer_failure_keeps_generation_recoverable() {
        let result = copy_snapshot_json("video composition request", |generation| {
            *generation = 12;
            ptr::null_mut()
        });
        let error = result.unwrap_err();
        assert!(error.to_string().contains("generation 12"));
    }

    #[test]
    fn video_snapshot_take_is_single_consumer() {
        let store = Arc::new(MockSnapshotStore::with_entry(21, video_snapshot_json(21)));
        let start = Arc::new(Barrier::new(3));
        let mut threads = Vec::new();
        for _ in 0..2 {
            let store = Arc::clone(&store);
            let start = Arc::clone(&start);
            threads.push(std::thread::spawn(move || {
                start.wait();
                take_latest_snapshot::<AsynchronousVideoCompositionRequestPayload>(
                    &VIDEO_COMPOSITION_SNAPSHOT_TAKE_LOCK,
                    "video composition request",
                    || Ok(store.copy()),
                    |generation| store.acknowledge(generation),
                )
            }));
        }
        start.wait();
        let results = threads
            .into_iter()
            .map(|thread| thread.join().expect("video take thread").unwrap())
            .collect::<Vec<_>>();
        assert_eq!(results.iter().filter(|result| result.is_some()).count(), 1);
        assert_eq!(results.iter().filter(|result| result.is_none()).count(), 1);
        assert_eq!(store.acknowledgments.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn ci_snapshot_take_is_single_consumer() {
        let store = Arc::new(MockSnapshotStore::with_entry(22, ci_snapshot_json(22)));
        let start = Arc::new(Barrier::new(3));
        let mut threads = Vec::new();
        for _ in 0..2 {
            let store = Arc::clone(&store);
            let start = Arc::clone(&start);
            threads.push(std::thread::spawn(move || {
                start.wait();
                take_latest_snapshot::<AsynchronousCIImageFilteringRequestPayload>(
                    &CI_IMAGE_FILTERING_SNAPSHOT_TAKE_LOCK,
                    "ci image filtering request",
                    || Ok(store.copy()),
                    |generation| store.acknowledge(generation),
                )
            }));
        }
        start.wait();
        let results = threads
            .into_iter()
            .map(|thread| thread.join().expect("ci take thread").unwrap())
            .collect::<Vec<_>>();
        assert_eq!(results.iter().filter(|result| result.is_some()).count(), 1);
        assert_eq!(results.iter().filter(|result| result.is_none()).count(), 1);
        assert_eq!(store.acknowledgments.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn snapshot_decode_failure_preserves_both_snapshot_types() {
        let video_lock = Mutex::new(());
        let video_store = MockSnapshotStore::with_entry(31, video_snapshot_json(31));
        let video_error = take_latest_snapshot::<AsynchronousVideoCompositionRequestPayload>(
            &video_lock,
            "video composition request",
            || {
                Ok(video_store
                    .copy()
                    .map(|(generation, _)| (generation, "{".into())))
            },
            |generation| video_store.acknowledge(generation),
        )
        .unwrap_err();
        assert!(video_error.to_string().contains("failed to decode"));
        assert!(video_store.copy().is_some());
        assert_eq!(video_store.acknowledgments.load(Ordering::SeqCst), 0);
        assert!(
            take_latest_snapshot::<AsynchronousVideoCompositionRequestPayload>(
                &video_lock,
                "video composition request",
                || Ok(video_store.copy()),
                |generation| video_store.acknowledge(generation),
            )
            .unwrap()
            .is_some()
        );

        let ci_lock = Mutex::new(());
        let ci_store = MockSnapshotStore::with_entry(32, ci_snapshot_json(32));
        let ci_error = take_latest_snapshot::<AsynchronousCIImageFilteringRequestPayload>(
            &ci_lock,
            "ci image filtering request",
            || {
                Ok(ci_store
                    .copy()
                    .map(|(generation, _)| (generation, "[".into())))
            },
            |generation| ci_store.acknowledge(generation),
        )
        .unwrap_err();
        assert!(ci_error.to_string().contains("failed to decode"));
        assert!(ci_store.copy().is_some());
        assert_eq!(ci_store.acknowledgments.load(Ordering::SeqCst), 0);
        assert!(
            take_latest_snapshot::<AsynchronousCIImageFilteringRequestPayload>(
                &ci_lock,
                "ci image filtering request",
                || Ok(ci_store.copy()),
                |generation| ci_store.acknowledge(generation),
            )
            .unwrap()
            .is_some()
        );
    }

    #[test]
    fn lost_snapshot_acknowledgment_is_not_returned() {
        let take_lock = Mutex::new(());
        let store = MockSnapshotStore::with_entry(41, video_snapshot_json(41));
        let error = take_latest_snapshot::<AsynchronousVideoCompositionRequestPayload>(
            &take_lock,
            "video composition request",
            || Ok(store.copy()),
            |_| false,
        )
        .unwrap_err();
        assert!(error.to_string().contains("consumed before acknowledgment"));
    }

    #[test]
    fn request_completion_state_cancels_only_while_pending() {
        let mut state = RequestCompletionState::Pending;
        assert!(state.begin_drop_cancel());
        assert!(!state.begin_drop_cancel());

        let mut completed = RequestCompletionState::Pending;
        assert!(completed.begin_terminal_action());
        assert!(!completed.begin_drop_cancel());
    }

    #[test]
    fn rust_compositor_capabilities_are_explicit() {
        let capabilities = RustVideoCompositorCapabilities::CURRENT;
        assert!(capabilities.source_pixel_buffers);
        assert!(capabilities.source_sample_buffers);
        assert!(capabilities.output_pixel_buffer_allocation);
        assert!(!capabilities.tagged_buffers);
        assert!(!capabilities.wide_color_source_frames);
        assert!(!capabilities.hdr_source_frames);
        assert!(!capabilities.ci_images);
    }

    #[test]
    fn rust_callback_compositor_class_is_distinct() {
        assert_eq!(
            VideoCompositorClass::from_class_name("AVAssetWriterBridge.AVWRRustVideoCompositor"),
            Some(VideoCompositorClass::RustCallback)
        );
        assert_ne!(
            VideoCompositorClass::RustCallback,
            VideoCompositorClass::Passthrough
        );
    }
}
