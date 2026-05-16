#![allow(clippy::missing_errors_doc, clippy::needless_pass_by_value)]

use core::ffi::{c_char, c_void};
use core::ptr;
use std::path::Path;

use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::bridge_support::{
    cstring_arg, parse_json_ptr, parse_optional_json_string, take_swift_string,
};
use crate::error::{from_swift, AVWriterError};
use crate::ffi;
use crate::time::Time;

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

/// Safe wrapper around `AVMetadataItemFilter`.
pub struct MetadataItemFilter {
    ptr: *mut c_void,
}

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

/// Safe wrapper around `AVAudioMix`.
pub struct AudioMix {
    ptr: *mut c_void,
}

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

/// Safe wrapper around `AVVideoComposition` / `AVMutableVideoComposition`.
pub struct VideoComposition {
    ptr: *mut c_void,
}

impl VideoComposition {
    /// Create a mutable composition seeded from the video tracks in `path`.
    pub fn from_asset(path: impl AsRef<Path>) -> Result<Self, AVWriterError> {
        let path_c = path_arg(path, "video composition asset path")?;
        let ptr = unsafe { ffi::av_video_composition_create_from_asset(path_c.as_ptr()) };
        Self::from_raw(ptr).ok_or_else(|| missing_payload("video composition"))
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

impl VideoCompositor {
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
