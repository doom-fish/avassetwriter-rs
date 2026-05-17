#![allow(clippy::missing_errors_doc, clippy::needless_pass_by_value)]

use core::ffi::{c_char, c_void};
use core::ptr;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::bridge_support::{
    cstring_arg, parse_json_ptr, serialize_json, time_kind, time_scale, time_value,
};
use crate::error::{from_swift, AVWriterError};
use crate::ffi;
use crate::media_processing::{AudioMix, MetadataItemFilter, VideoComposition, VideoCompositor};
use crate::metadata::MetadataItem;
use crate::time::{Time, TimeRange};
use crate::writer::FileType;

/// `AVAssetExportSession` preset identifiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ExportPreset {
    LowQuality,
    MediumQuality,
    HighestQuality,
    HevcHighestQuality,
    HevcHighestQualityWithAlpha,
    P640x480,
    P960x540,
    P1280x720,
    P1920x1080,
    P3840x2160,
    Hevc1920x1080,
    Hevc1920x1080WithAlpha,
    Hevc3840x2160,
    Hevc3840x2160WithAlpha,
    Hevc4320x2160,
    Hevc7680x4320,
    MvHevc960x960,
    MvHevc1440x1440,
    MvHevc4320x4320,
    MvHevc7680x7680,
    AppleM4A,
    Passthrough,
    AppleProRes422Lpcm,
    AppleProRes4444Lpcm,
    AppleM4vCellular,
    AppleM4viPod,
    AppleM4v480pSd,
    AppleM4vAppleTv,
    AppleM4vWiFi,
    AppleM4v720pHd,
    AppleM4v1080pHd,
}

impl ExportPreset {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::LowQuality => "low_quality",
            Self::MediumQuality => "medium_quality",
            Self::HighestQuality => "highest_quality",
            Self::HevcHighestQuality => "hevc_highest_quality",
            Self::HevcHighestQualityWithAlpha => "hevc_highest_quality_with_alpha",
            Self::P640x480 => "640x480",
            Self::P960x540 => "960x540",
            Self::P1280x720 => "1280x720",
            Self::P1920x1080 => "1920x1080",
            Self::P3840x2160 => "3840x2160",
            Self::Hevc1920x1080 => "hevc_1920x1080",
            Self::Hevc1920x1080WithAlpha => "hevc_1920x1080_with_alpha",
            Self::Hevc3840x2160 => "hevc_3840x2160",
            Self::Hevc3840x2160WithAlpha => "hevc_3840x2160_with_alpha",
            Self::Hevc4320x2160 => "hevc_4320x2160",
            Self::Hevc7680x4320 => "hevc_7680x4320",
            Self::MvHevc960x960 => "mvhevc_960x960",
            Self::MvHevc1440x1440 => "mvhevc_1440x1440",
            Self::MvHevc4320x4320 => "mvhevc_4320x4320",
            Self::MvHevc7680x7680 => "mvhevc_7680x7680",
            Self::AppleM4A => "apple_m4a",
            Self::Passthrough => "passthrough",
            Self::AppleProRes422Lpcm => "apple_prores_422_lpcm",
            Self::AppleProRes4444Lpcm => "apple_prores_4444_lpcm",
            Self::AppleM4vCellular => "apple_m4v_cellular",
            Self::AppleM4viPod => "apple_m4v_ipod",
            Self::AppleM4v480pSd => "apple_m4v_480p_sd",
            Self::AppleM4vAppleTv => "apple_m4v_apple_tv",
            Self::AppleM4vWiFi => "apple_m4v_wifi",
            Self::AppleM4v720pHd => "apple_m4v_720p_hd",
            Self::AppleM4v1080pHd => "apple_m4v_1080p_hd",
        }
    }

    pub(crate) fn from_raw(raw: &str) -> Option<Self> {
        Some(match raw {
            "low_quality" => Self::LowQuality,
            "medium_quality" => Self::MediumQuality,
            "highest_quality" => Self::HighestQuality,
            "hevc_highest_quality" => Self::HevcHighestQuality,
            "hevc_highest_quality_with_alpha" => Self::HevcHighestQualityWithAlpha,
            "640x480" => Self::P640x480,
            "960x540" => Self::P960x540,
            "1280x720" => Self::P1280x720,
            "1920x1080" => Self::P1920x1080,
            "3840x2160" => Self::P3840x2160,
            "hevc_1920x1080" => Self::Hevc1920x1080,
            "hevc_1920x1080_with_alpha" => Self::Hevc1920x1080WithAlpha,
            "hevc_3840x2160" => Self::Hevc3840x2160,
            "hevc_3840x2160_with_alpha" => Self::Hevc3840x2160WithAlpha,
            "hevc_4320x2160" => Self::Hevc4320x2160,
            "hevc_7680x4320" => Self::Hevc7680x4320,
            "mvhevc_960x960" => Self::MvHevc960x960,
            "mvhevc_1440x1440" => Self::MvHevc1440x1440,
            "mvhevc_4320x4320" => Self::MvHevc4320x4320,
            "mvhevc_7680x7680" => Self::MvHevc7680x7680,
            "apple_m4a" => Self::AppleM4A,
            "passthrough" => Self::Passthrough,
            "apple_prores_422_lpcm" => Self::AppleProRes422Lpcm,
            "apple_prores_4444_lpcm" => Self::AppleProRes4444Lpcm,
            "apple_m4v_cellular" => Self::AppleM4vCellular,
            "apple_m4v_ipod" => Self::AppleM4viPod,
            "apple_m4v_480p_sd" => Self::AppleM4v480pSd,
            "apple_m4v_apple_tv" => Self::AppleM4vAppleTv,
            "apple_m4v_wifi" => Self::AppleM4vWiFi,
            "apple_m4v_720p_hd" => Self::AppleM4v720pHd,
            "apple_m4v_1080p_hd" => Self::AppleM4v1080pHd,
            _ => return None,
        })
    }
}

/// `AVAssetExportSessionStatus`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ExportStatus {
    Unknown,
    Waiting,
    Exporting,
    Completed,
    Failed,
    Cancelled,
    Other(i32),
}

impl ExportStatus {
    #[must_use]
    pub const fn from_raw(raw: i32) -> Self {
        match raw {
            0 => Self::Unknown,
            1 => Self::Waiting,
            2 => Self::Exporting,
            3 => Self::Completed,
            4 => Self::Failed,
            5 => Self::Cancelled,
            other => Self::Other(other),
        }
    }
}

/// `AVAssetTrackGroupOutputHandling`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TrackGroupOutputHandling {
    None,
    PreserveAlternateTracks,
    Other(u64),
}

impl TrackGroupOutputHandling {
    pub(crate) const fn from_raw(raw: u64) -> Self {
        match raw {
            0 => Self::None,
            1 => Self::PreserveAlternateTracks,
            other => Self::Other(other),
        }
    }

    pub(crate) const fn raw(self) -> u64 {
        match self {
            Self::None => 0,
            Self::PreserveAlternateTracks => 1,
            Self::Other(raw) => raw,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportSessionInfoPayload {
    preset_name: String,
    asset_path: Option<String>,
    output_file_type: Option<String>,
    output_path: Option<String>,
    should_optimize_for_network_use: bool,
    allows_parallelized_export: bool,
    status: i32,
    error_message: Option<String>,
    progress: f32,
    supported_file_types: Vec<String>,
    time_range: TimeRange,
    file_length_limit: i64,
    metadata: Vec<MetadataItem>,
    can_perform_multiple_passes_over_source_media_data: bool,
    directory_for_temporary_files: Option<String>,
    audio_track_group_handling: u64,
}

/// Safe wrapper around `AVAssetExportSession`, backed by an internal `AVURLAsset`.
pub struct ExportSession {
    ptr: *mut c_void,
}

// SAFETY: `ExportSession` wraps an ARC-retained `AVAssetExportSession` pointer.
// ARC retain/release operations are atomic, so the pointer is safe to move
// across threads.  Concurrent shared access (`Sync`) is not implemented because
// `AVAssetExportSession` is not documented as thread-safe for simultaneous
// method calls.
unsafe impl Send for ExportSession {}

impl ExportSession {
    /// Returns the raw Swift object pointer (for use by `async_api`).
    pub(crate) const fn as_raw_ptr(&self) -> *mut c_void {
        self.ptr
    }

    /// Return every export preset reported by the current runtime.
    pub fn available_presets() -> Result<Vec<ExportPreset>, AVWriterError> {
        let ptr = unsafe { ffi::av_export_session_all_presets_json() };
        let raw: Vec<String> = parse_json_ptr(ptr, "export presets")?;
        raw.into_iter()
            .map(|value| {
                ExportPreset::from_raw(&value).ok_or_else(|| {
                    AVWriterError::InvalidState(format!(
                        "swift bridge returned unknown export preset '{value}'"
                    ))
                })
            })
            .collect()
    }

    /// Return presets compatible with the asset at `path`.
    pub fn compatible_presets(path: impl AsRef<Path>) -> Result<Vec<ExportPreset>, AVWriterError> {
        let path_c = path_arg(path, "source asset path")?;
        let ptr = unsafe { ffi::av_export_session_compatible_presets_json(path_c.as_ptr()) };
        let raw: Vec<String> = parse_json_ptr(ptr, "compatible export presets")?;
        raw.into_iter()
            .map(|value| {
                ExportPreset::from_raw(&value).ok_or_else(|| {
                    AVWriterError::InvalidState(format!(
                        "swift bridge returned unknown compatible export preset '{value}'"
                    ))
                })
            })
            .collect()
    }

    /// Determine whether a preset can export the asset at `path` to `output_file_type`.
    pub fn determine_compatibility(
        path: impl AsRef<Path>,
        preset: ExportPreset,
        output_file_type: Option<FileType>,
    ) -> Result<bool, AVWriterError> {
        let path_c = path_arg(path, "source asset path")?;
        let preset_c = cstring_arg(preset.as_str(), "export preset")?;
        let file_type_c = output_file_type
            .map(|value| cstring_arg(value.as_str(), "output file type"))
            .transpose()?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let result = unsafe {
            ffi::av_export_session_determine_compatibility(
                path_c.as_ptr(),
                preset_c.as_ptr(),
                file_type_c
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

    /// Create an export session for the asset at `path` using `preset`.
    pub fn new(path: impl AsRef<Path>, preset: ExportPreset) -> Result<Self, AVWriterError> {
        let path_c = path_arg(path, "source asset path")?;
        let preset_c = cstring_arg(preset.as_str(), "export preset")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let ptr = unsafe {
            ffi::av_export_session_create(path_c.as_ptr(), preset_c.as_ptr(), &mut err_msg)
        };
        if ptr.is_null() {
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        Ok(Self { ptr })
    }

    pub fn preset_name(&self) -> Result<ExportPreset, AVWriterError> {
        let raw = self.info()?.preset_name;
        ExportPreset::from_raw(&raw).ok_or_else(|| {
            AVWriterError::InvalidState(format!(
                "swift bridge returned unknown export preset '{raw}'"
            ))
        })
    }

    pub fn asset_path(&self) -> Result<Option<String>, AVWriterError> {
        Ok(self.info()?.asset_path)
    }

    pub fn output_file_type(&self) -> Result<Option<FileType>, AVWriterError> {
        self.info()?.output_file_type.map_or(Ok(None), |raw| {
            decode_file_type(&raw, "export session output file type").map(Some)
        })
    }

    pub fn set_output_file_type(&self, file_type: Option<FileType>) -> Result<(), AVWriterError> {
        let file_type_c = file_type
            .map(|value| cstring_arg(value.as_str(), "output file type"))
            .transpose()?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_export_session_set_output_file_type(
                self.ptr,
                file_type_c
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

    pub fn output_path(&self) -> Result<Option<String>, AVWriterError> {
        Ok(self.info()?.output_path)
    }

    pub fn set_output_path(&self, path: Option<&Path>) -> Result<(), AVWriterError> {
        let path_c = path
            .map(|value| path_arg(value, "output path"))
            .transpose()?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_export_session_set_output_path(
                self.ptr,
                path_c.as_ref().map_or(ptr::null(), |value| value.as_ptr()),
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn should_optimize_for_network_use(&self) -> Result<bool, AVWriterError> {
        Ok(self.info()?.should_optimize_for_network_use)
    }

    pub fn set_should_optimize_for_network_use(&self, enabled: bool) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_export_session_set_should_optimize_for_network_use(
                self.ptr,
                enabled,
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn allows_parallelized_export(&self) -> Result<bool, AVWriterError> {
        Ok(self.info()?.allows_parallelized_export)
    }

    pub fn set_allows_parallelized_export(&self, enabled: bool) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_export_session_set_allows_parallelized_export(self.ptr, enabled, &mut err_msg)
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn status(&self) -> Result<ExportStatus, AVWriterError> {
        Ok(ExportStatus::from_raw(self.info()?.status))
    }

    pub fn error_message(&self) -> Result<Option<String>, AVWriterError> {
        Ok(self.info()?.error_message)
    }

    pub fn progress(&self) -> Result<f32, AVWriterError> {
        Ok(self.info()?.progress)
    }

    /// Start the export and block until completion.
    pub fn export(&self) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe { ffi::av_export_session_export(self.ptr, &mut err_msg) };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    /// Cancel a running export.
    pub fn cancel(&self) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe { ffi::av_export_session_cancel(self.ptr, &mut err_msg) };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn supported_file_types(&self) -> Result<Vec<FileType>, AVWriterError> {
        decode_file_types(
            &self.info()?.supported_file_types,
            "supported export file types",
        )
    }

    pub fn compatible_file_types(&self) -> Result<Vec<FileType>, AVWriterError> {
        let ptr = unsafe { ffi::av_export_session_compatible_file_types_json(self.ptr) };
        let raw: Vec<String> = parse_json_ptr(ptr, "compatible export file types")?;
        decode_file_types(&raw, "compatible export file types")
    }

    pub fn time_range(&self) -> Result<TimeRange, AVWriterError> {
        Ok(self.info()?.time_range)
    }

    pub fn set_time_range(&self, range: TimeRange) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_export_session_set_time_range(
                self.ptr,
                time_value(&range.start),
                time_scale(&range.start),
                time_kind(&range.start),
                time_value(&range.duration),
                time_scale(&range.duration),
                time_kind(&range.duration),
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn file_length_limit(&self) -> Result<i64, AVWriterError> {
        Ok(self.info()?.file_length_limit)
    }

    pub fn set_file_length_limit(&self, limit: i64) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status =
            unsafe { ffi::av_export_session_set_file_length_limit(self.ptr, limit, &mut err_msg) };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn estimated_maximum_duration(&self) -> Result<Time, AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let ptr = unsafe {
            ffi::av_export_session_estimated_maximum_duration_json(self.ptr, &mut err_msg)
        };
        if ptr.is_null() {
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        parse_json_ptr(ptr, "estimated maximum duration")
    }

    pub fn estimated_output_file_length(&self) -> Result<i64, AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let result =
            unsafe { ffi::av_export_session_estimated_output_file_length(self.ptr, &mut err_msg) };
        if result == i64::MIN {
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        Ok(result)
    }

    pub fn metadata(&self) -> Result<Vec<MetadataItem>, AVWriterError> {
        Ok(self.info()?.metadata)
    }

    pub fn set_metadata(&self, metadata: &[MetadataItem]) -> Result<(), AVWriterError> {
        let payload = serialize_json(metadata)?;
        let payload_c = cstring_arg(&payload, "export metadata json")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_export_session_set_metadata_json(self.ptr, payload_c.as_ptr(), &mut err_msg)
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn metadata_item_filter(&self) -> Result<Option<MetadataItemFilter>, AVWriterError> {
        Ok(MetadataItemFilter::from_raw(unsafe {
            ffi::av_export_session_metadata_item_filter(self.ptr)
        }))
    }

    pub fn set_metadata_item_filter(
        &self,
        filter: Option<&MetadataItemFilter>,
    ) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_export_session_set_metadata_item_filter(
                self.ptr,
                filter.map_or(ptr::null_mut(), MetadataItemFilter::as_ptr),
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn audio_mix(&self) -> Result<Option<AudioMix>, AVWriterError> {
        Ok(AudioMix::from_raw(unsafe {
            ffi::av_export_session_audio_mix(self.ptr)
        }))
    }

    pub fn set_audio_mix(&self, audio_mix: Option<&AudioMix>) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_export_session_set_audio_mix(
                self.ptr,
                audio_mix.map_or(ptr::null_mut(), AudioMix::as_ptr),
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn video_composition(&self) -> Result<Option<VideoComposition>, AVWriterError> {
        Ok(VideoComposition::from_raw(unsafe {
            ffi::av_export_session_video_composition(self.ptr)
        }))
    }

    pub fn set_video_composition(
        &self,
        composition: Option<&VideoComposition>,
    ) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_export_session_set_video_composition(
                self.ptr,
                composition.map_or(ptr::null_mut(), VideoComposition::as_ptr),
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn custom_video_compositor(&self) -> Result<Option<VideoCompositor>, AVWriterError> {
        Ok(VideoCompositor::from_raw(unsafe {
            ffi::av_export_session_custom_video_compositor(self.ptr)
        }))
    }

    pub fn can_perform_multiple_passes_over_source_media_data(
        &self,
    ) -> Result<bool, AVWriterError> {
        Ok(self
            .info()?
            .can_perform_multiple_passes_over_source_media_data)
    }

    pub fn set_can_perform_multiple_passes_over_source_media_data(
        &self,
        enabled: bool,
    ) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_export_session_set_can_perform_multiple_passes_over_source_media_data(
                self.ptr,
                enabled,
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn directory_for_temporary_files(&self) -> Result<Option<String>, AVWriterError> {
        Ok(self.info()?.directory_for_temporary_files)
    }

    pub fn set_directory_for_temporary_files(
        &self,
        path: Option<&Path>,
    ) -> Result<(), AVWriterError> {
        let path_c = path
            .map(|value| path_arg(value, "temporary directory path"))
            .transpose()?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_export_session_set_directory_for_temporary_files(
                self.ptr,
                path_c.as_ref().map_or(ptr::null(), |value| value.as_ptr()),
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn audio_track_group_handling(&self) -> Result<TrackGroupOutputHandling, AVWriterError> {
        Ok(TrackGroupOutputHandling::from_raw(
            self.info()?.audio_track_group_handling,
        ))
    }

    pub fn set_audio_track_group_handling(
        &self,
        handling: TrackGroupOutputHandling,
    ) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_export_session_set_audio_track_group_handling(
                self.ptr,
                handling.raw(),
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    fn info(&self) -> Result<ExportSessionInfoPayload, AVWriterError> {
        let ptr = unsafe { ffi::av_export_session_info_json(self.ptr) };
        parse_json_ptr(ptr, "export session info")
    }
}

impl Drop for ExportSession {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ffi::av_export_session_release(self.ptr) };
        }
    }
}

impl core::fmt::Debug for ExportSession {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ExportSession")
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

fn decode_file_type(raw: &str, context: &str) -> Result<FileType, AVWriterError> {
    FileType::from_raw(raw).ok_or_else(|| {
        AVWriterError::InvalidState(format!("swift bridge returned unknown {context} '{raw}'"))
    })
}

fn decode_file_types(raw: &[String], context: &str) -> Result<Vec<FileType>, AVWriterError> {
    raw.iter()
        .map(|value| decode_file_type(value, context))
        .collect()
}
