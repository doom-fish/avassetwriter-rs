#![allow(clippy::missing_errors_doc, clippy::needless_pass_by_value)]

use core::ffi::c_void;
use core::ptr;

use crate::PropertyValue;
use apple_cf::cm::CMFormatDescription;

use crate::bridge_support::{string_arg, take_payload, time_kind, time_scale, time_value};
use crate::error::{from_swift, AVWriterError};
use crate::ffi;
use crate::writer::{FileType, VideoPreset};
use apple_cf::cm::CMTime;

#[derive(Debug)]
struct OutputSettingsAssistantInfoPayload {
    audio_settings: Option<PropertyValue>,
    video_settings: Option<PropertyValue>,
    output_file_type: Option<String>,
    source_video_average_frame_duration: CMTime,
    source_video_min_frame_duration: CMTime,
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
        let ptr = unsafe { ffi::av_output_settings_assistant_available_presets_payload() };
        let raw: Vec<String> = take_payload(ptr, "output settings assistant presets")?;
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
        let preset_c = string_arg(preset.as_str(), "output settings preset")?;
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let ptr = unsafe {
            ffi::av_output_settings_assistant_create(preset_c.as_ptr(), &raw mut err_msg)
        };
        if ptr.is_null() {
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        Ok(Self { ptr })
    }

    pub fn audio_settings(&self) -> Result<Option<PropertyValue>, AVWriterError> {
        Ok(self.info()?.audio_settings)
    }

    pub fn video_settings(&self) -> Result<Option<PropertyValue>, AVWriterError> {
        Ok(self.info()?.video_settings)
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
        // SAFETY: the Swift bridge returns a +1 retained CMFormatDescription.
        Ok(unsafe { CMFormatDescription::from_raw(ptr) })
    }

    /// Set or clear the source-audio format hint.
    pub fn set_source_audio_format(
        &self,
        format: Option<&CMFormatDescription>,
    ) -> Result<(), AVWriterError> {
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let status = unsafe {
            ffi::av_output_settings_assistant_set_source_audio_format(
                self.ptr,
                format.map_or(ptr::null_mut(), CMFormatDescription::as_ptr),
                &raw mut err_msg,
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
        // SAFETY: the Swift bridge returns a +1 retained CMFormatDescription.
        Ok(unsafe { CMFormatDescription::from_raw(ptr) })
    }

    /// Set or clear the source-video format hint.
    pub fn set_source_video_format(
        &self,
        format: Option<&CMFormatDescription>,
    ) -> Result<(), AVWriterError> {
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let status = unsafe {
            ffi::av_output_settings_assistant_set_source_video_format(
                self.ptr,
                format.map_or(ptr::null_mut(), CMFormatDescription::as_ptr),
                &raw mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    /// Return the configured source-video average frame duration.
    pub fn source_video_average_frame_duration(&self) -> Result<CMTime, AVWriterError> {
        Ok(self.info()?.source_video_average_frame_duration)
    }

    /// Set the source-video average frame duration used for recommendations.
    pub fn set_source_video_average_frame_duration(
        &self,
        duration: CMTime,
    ) -> Result<(), AVWriterError> {
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let status = unsafe {
            ffi::av_output_settings_assistant_set_source_video_average_frame_duration(
                self.ptr,
                time_value(&duration),
                time_scale(&duration),
                time_kind(&duration),
                &raw mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    /// Return the configured source-video minimum frame duration.
    pub fn source_video_min_frame_duration(&self) -> Result<CMTime, AVWriterError> {
        Ok(self.info()?.source_video_min_frame_duration)
    }

    /// Set the source-video minimum frame duration used for recommendations.
    pub fn set_source_video_min_frame_duration(
        &self,
        duration: CMTime,
    ) -> Result<(), AVWriterError> {
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let status = unsafe {
            ffi::av_output_settings_assistant_set_source_video_min_frame_duration(
                self.ptr,
                time_value(&duration),
                time_scale(&duration),
                time_kind(&duration),
                &raw mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    fn info(&self) -> Result<OutputSettingsAssistantInfoPayload, AVWriterError> {
        let ptr = unsafe { ffi::av_output_settings_assistant_info_payload(self.ptr) };
        take_payload(ptr, "output settings assistant info")
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

impl crate::binary::ToAbi for OutputSettingsAssistantInfoPayload {
    type Abi = crate::ffi::AVWOutputSettingsAssistantInfo;

    fn to_abi(&self, arena: &mut crate::binary::Arena) -> Self::Abi {
        Self::Abi {
            audio_settings: crate::binary::ToAbi::to_abi(&self.audio_settings, arena),
            video_settings: crate::binary::ToAbi::to_abi(&self.video_settings, arena),
            output_file_type: crate::binary::ToAbi::to_abi(&self.output_file_type, arena),
            source_video_average_frame_duration: crate::binary::ToAbi::to_abi(
                &self.source_video_average_frame_duration,
                arena,
            ),
            source_video_min_frame_duration: crate::binary::ToAbi::to_abi(
                &self.source_video_min_frame_duration,
                arena,
            ),
        }
    }
}

impl crate::binary::FromAbi for OutputSettingsAssistantInfoPayload {
    fn from_abi(raw: &Self::Abi) -> Result<Self, crate::error::AVWriterError> {
        Ok(Self {
            audio_settings: crate::binary::FromAbi::from_abi(&raw.audio_settings)?,
            video_settings: crate::binary::FromAbi::from_abi(&raw.video_settings)?,
            output_file_type: crate::binary::FromAbi::from_abi(&raw.output_file_type)?,
            source_video_average_frame_duration: crate::binary::FromAbi::from_abi(
                &raw.source_video_average_frame_duration,
            )?,
            source_video_min_frame_duration: crate::binary::FromAbi::from_abi(
                &raw.source_video_min_frame_duration,
            )?,
        })
    }
}
