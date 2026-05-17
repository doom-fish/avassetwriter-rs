#![allow(clippy::missing_errors_doc, clippy::needless_pass_by_value)]

use core::ffi::{c_char, c_void};
use core::ptr;

use apple_cf::cm::CMFormatDescription;
use serde::Deserialize;
use serde_json::Value as JsonValue;

use crate::bridge_support::{
    cstring_arg, parse_json_ptr, parse_optional_json_string, time_kind, time_scale, time_value,
};
use crate::error::{from_swift, AVWriterError};
use crate::ffi;
use crate::time::Time;
use crate::writer::{FileType, VideoPreset};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OutputSettingsAssistantInfoPayload {
    audio_settings_json: Option<String>,
    video_settings_json: Option<String>,
    output_file_type: Option<String>,
    source_video_average_frame_duration: Time,
    source_video_min_frame_duration: Time,
}

/// Safe wrapper around `AVOutputSettingsAssistant`.
pub struct OutputSettingsAssistant {
    ptr: *mut c_void,
}

// SAFETY: `OutputSettingsAssistant` wraps an ARC-retained pointer.
// ARC retain/release operations are atomic, so the pointer is safe to move
// across threads.  Concurrent shared access (`Sync`) is not implemented because
// `AVOutputSettingsAssistant` is not documented as thread-safe.
unsafe impl Send for OutputSettingsAssistant {}

impl OutputSettingsAssistant {
    /// Return all output-settings presets available on the current macOS SDK/runtime.
    pub fn available_presets() -> Result<Vec<VideoPreset>, AVWriterError> {
        let ptr = unsafe { ffi::av_output_settings_assistant_available_presets_json() };
        let raw: Vec<String> = parse_json_ptr(ptr, "output settings assistant presets")?;
        raw.into_iter()
            .map(|value| {
                VideoPreset::from_raw(&value).ok_or_else(|| {
                    AVWriterError::InvalidState(format!(
                        "swift bridge returned unknown output settings preset '{value}'"
                    ))
                })
            })
            .collect()
    }

    /// Create a new assistant for one of Apple's named `AVOutputSettingsPreset` values.
    pub fn new(preset: VideoPreset) -> Result<Self, AVWriterError> {
        let preset_c = cstring_arg(preset.as_str(), "output settings preset")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let ptr =
            unsafe { ffi::av_output_settings_assistant_create(preset_c.as_ptr(), &mut err_msg) };
        if ptr.is_null() {
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        Ok(Self { ptr })
    }

    /// Return the recommended `AVAudioSettings` dictionary as JSON.
    pub fn audio_settings(&self) -> Result<Option<JsonValue>, AVWriterError> {
        parse_optional_json_string(self.info()?.audio_settings_json)
    }

    /// Return the recommended `AVVideoSettings` dictionary as JSON.
    pub fn video_settings(&self) -> Result<Option<JsonValue>, AVWriterError> {
        parse_optional_json_string(self.info()?.video_settings_json)
    }

    /// Return the output file type recommended by the assistant.
    pub fn output_file_type(&self) -> Result<Option<FileType>, AVWriterError> {
        self.info()?.output_file_type.map_or(Ok(None), |raw| {
            FileType::from_raw(&raw).map(Some).ok_or_else(|| {
                AVWriterError::InvalidState(format!(
                    "swift bridge returned unknown assistant output file type '{raw}'"
                ))
            })
        })
    }

    /// Return the current source-audio format hint, if present.
    pub fn source_audio_format(&self) -> Result<Option<CMFormatDescription>, AVWriterError> {
        let ptr = unsafe { ffi::av_output_settings_assistant_source_audio_format(self.ptr) };
        Ok(CMFormatDescription::from_raw(ptr))
    }

    /// Set or clear the source-audio format hint.
    pub fn set_source_audio_format(
        &self,
        format: Option<&CMFormatDescription>,
    ) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_output_settings_assistant_set_source_audio_format(
                self.ptr,
                format.map_or(ptr::null_mut(), CMFormatDescription::as_ptr),
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    /// Return the current source-video format hint, if present.
    pub fn source_video_format(&self) -> Result<Option<CMFormatDescription>, AVWriterError> {
        let ptr = unsafe { ffi::av_output_settings_assistant_source_video_format(self.ptr) };
        Ok(CMFormatDescription::from_raw(ptr))
    }

    /// Set or clear the source-video format hint.
    pub fn set_source_video_format(
        &self,
        format: Option<&CMFormatDescription>,
    ) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_output_settings_assistant_set_source_video_format(
                self.ptr,
                format.map_or(ptr::null_mut(), CMFormatDescription::as_ptr),
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    /// Return the configured source-video average frame duration.
    pub fn source_video_average_frame_duration(&self) -> Result<Time, AVWriterError> {
        Ok(self.info()?.source_video_average_frame_duration)
    }

    /// Set the source-video average frame duration used for recommendations.
    pub fn set_source_video_average_frame_duration(
        &self,
        duration: impl Into<Time>,
    ) -> Result<(), AVWriterError> {
        let duration = duration.into();
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_output_settings_assistant_set_source_video_average_frame_duration(
                self.ptr,
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

    /// Return the configured source-video minimum frame duration.
    pub fn source_video_min_frame_duration(&self) -> Result<Time, AVWriterError> {
        Ok(self.info()?.source_video_min_frame_duration)
    }

    /// Set the source-video minimum frame duration used for recommendations.
    pub fn set_source_video_min_frame_duration(
        &self,
        duration: impl Into<Time>,
    ) -> Result<(), AVWriterError> {
        let duration = duration.into();
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_output_settings_assistant_set_source_video_min_frame_duration(
                self.ptr,
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

    fn info(&self) -> Result<OutputSettingsAssistantInfoPayload, AVWriterError> {
        let ptr = unsafe { ffi::av_output_settings_assistant_info_json(self.ptr) };
        parse_json_ptr(ptr, "output settings assistant info")
    }
}

impl Drop for OutputSettingsAssistant {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ffi::av_output_settings_assistant_release(self.ptr) };
        }
    }
}

impl core::fmt::Debug for OutputSettingsAssistant {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("OutputSettingsAssistant")
            .field("ptr", &self.ptr)
            .finish()
    }
}
