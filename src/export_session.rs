#![allow(clippy::missing_errors_doc, clippy::needless_pass_by_value)]

use core::ffi::c_void;
use core::ptr;
use std::path::Path;

use crate::bridge_support::{string_arg, take_payload, time_kind, time_scale, time_value};
use crate::error::{from_swift, AVWriterError};
use crate::ffi;
use crate::media_processing::{AudioMix, MetadataItemFilter, VideoComposition, VideoCompositor};
use crate::metadata::MetadataItem;
use crate::output::{prepare_output, OutputPolicy};
use crate::writer::FileType;
use apple_cf::cm::{CMTime, CMTimeRange};

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

#[derive(Debug)]
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
    time_range: CMTimeRange,
    file_length_limit: i64,
    can_perform_multiple_passes_over_source_media_data: bool,
    directory_for_temporary_files: Option<String>,
    audio_track_group_handling: u64,
}

/// Safe wrapper around `AVAssetExportSession`, backed by an internal `AVURLAsset`.
pub struct ExportSession {
    ptr: *mut c_void,
    output_policy: core::cell::Cell<OutputPolicy>,
}

// SAFETY: `ExportSession` wraps an ARC-retained `AVAssetExportSession` pointer.
// ARC retain/release operations are atomic, so the pointer is safe to move
// across threads.  Concurrent shared access (`Sync`) is not implemented because
// `AVAssetExportSession` is not documented as thread-safe for simultaneous
// method calls.
unsafe impl Send for ExportSession {}

impl ExportSession {
    /// Returns the raw Swift object pointer (for use by `async_api`).
    #[cfg(feature = "async")]
    pub(crate) const fn as_raw_ptr(&self) -> *mut c_void {
        self.ptr
    }

    /// Return every export preset reported by the current runtime.
    pub fn available_presets() -> Result<Vec<ExportPreset>, AVWriterError> {
        let ptr = unsafe { ffi::av_export_session_all_presets_payload() };
        let raw: Vec<String> = take_payload(ptr, "export presets")?;
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
        let ptr = unsafe { ffi::av_export_session_compatible_presets_payload(path_c.as_ptr()) };
        let raw: Vec<String> = take_payload(ptr, "compatible export presets")?;
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
        let preset_c = string_arg(preset.as_str(), "export preset")?;
        let file_type_c = output_file_type
            .map(|value| string_arg(value.as_str(), "output file type"))
            .transpose()?;
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let result = unsafe {
            ffi::av_export_session_determine_compatibility(
                path_c.as_ptr(),
                preset_c.as_ptr(),
                file_type_c
                    .as_ref()
                    .map_or(ptr::null(), super::binary::Argument::as_ptr),
                &raw mut err_msg,
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
        let preset_c = string_arg(preset.as_str(), "export preset")?;
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let ptr = unsafe {
            ffi::av_export_session_create(path_c.as_ptr(), preset_c.as_ptr(), &raw mut err_msg)
        };
        if ptr.is_null() {
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        Ok(Self {
            ptr,
            output_policy: core::cell::Cell::new(OutputPolicy::FailIfExists),
        })
    }

    #[must_use]
    pub fn output_policy(&self) -> OutputPolicy {
        self.output_policy.get()
    }

    pub fn set_output_policy(&self, policy: OutputPolicy) {
        self.output_policy.set(policy);
    }

    pub(crate) fn prepare_output(&self) -> Result<(), AVWriterError> {
        let Some(output) = self.output_path()? else {
            return Ok(());
        };
        let source = self.asset_path()?;
        prepare_output(
            Path::new(&output),
            self.output_policy.get(),
            source.as_deref().map(Path::new),
        )
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
            .map(|value| string_arg(value.as_str(), "output file type"))
            .transpose()?;
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let status = unsafe {
            ffi::av_export_session_set_output_file_type(
                self.ptr,
                file_type_c
                    .as_ref()
                    .map_or(ptr::null(), super::binary::Argument::as_ptr),
                &raw mut err_msg,
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
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let status = unsafe {
            ffi::av_export_session_set_output_path(
                self.ptr,
                path_c
                    .as_ref()
                    .map_or(ptr::null(), super::binary::Argument::as_ptr),
                &raw mut err_msg,
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
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let status = unsafe {
            ffi::av_export_session_set_should_optimize_for_network_use(
                self.ptr,
                enabled,
                &raw mut err_msg,
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
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let status = unsafe {
            ffi::av_export_session_set_allows_parallelized_export(
                self.ptr,
                enabled,
                &raw mut err_msg,
            )
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
    ///
    /// An existing file at the output path is only replaced under
    /// [`OutputPolicy::ReplaceFile`]; otherwise the export fails with
    /// [`AVWriterError::OutputExists`].
    pub fn export(&self) -> Result<(), AVWriterError> {
        self.prepare_output()?;
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let status = unsafe { ffi::av_export_session_export(self.ptr, &raw mut err_msg) };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    /// Cancel a running export.
    pub fn cancel(&self) -> Result<(), AVWriterError> {
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let status = unsafe { ffi::av_export_session_cancel(self.ptr, &raw mut err_msg) };
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
        let ptr = unsafe { ffi::av_export_session_compatible_file_types_payload(self.ptr) };
        let raw: Vec<String> = take_payload(ptr, "compatible export file types")?;
        decode_file_types(&raw, "compatible export file types")
    }

    pub fn time_range(&self) -> Result<CMTimeRange, AVWriterError> {
        Ok(self.info()?.time_range)
    }

    pub fn set_time_range(&self, range: CMTimeRange) -> Result<(), AVWriterError> {
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let status = unsafe {
            ffi::av_export_session_set_time_range(
                self.ptr,
                time_value(&range.start),
                time_scale(&range.start),
                time_kind(&range.start),
                time_value(&range.duration),
                time_scale(&range.duration),
                time_kind(&range.duration),
                &raw mut err_msg,
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
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let status = unsafe {
            ffi::av_export_session_set_file_length_limit(self.ptr, limit, &raw mut err_msg)
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn estimated_maximum_duration(&self) -> Result<CMTime, AVWriterError> {
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let ptr = unsafe {
            ffi::av_export_session_estimated_maximum_duration_payload(self.ptr, &raw mut err_msg)
        };
        if ptr.is_null() {
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        take_payload::<crate::time::TimeValue>(ptr, "estimated maximum duration").map(|time| time.0)
    }

    pub fn estimated_output_file_length(&self) -> Result<i64, AVWriterError> {
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let result = unsafe {
            ffi::av_export_session_estimated_output_file_length(self.ptr, &raw mut err_msg)
        };
        if result == i64::MIN {
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        Ok(result)
    }

    pub fn metadata(&self) -> Result<Vec<MetadataItem>, AVWriterError> {
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let array = unsafe { ffi::av_export_session_metadata(self.ptr, &raw mut err_msg) };
        if array.is_null() {
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        MetadataItem::from_retained_array(array, "export session metadata")
    }

    pub fn set_metadata(&self, metadata: &[MetadataItem]) -> Result<(), AVWriterError> {
        let item_ptrs = MetadataItem::native_ptrs(metadata)?;
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let status = unsafe {
            ffi::av_export_session_set_metadata_items(
                self.ptr,
                item_ptrs.as_ptr(),
                item_ptrs.len(),
                &raw mut err_msg,
            )
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
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let status = unsafe {
            ffi::av_export_session_set_metadata_item_filter(
                self.ptr,
                filter.map_or(ptr::null_mut(), MetadataItemFilter::as_ptr),
                &raw mut err_msg,
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
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let status = unsafe {
            ffi::av_export_session_set_audio_mix(
                self.ptr,
                audio_mix.map_or(ptr::null_mut(), AudioMix::as_ptr),
                &raw mut err_msg,
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
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let status = unsafe {
            ffi::av_export_session_set_video_composition(
                self.ptr,
                composition.map_or(ptr::null_mut(), VideoComposition::as_ptr),
                &raw mut err_msg,
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
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let status = unsafe {
            ffi::av_export_session_set_can_perform_multiple_passes_over_source_media_data(
                self.ptr,
                enabled,
                &raw mut err_msg,
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
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let status = unsafe {
            ffi::av_export_session_set_directory_for_temporary_files(
                self.ptr,
                path_c
                    .as_ref()
                    .map_or(ptr::null(), super::binary::Argument::as_ptr),
                &raw mut err_msg,
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
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let status = unsafe {
            ffi::av_export_session_set_audio_track_group_handling(
                self.ptr,
                handling.raw(),
                &raw mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    fn info(&self) -> Result<ExportSessionInfoPayload, AVWriterError> {
        let ptr = unsafe { ffi::av_export_session_info_payload(self.ptr) };
        take_payload(ptr, "export session info")
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
            .field("output_policy", &self.output_policy.get())
            .finish()
    }
}

fn path_arg(
    path: impl AsRef<Path>,
    context: &str,
) -> Result<crate::binary::Argument<ffi::AVWString>, AVWriterError> {
    let path_str = path
        .as_ref()
        .to_str()
        .ok_or_else(|| AVWriterError::InvalidArgument(format!("{context} is not valid UTF-8")))?;
    string_arg(path_str, context)
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

impl crate::binary::ToAbi for ExportSessionInfoPayload {
    type Abi = crate::ffi::AVWExportSessionInfo;

    fn to_abi(&self, arena: &mut crate::binary::Arena) -> Self::Abi {
        Self::Abi {
            preset_name: crate::binary::ToAbi::to_abi(&self.preset_name, arena),
            asset_path: crate::binary::ToAbi::to_abi(&self.asset_path, arena),
            output_file_type: crate::binary::ToAbi::to_abi(&self.output_file_type, arena),
            output_path: crate::binary::ToAbi::to_abi(&self.output_path, arena),
            should_optimize_for_network_use: crate::binary::ToAbi::to_abi(
                &self.should_optimize_for_network_use,
                arena,
            ),
            allows_parallelized_export: crate::binary::ToAbi::to_abi(
                &self.allows_parallelized_export,
                arena,
            ),
            status: crate::binary::ToAbi::to_abi(&self.status, arena),
            error_message: crate::binary::ToAbi::to_abi(&self.error_message, arena),
            progress: crate::binary::ToAbi::to_abi(&self.progress, arena),
            supported_file_types: crate::binary::ToAbi::to_abi(&self.supported_file_types, arena),
            time_range: crate::binary::ToAbi::to_abi(&self.time_range, arena),
            file_length_limit: crate::binary::ToAbi::to_abi(&self.file_length_limit, arena),
            can_perform_multiple_passes_over_source_media_data: crate::binary::ToAbi::to_abi(
                &self.can_perform_multiple_passes_over_source_media_data,
                arena,
            ),
            directory_for_temporary_files: crate::binary::ToAbi::to_abi(
                &self.directory_for_temporary_files,
                arena,
            ),
            audio_track_group_handling: crate::binary::ToAbi::to_abi(
                &self.audio_track_group_handling,
                arena,
            ),
        }
    }
}

impl crate::binary::FromAbi for ExportSessionInfoPayload {
    fn from_abi(raw: &Self::Abi) -> Result<Self, crate::error::AVWriterError> {
        Ok(Self {
            preset_name: crate::binary::FromAbi::from_abi(&raw.preset_name)?,
            asset_path: crate::binary::FromAbi::from_abi(&raw.asset_path)?,
            output_file_type: crate::binary::FromAbi::from_abi(&raw.output_file_type)?,
            output_path: crate::binary::FromAbi::from_abi(&raw.output_path)?,
            should_optimize_for_network_use: crate::binary::FromAbi::from_abi(
                &raw.should_optimize_for_network_use,
            )?,
            allows_parallelized_export: crate::binary::FromAbi::from_abi(
                &raw.allows_parallelized_export,
            )?,
            status: crate::binary::FromAbi::from_abi(&raw.status)?,
            error_message: crate::binary::FromAbi::from_abi(&raw.error_message)?,
            progress: crate::binary::FromAbi::from_abi(&raw.progress)?,
            supported_file_types: crate::binary::FromAbi::from_abi(&raw.supported_file_types)?,
            time_range: crate::binary::FromAbi::from_abi(&raw.time_range)?,
            file_length_limit: crate::binary::FromAbi::from_abi(&raw.file_length_limit)?,
            can_perform_multiple_passes_over_source_media_data: crate::binary::FromAbi::from_abi(
                &raw.can_perform_multiple_passes_over_source_media_data,
            )?,
            directory_for_temporary_files: crate::binary::FromAbi::from_abi(
                &raw.directory_for_temporary_files,
            )?,
            audio_track_group_handling: crate::binary::FromAbi::from_abi(
                &raw.audio_track_group_handling,
            )?,
        })
    }
}
