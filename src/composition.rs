#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::needless_pass_by_value,
    clippy::struct_excessive_bools
)]

use core::ffi::c_void;
use core::ptr;
use std::path::Path;

use crate::PropertyValue;
use apple_cf::cm::CMFormatDescription;

use crate::asset::Asset;
use crate::bridge_support::{string_arg, take_payload, time_kind, time_scale, time_value};
use crate::error::{from_swift, AVWriterError};
use crate::ffi;
use crate::writer::MediaType;
use apple_cf::cm::{CMTime, CMTimeRange};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompositionSize {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CompositionTransform {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub tx: f64,
    pub ty: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompositionTimeMapping {
    pub source: CMTimeRange,
    pub target: CMTimeRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatDescriptionSummary {
    pub media_type_raw: u32,
    pub media_type: String,
    pub media_subtype_raw: u32,
    pub media_subtype: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompositionInfo {
    pub duration: CMTime,
    pub natural_size: CompositionSize,
    pub is_playable: bool,
    pub is_exportable: bool,
    pub is_readable: bool,
    pub is_composable: bool,
    pub url_asset_initialization_options: Option<PropertyValue>,
    pub track_ids: Vec<i32>,
    pub unused_track_id: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompositionTrackInfo {
    pub track_id: i32,
    pub media_type: MediaType,
    pub is_playable: bool,
    pub is_decodable: bool,
    pub is_enabled: bool,
    pub is_self_contained: bool,
    pub total_sample_data_length: i64,
    pub time_range: CMTimeRange,
    pub natural_time_scale: i32,
    pub estimated_data_rate: f32,
    pub language_code: Option<String>,
    pub extended_language_tag: Option<String>,
    pub natural_size: CompositionSize,
    pub preferred_transform: CompositionTransform,
    pub preferred_volume: f32,
    pub has_audio_sample_dependencies: bool,
    pub nominal_frame_rate: f32,
    pub min_frame_duration: CMTime,
    pub requires_frame_reordering: bool,
    pub segment_count: usize,
    pub format_description_count: usize,
    pub format_description_replacement_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetTrackSegmentInfo {
    pub time_mapping: CompositionTimeMapping,
    pub is_empty: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositionTrackSegmentInfo {
    pub time_mapping: CompositionTimeMapping,
    pub is_empty: bool,
    pub source_url: Option<String>,
    pub source_track_id: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompositionTrackFormatDescriptionReplacementInfo {
    pub original_format_description: FormatDescriptionSummary,
    pub replacement_format_description: FormatDescriptionSummary,
}

#[derive(Debug, Clone, PartialEq)]
struct CompositionInfoPayload {
    duration: CMTime,
    natural_size: CompositionSize,
    is_playable: bool,
    is_exportable: bool,
    is_readable: bool,
    is_composable: bool,
    url_asset_initialization_options: Option<PropertyValue>,
    track_ids: Vec<i32>,
    unused_track_id: i32,
}

#[derive(Debug, Clone, PartialEq)]
struct CompositionTrackInfoPayload {
    track_id: i32,
    media_type: String,
    is_playable: bool,
    is_decodable: bool,
    is_enabled: bool,
    is_self_contained: bool,
    total_sample_data_length: i64,
    time_range: CMTimeRange,
    natural_time_scale: i32,
    estimated_data_rate: f32,
    language_code: Option<String>,
    extended_language_tag: Option<String>,
    natural_size: CompositionSize,
    preferred_transform: CompositionTransform,
    preferred_volume: f32,
    has_audio_sample_dependencies: bool,
    nominal_frame_rate: f32,
    min_frame_duration: CMTime,
    requires_frame_reordering: bool,
    segment_count: usize,
    format_description_count: usize,
    format_description_replacement_count: usize,
}

fn missing_payload(context: &str) -> AVWriterError {
    AVWriterError::InvalidState(format!("swift bridge returned no {context}"))
}

fn decode_composition_info(payload: CompositionInfoPayload) -> CompositionInfo {
    CompositionInfo {
        duration: payload.duration,
        natural_size: payload.natural_size,
        is_playable: payload.is_playable,
        is_exportable: payload.is_exportable,
        is_readable: payload.is_readable,
        is_composable: payload.is_composable,
        url_asset_initialization_options: payload.url_asset_initialization_options,
        track_ids: payload.track_ids,
        unused_track_id: payload.unused_track_id,
    }
}

fn decode_composition_track_info(payload: CompositionTrackInfoPayload) -> CompositionTrackInfo {
    CompositionTrackInfo {
        track_id: payload.track_id,
        media_type: MediaType::from_raw(&payload.media_type),
        is_playable: payload.is_playable,
        is_decodable: payload.is_decodable,
        is_enabled: payload.is_enabled,
        is_self_contained: payload.is_self_contained,
        total_sample_data_length: payload.total_sample_data_length,
        time_range: payload.time_range,
        natural_time_scale: payload.natural_time_scale,
        estimated_data_rate: payload.estimated_data_rate,
        language_code: payload.language_code,
        extended_language_tag: payload.extended_language_tag,
        natural_size: payload.natural_size,
        preferred_transform: payload.preferred_transform,
        preferred_volume: payload.preferred_volume,
        has_audio_sample_dependencies: payload.has_audio_sample_dependencies,
        nominal_frame_rate: payload.nominal_frame_rate,
        min_frame_duration: payload.min_frame_duration,
        requires_frame_reordering: payload.requires_frame_reordering,
        segment_count: payload.segment_count,
        format_description_count: payload.format_description_count,
        format_description_replacement_count: payload.format_description_replacement_count,
    }
}

fn take_format_description(
    ptr: *mut c_void,
    context: &str,
) -> Result<CMFormatDescription, AVWriterError> {
    // SAFETY: the Swift bridge returns a +1 retained CMFormatDescription.
    unsafe { CMFormatDescription::from_raw(ptr) }.ok_or_else(|| missing_payload(context))
}

pub struct Composition {
    ptr: *mut c_void,
}

unsafe impl Send for Composition {}

impl Composition {
    pub fn empty() -> Result<Self, AVWriterError> {
        let ptr = unsafe { ffi::av_composition_create_empty() };
        Self::from_raw(ptr).ok_or_else(|| missing_payload("composition"))
    }

    pub fn from_asset(asset: &Asset) -> Result<Self, AVWriterError> {
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let ptr =
            unsafe { ffi::av_composition_create_from_asset(asset.as_ptr(), &raw mut err_msg) };
        Self::from_raw(ptr)
            .ok_or_else(|| unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) })
    }

    pub fn from_file_path(path: impl AsRef<Path>) -> Result<Self, AVWriterError> {
        let asset = Asset::from_file_path(path)?;
        Self::from_asset(&asset)
    }

    pub fn snapshot(&self) -> Result<CompositionInfo, AVWriterError> {
        Ok(decode_composition_info(self.info_payload()?))
    }

    pub fn duration(&self) -> Result<CMTime, AVWriterError> {
        Ok(self.snapshot()?.duration)
    }

    pub fn natural_size(&self) -> Result<(f64, f64), AVWriterError> {
        let size = self.snapshot()?.natural_size;
        Ok((size.width, size.height))
    }

    pub fn is_playable(&self) -> Result<bool, AVWriterError> {
        Ok(self.snapshot()?.is_playable)
    }

    pub fn is_exportable(&self) -> Result<bool, AVWriterError> {
        Ok(self.snapshot()?.is_exportable)
    }

    pub fn is_readable(&self) -> Result<bool, AVWriterError> {
        Ok(self.snapshot()?.is_readable)
    }

    pub fn is_composable(&self) -> Result<bool, AVWriterError> {
        Ok(self.snapshot()?.is_composable)
    }

    pub fn url_asset_initialization_options(&self) -> Result<Option<PropertyValue>, AVWriterError> {
        Ok(self.snapshot()?.url_asset_initialization_options)
    }

    pub fn track_ids(&self) -> Result<Vec<i32>, AVWriterError> {
        Ok(self.snapshot()?.track_ids)
    }

    pub fn unused_track_id(&self) -> Result<i32, AVWriterError> {
        Ok(self.snapshot()?.unused_track_id)
    }

    pub fn track_count(&self) -> usize {
        unsafe { ffi::av_composition_track_count(self.ptr) }
    }

    pub fn tracks(&self) -> Result<Vec<CompositionTrack>, AVWriterError> {
        let count = self.track_count();
        let mut tracks = Vec::with_capacity(count);
        for index in 0..count {
            let ptr = unsafe { ffi::av_composition_copy_track_at_index(self.ptr, index) };
            let track = CompositionTrack::from_raw(ptr)
                .ok_or_else(|| missing_payload("composition track"))?;
            tracks.push(track);
        }
        Ok(tracks)
    }

    pub fn track_with_track_id(
        &self,
        track_id: i32,
    ) -> Result<Option<CompositionTrack>, AVWriterError> {
        for track in self.tracks()? {
            if track.track_id()? == track_id {
                return Ok(Some(track));
            }
        }
        Ok(None)
    }

    pub fn tracks_with_media_type(
        &self,
        media_type: MediaType,
    ) -> Result<Vec<CompositionTrack>, AVWriterError> {
        let mut tracks = Vec::new();
        for track in self.tracks()? {
            if track.media_type()? == media_type {
                tracks.push(track);
            }
        }
        Ok(tracks)
    }

    pub fn tracks_with_media_characteristic(
        &self,
        media_characteristic: &str,
    ) -> Result<Vec<CompositionTrack>, AVWriterError> {
        let mut tracks = Vec::new();
        for track in self.tracks()? {
            if track.has_media_characteristic(media_characteristic)? {
                tracks.push(track);
            }
        }
        Ok(tracks)
    }

    pub(crate) fn from_raw(ptr: *mut c_void) -> Option<Self> {
        (!ptr.is_null()).then_some(Self { ptr })
    }

    fn info_payload(&self) -> Result<CompositionInfoPayload, AVWriterError> {
        let ptr = unsafe { ffi::av_composition_info_payload(self.ptr) };
        take_payload(ptr, "composition info")
    }
}

impl Drop for Composition {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ffi::av_composition_release(self.ptr) };
            self.ptr = ptr::null_mut();
        }
    }
}

impl core::fmt::Debug for Composition {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Composition")
            .field("ptr", &self.ptr)
            .finish()
    }
}

pub struct CompositionTrack {
    ptr: *mut c_void,
}

unsafe impl Send for CompositionTrack {}

impl CompositionTrack {
    pub fn snapshot(&self) -> Result<CompositionTrackInfo, AVWriterError> {
        Ok(decode_composition_track_info(self.info_payload()?))
    }

    pub fn track_id(&self) -> Result<i32, AVWriterError> {
        Ok(self.snapshot()?.track_id)
    }

    pub fn media_type(&self) -> Result<MediaType, AVWriterError> {
        Ok(self.snapshot()?.media_type)
    }

    pub fn is_playable(&self) -> Result<bool, AVWriterError> {
        Ok(self.snapshot()?.is_playable)
    }

    pub fn is_decodable(&self) -> Result<bool, AVWriterError> {
        Ok(self.snapshot()?.is_decodable)
    }

    pub fn is_enabled(&self) -> Result<bool, AVWriterError> {
        Ok(self.snapshot()?.is_enabled)
    }

    pub fn is_self_contained(&self) -> Result<bool, AVWriterError> {
        Ok(self.snapshot()?.is_self_contained)
    }

    pub fn total_sample_data_length(&self) -> Result<i64, AVWriterError> {
        Ok(self.snapshot()?.total_sample_data_length)
    }

    pub fn time_range(&self) -> Result<CMTimeRange, AVWriterError> {
        Ok(self.snapshot()?.time_range)
    }

    pub fn natural_time_scale(&self) -> Result<i32, AVWriterError> {
        Ok(self.snapshot()?.natural_time_scale)
    }

    pub fn estimated_data_rate(&self) -> Result<f32, AVWriterError> {
        Ok(self.snapshot()?.estimated_data_rate)
    }

    pub fn language_code(&self) -> Result<Option<String>, AVWriterError> {
        Ok(self.snapshot()?.language_code)
    }

    pub fn extended_language_tag(&self) -> Result<Option<String>, AVWriterError> {
        Ok(self.snapshot()?.extended_language_tag)
    }

    pub fn natural_size(&self) -> Result<(f64, f64), AVWriterError> {
        let size = self.snapshot()?.natural_size;
        Ok((size.width, size.height))
    }

    pub fn preferred_transform(&self) -> Result<CompositionTransform, AVWriterError> {
        Ok(self.snapshot()?.preferred_transform)
    }

    pub fn preferred_volume(&self) -> Result<f32, AVWriterError> {
        Ok(self.snapshot()?.preferred_volume)
    }

    pub fn has_audio_sample_dependencies(&self) -> Result<bool, AVWriterError> {
        Ok(self.snapshot()?.has_audio_sample_dependencies)
    }

    pub fn nominal_frame_rate(&self) -> Result<f32, AVWriterError> {
        Ok(self.snapshot()?.nominal_frame_rate)
    }

    pub fn min_frame_duration(&self) -> Result<CMTime, AVWriterError> {
        Ok(self.snapshot()?.min_frame_duration)
    }

    pub fn requires_frame_reordering(&self) -> Result<bool, AVWriterError> {
        Ok(self.snapshot()?.requires_frame_reordering)
    }

    pub fn segment_count(&self) -> usize {
        unsafe { ffi::av_composition_track_segment_count(self.ptr) }
    }

    pub fn segments(&self) -> Result<Vec<CompositionTrackSegment>, AVWriterError> {
        let count = self.segment_count();
        let mut segments = Vec::with_capacity(count);
        for index in 0..count {
            let ptr = unsafe { ffi::av_composition_track_copy_segment_at_index(self.ptr, index) };
            let segment = CompositionTrackSegment::from_raw(ptr)
                .ok_or_else(|| missing_payload("composition track segment"))?;
            segments.push(segment);
        }
        Ok(segments)
    }

    pub fn segment_for_track_time(
        &self,
        track_time: CMTime,
    ) -> Result<Option<CompositionTrackSegment>, AVWriterError> {
        Ok(CompositionTrackSegment::from_raw(unsafe {
            ffi::av_composition_track_segment_for_track_time(
                self.ptr,
                time_value(&track_time),
                time_scale(&track_time),
                time_kind(&track_time),
            )
        }))
    }

    pub fn has_media_characteristic(
        &self,
        media_characteristic: &str,
    ) -> Result<bool, AVWriterError> {
        let media_characteristic_c = string_arg(media_characteristic, "media characteristic")?;
        Ok(unsafe {
            ffi::av_composition_track_has_media_characteristic(
                self.ptr,
                media_characteristic_c.as_ptr(),
            )
        })
    }

    pub fn sample_presentation_time_for_track_time(
        &self,
        track_time: CMTime,
    ) -> Result<CMTime, AVWriterError> {
        let ptr = unsafe {
            ffi::av_composition_track_sample_presentation_time_payload(
                self.ptr,
                time_value(&track_time),
                time_scale(&track_time),
                time_kind(&track_time),
            )
        };
        take_payload::<crate::time::TimeValue>(ptr, "composition track sample presentation time")
            .map(|time| time.0)
    }

    pub fn format_description_count(&self) -> usize {
        unsafe { ffi::av_composition_track_format_description_count(self.ptr) }
    }

    pub fn format_descriptions(&self) -> Result<Vec<CMFormatDescription>, AVWriterError> {
        let count = self.format_description_count();
        let mut descriptions = Vec::with_capacity(count);
        for index in 0..count {
            let ptr = unsafe {
                ffi::av_composition_track_copy_format_description_at_index(self.ptr, index)
            };
            descriptions.push(take_format_description(
                ptr,
                "composition track format description",
            )?);
        }
        Ok(descriptions)
    }

    pub fn format_description_replacement_count(&self) -> usize {
        unsafe { ffi::av_composition_track_format_description_replacement_count(self.ptr) }
    }

    pub fn format_description_replacements(
        &self,
    ) -> Result<Vec<CompositionTrackFormatDescriptionReplacement>, AVWriterError> {
        let count = self.format_description_replacement_count();
        let mut replacements = Vec::with_capacity(count);
        for index in 0..count {
            let ptr = unsafe {
                ffi::av_composition_track_copy_format_description_replacement_at_index(
                    self.ptr, index,
                )
            };
            let replacement = CompositionTrackFormatDescriptionReplacement::from_raw(ptr)
                .ok_or_else(|| {
                    missing_payload("composition track format description replacement")
                })?;
            replacements.push(replacement);
        }
        Ok(replacements)
    }

    pub(crate) fn from_raw(ptr: *mut c_void) -> Option<Self> {
        (!ptr.is_null()).then_some(Self { ptr })
    }

    fn info_payload(&self) -> Result<CompositionTrackInfoPayload, AVWriterError> {
        let ptr = unsafe { ffi::av_composition_track_info_payload(self.ptr) };
        take_payload(ptr, "composition track info")
    }
}

impl Drop for CompositionTrack {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ffi::av_composition_track_release(self.ptr) };
            self.ptr = ptr::null_mut();
        }
    }
}

impl core::fmt::Debug for CompositionTrack {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CompositionTrack")
            .field("ptr", &self.ptr)
            .finish()
    }
}

pub struct CompositionTrackSegment {
    ptr: *mut c_void,
}

unsafe impl Send for CompositionTrackSegment {}

impl CompositionTrackSegment {
    pub fn from_source_file_path(
        path: impl AsRef<Path>,
        track_id: i32,
        source_time_range: CMTimeRange,
        target_time_range: CMTimeRange,
    ) -> Result<Self, AVWriterError> {
        let path = path
            .as_ref()
            .to_str()
            .ok_or_else(|| AVWriterError::InvalidArgument("path is not valid UTF-8".into()))?;
        Self::from_raw_url(path, true, track_id, source_time_range, target_time_range)
    }

    pub fn from_source_url(
        url: impl AsRef<str>,
        track_id: i32,
        source_time_range: CMTimeRange,
        target_time_range: CMTimeRange,
    ) -> Result<Self, AVWriterError> {
        Self::from_raw_url(
            url.as_ref(),
            false,
            track_id,
            source_time_range,
            target_time_range,
        )
    }

    pub fn empty(time_range: CMTimeRange) -> Result<Self, AVWriterError> {
        let ptr = unsafe {
            ffi::av_composition_track_segment_create_empty(
                time_value(&time_range.start),
                time_scale(&time_range.start),
                time_kind(&time_range.start),
                time_value(&time_range.duration),
                time_scale(&time_range.duration),
                time_kind(&time_range.duration),
            )
        };
        Self::from_raw(ptr).ok_or_else(|| missing_payload("composition track segment"))
    }

    pub fn snapshot(&self) -> Result<CompositionTrackSegmentInfo, AVWriterError> {
        let ptr = unsafe { ffi::av_composition_track_segment_info_payload(self.ptr) };
        take_payload(ptr, "composition track segment info")
    }

    pub fn time_mapping(&self) -> Result<CompositionTimeMapping, AVWriterError> {
        Ok(self.snapshot()?.time_mapping)
    }

    pub fn is_empty(&self) -> Result<bool, AVWriterError> {
        Ok(self.snapshot()?.is_empty)
    }

    pub fn source_url(&self) -> Result<Option<String>, AVWriterError> {
        Ok(self.snapshot()?.source_url)
    }

    pub fn source_track_id(&self) -> Result<Option<i32>, AVWriterError> {
        Ok(self.snapshot()?.source_track_id)
    }

    pub fn asset_track_segment(&self) -> Result<AssetTrackSegment, AVWriterError> {
        let ptr = unsafe { ffi::av_composition_track_segment_asset_track_segment(self.ptr) };
        AssetTrackSegment::from_raw(ptr).ok_or_else(|| missing_payload("asset track segment"))
    }

    pub(crate) fn from_raw(ptr: *mut c_void) -> Option<Self> {
        (!ptr.is_null()).then_some(Self { ptr })
    }

    fn from_raw_url(
        url: &str,
        is_file_url: bool,
        track_id: i32,
        source_time_range: CMTimeRange,
        target_time_range: CMTimeRange,
    ) -> Result<Self, AVWriterError> {
        let url_c = string_arg(url, "segment source URL")?;
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let ptr = unsafe {
            ffi::av_composition_track_segment_create_url(
                url_c.as_ptr(),
                is_file_url,
                track_id,
                time_value(&source_time_range.start),
                time_scale(&source_time_range.start),
                time_kind(&source_time_range.start),
                time_value(&source_time_range.duration),
                time_scale(&source_time_range.duration),
                time_kind(&source_time_range.duration),
                time_value(&target_time_range.start),
                time_scale(&target_time_range.start),
                time_kind(&target_time_range.start),
                time_value(&target_time_range.duration),
                time_scale(&target_time_range.duration),
                time_kind(&target_time_range.duration),
                &raw mut err_msg,
            )
        };
        Self::from_raw(ptr)
            .ok_or_else(|| unsafe { from_swift(ffi::status::INVALID_ARGUMENT, err_msg) })
    }
}

impl Drop for CompositionTrackSegment {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ffi::av_composition_track_segment_release(self.ptr) };
            self.ptr = ptr::null_mut();
        }
    }
}

impl core::fmt::Debug for CompositionTrackSegment {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CompositionTrackSegment")
            .field("ptr", &self.ptr)
            .finish()
    }
}

pub struct AssetTrackSegment {
    ptr: *mut c_void,
}

unsafe impl Send for AssetTrackSegment {}

impl AssetTrackSegment {
    pub fn snapshot(&self) -> Result<AssetTrackSegmentInfo, AVWriterError> {
        let ptr = unsafe { ffi::av_asset_track_segment_info_payload(self.ptr) };
        take_payload(ptr, "asset track segment info")
    }

    pub fn time_mapping(&self) -> Result<CompositionTimeMapping, AVWriterError> {
        Ok(self.snapshot()?.time_mapping)
    }

    pub fn is_empty(&self) -> Result<bool, AVWriterError> {
        Ok(self.snapshot()?.is_empty)
    }

    pub(crate) fn from_raw(ptr: *mut c_void) -> Option<Self> {
        (!ptr.is_null()).then_some(Self { ptr })
    }
}

impl Drop for AssetTrackSegment {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ffi::av_asset_track_segment_release(self.ptr) };
            self.ptr = ptr::null_mut();
        }
    }
}

impl core::fmt::Debug for AssetTrackSegment {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AssetTrackSegment")
            .field("ptr", &self.ptr)
            .finish()
    }
}

pub struct CompositionTrackFormatDescriptionReplacement {
    ptr: *mut c_void,
}

unsafe impl Send for CompositionTrackFormatDescriptionReplacement {}

impl CompositionTrackFormatDescriptionReplacement {
    pub fn snapshot(
        &self,
    ) -> Result<CompositionTrackFormatDescriptionReplacementInfo, AVWriterError> {
        let ptr = unsafe {
            ffi::av_composition_track_format_description_replacement_info_payload(self.ptr)
        };
        take_payload(ptr, "composition track format description replacement info")
    }

    pub fn original_format_description(&self) -> Result<CMFormatDescription, AVWriterError> {
        let ptr = unsafe {
            ffi::av_composition_track_format_description_replacement_original_format_description(
                self.ptr,
            )
        };
        take_format_description(ptr, "original format description")
    }

    pub fn replacement_format_description(&self) -> Result<CMFormatDescription, AVWriterError> {
        let ptr = unsafe {
            ffi::av_composition_track_format_description_replacement_replacement_format_description(
                self.ptr,
            )
        };
        take_format_description(ptr, "replacement format description")
    }

    pub(crate) fn from_raw(ptr: *mut c_void) -> Option<Self> {
        (!ptr.is_null()).then_some(Self { ptr })
    }
}

impl Drop for CompositionTrackFormatDescriptionReplacement {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ffi::av_composition_track_format_description_replacement_release(self.ptr) };
            self.ptr = ptr::null_mut();
        }
    }
}

impl core::fmt::Debug for CompositionTrackFormatDescriptionReplacement {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CompositionTrackFormatDescriptionReplacement")
            .field("ptr", &self.ptr)
            .finish()
    }
}

impl crate::binary::ToAbi for CompositionSize {
    type Abi = crate::ffi::AVWSize;

    fn to_abi(&self, arena: &mut crate::binary::Arena) -> Self::Abi {
        Self::Abi {
            width: crate::binary::ToAbi::to_abi(&self.width, arena),
            height: crate::binary::ToAbi::to_abi(&self.height, arena),
        }
    }
}

impl crate::binary::FromAbi for CompositionSize {
    fn from_abi(raw: &Self::Abi) -> Result<Self, crate::error::AVWriterError> {
        Ok(Self {
            width: crate::binary::FromAbi::from_abi(&raw.width)?,
            height: crate::binary::FromAbi::from_abi(&raw.height)?,
        })
    }
}

impl crate::binary::ToAbi for CompositionTransform {
    type Abi = crate::ffi::AVWTransform;

    fn to_abi(&self, arena: &mut crate::binary::Arena) -> Self::Abi {
        Self::Abi {
            a: crate::binary::ToAbi::to_abi(&self.a, arena),
            b: crate::binary::ToAbi::to_abi(&self.b, arena),
            c: crate::binary::ToAbi::to_abi(&self.c, arena),
            d: crate::binary::ToAbi::to_abi(&self.d, arena),
            tx: crate::binary::ToAbi::to_abi(&self.tx, arena),
            ty: crate::binary::ToAbi::to_abi(&self.ty, arena),
        }
    }
}

impl crate::binary::FromAbi for CompositionTransform {
    fn from_abi(raw: &Self::Abi) -> Result<Self, crate::error::AVWriterError> {
        Ok(Self {
            a: crate::binary::FromAbi::from_abi(&raw.a)?,
            b: crate::binary::FromAbi::from_abi(&raw.b)?,
            c: crate::binary::FromAbi::from_abi(&raw.c)?,
            d: crate::binary::FromAbi::from_abi(&raw.d)?,
            tx: crate::binary::FromAbi::from_abi(&raw.tx)?,
            ty: crate::binary::FromAbi::from_abi(&raw.ty)?,
        })
    }
}

impl crate::binary::ToAbi for CompositionInfoPayload {
    type Abi = crate::ffi::AVWCompositionInfo;

    fn to_abi(&self, arena: &mut crate::binary::Arena) -> Self::Abi {
        Self::Abi {
            duration: crate::binary::ToAbi::to_abi(&self.duration, arena),
            natural_size: crate::binary::ToAbi::to_abi(&self.natural_size, arena),
            is_playable: crate::binary::ToAbi::to_abi(&self.is_playable, arena),
            is_exportable: crate::binary::ToAbi::to_abi(&self.is_exportable, arena),
            is_readable: crate::binary::ToAbi::to_abi(&self.is_readable, arena),
            is_composable: crate::binary::ToAbi::to_abi(&self.is_composable, arena),
            url_asset_initialization_options: crate::binary::ToAbi::to_abi(
                &self.url_asset_initialization_options,
                arena,
            ),
            track_ids: crate::binary::ToAbi::to_abi(&self.track_ids, arena),
            unused_track_id: crate::binary::ToAbi::to_abi(&self.unused_track_id, arena),
        }
    }
}

impl crate::binary::FromAbi for CompositionInfoPayload {
    fn from_abi(raw: &Self::Abi) -> Result<Self, crate::error::AVWriterError> {
        Ok(Self {
            duration: crate::binary::FromAbi::from_abi(&raw.duration)?,
            natural_size: crate::binary::FromAbi::from_abi(&raw.natural_size)?,
            is_playable: crate::binary::FromAbi::from_abi(&raw.is_playable)?,
            is_exportable: crate::binary::FromAbi::from_abi(&raw.is_exportable)?,
            is_readable: crate::binary::FromAbi::from_abi(&raw.is_readable)?,
            is_composable: crate::binary::FromAbi::from_abi(&raw.is_composable)?,
            url_asset_initialization_options: crate::binary::FromAbi::from_abi(
                &raw.url_asset_initialization_options,
            )?,
            track_ids: crate::binary::FromAbi::from_abi(&raw.track_ids)?,
            unused_track_id: crate::binary::FromAbi::from_abi(&raw.unused_track_id)?,
        })
    }
}

impl crate::binary::ToAbi for CompositionTimeMapping {
    type Abi = crate::ffi::AVWCompositionTimeMapping;

    fn to_abi(&self, arena: &mut crate::binary::Arena) -> Self::Abi {
        Self::Abi {
            source: crate::binary::ToAbi::to_abi(&self.source, arena),
            target: crate::binary::ToAbi::to_abi(&self.target, arena),
        }
    }
}

impl crate::binary::FromAbi for CompositionTimeMapping {
    fn from_abi(raw: &Self::Abi) -> Result<Self, crate::error::AVWriterError> {
        Ok(Self {
            source: crate::binary::FromAbi::from_abi(&raw.source)?,
            target: crate::binary::FromAbi::from_abi(&raw.target)?,
        })
    }
}

impl crate::binary::ToAbi for AssetTrackSegmentInfo {
    type Abi = crate::ffi::AVWAssetTrackSegmentInfo;

    fn to_abi(&self, arena: &mut crate::binary::Arena) -> Self::Abi {
        Self::Abi {
            time_mapping: crate::binary::ToAbi::to_abi(&self.time_mapping, arena),
            is_empty: crate::binary::ToAbi::to_abi(&self.is_empty, arena),
        }
    }
}

impl crate::binary::FromAbi for AssetTrackSegmentInfo {
    fn from_abi(raw: &Self::Abi) -> Result<Self, crate::error::AVWriterError> {
        Ok(Self {
            time_mapping: crate::binary::FromAbi::from_abi(&raw.time_mapping)?,
            is_empty: crate::binary::FromAbi::from_abi(&raw.is_empty)?,
        })
    }
}

impl crate::binary::ToAbi for CompositionTrackSegmentInfo {
    type Abi = crate::ffi::AVWCompositionTrackSegmentInfo;

    fn to_abi(&self, arena: &mut crate::binary::Arena) -> Self::Abi {
        Self::Abi {
            time_mapping: crate::binary::ToAbi::to_abi(&self.time_mapping, arena),
            is_empty: crate::binary::ToAbi::to_abi(&self.is_empty, arena),
            source_url: crate::binary::ToAbi::to_abi(&self.source_url, arena),
            source_track_id: crate::binary::ToAbi::to_abi(&self.source_track_id, arena),
        }
    }
}

impl crate::binary::FromAbi for CompositionTrackSegmentInfo {
    fn from_abi(raw: &Self::Abi) -> Result<Self, crate::error::AVWriterError> {
        Ok(Self {
            time_mapping: crate::binary::FromAbi::from_abi(&raw.time_mapping)?,
            is_empty: crate::binary::FromAbi::from_abi(&raw.is_empty)?,
            source_url: crate::binary::FromAbi::from_abi(&raw.source_url)?,
            source_track_id: crate::binary::FromAbi::from_abi(&raw.source_track_id)?,
        })
    }
}

impl crate::binary::ToAbi for FormatDescriptionSummary {
    type Abi = crate::ffi::AVWFormatDescriptionSummary;

    fn to_abi(&self, arena: &mut crate::binary::Arena) -> Self::Abi {
        Self::Abi {
            media_type_raw: crate::binary::ToAbi::to_abi(&self.media_type_raw, arena),
            media_type: crate::binary::ToAbi::to_abi(&self.media_type, arena),
            media_subtype_raw: crate::binary::ToAbi::to_abi(&self.media_subtype_raw, arena),
            media_subtype: crate::binary::ToAbi::to_abi(&self.media_subtype, arena),
        }
    }
}

impl crate::binary::FromAbi for FormatDescriptionSummary {
    fn from_abi(raw: &Self::Abi) -> Result<Self, crate::error::AVWriterError> {
        Ok(Self {
            media_type_raw: crate::binary::FromAbi::from_abi(&raw.media_type_raw)?,
            media_type: crate::binary::FromAbi::from_abi(&raw.media_type)?,
            media_subtype_raw: crate::binary::FromAbi::from_abi(&raw.media_subtype_raw)?,
            media_subtype: crate::binary::FromAbi::from_abi(&raw.media_subtype)?,
        })
    }
}

impl crate::binary::ToAbi for CompositionTrackFormatDescriptionReplacementInfo {
    type Abi = crate::ffi::AVWCompositionTrackFormatDescriptionReplacementInfo;

    fn to_abi(&self, arena: &mut crate::binary::Arena) -> Self::Abi {
        Self::Abi {
            original_format_description: crate::binary::ToAbi::to_abi(
                &self.original_format_description,
                arena,
            ),
            replacement_format_description: crate::binary::ToAbi::to_abi(
                &self.replacement_format_description,
                arena,
            ),
        }
    }
}

impl crate::binary::FromAbi for CompositionTrackFormatDescriptionReplacementInfo {
    fn from_abi(raw: &Self::Abi) -> Result<Self, crate::error::AVWriterError> {
        Ok(Self {
            original_format_description: crate::binary::FromAbi::from_abi(
                &raw.original_format_description,
            )?,
            replacement_format_description: crate::binary::FromAbi::from_abi(
                &raw.replacement_format_description,
            )?,
        })
    }
}

impl crate::binary::ToAbi for CompositionTrackInfoPayload {
    type Abi = crate::ffi::AVWCompositionTrackInfo;

    fn to_abi(&self, arena: &mut crate::binary::Arena) -> Self::Abi {
        Self::Abi {
            track_id: crate::binary::ToAbi::to_abi(&self.track_id, arena),
            media_type: crate::binary::ToAbi::to_abi(&self.media_type, arena),
            is_playable: crate::binary::ToAbi::to_abi(&self.is_playable, arena),
            is_decodable: crate::binary::ToAbi::to_abi(&self.is_decodable, arena),
            is_enabled: crate::binary::ToAbi::to_abi(&self.is_enabled, arena),
            is_self_contained: crate::binary::ToAbi::to_abi(&self.is_self_contained, arena),
            total_sample_data_length: crate::binary::ToAbi::to_abi(
                &self.total_sample_data_length,
                arena,
            ),
            time_range: crate::binary::ToAbi::to_abi(&self.time_range, arena),
            natural_time_scale: crate::binary::ToAbi::to_abi(&self.natural_time_scale, arena),
            estimated_data_rate: crate::binary::ToAbi::to_abi(&self.estimated_data_rate, arena),
            language_code: crate::binary::ToAbi::to_abi(&self.language_code, arena),
            extended_language_tag: crate::binary::ToAbi::to_abi(&self.extended_language_tag, arena),
            natural_size: crate::binary::ToAbi::to_abi(&self.natural_size, arena),
            preferred_transform: crate::binary::ToAbi::to_abi(&self.preferred_transform, arena),
            preferred_volume: crate::binary::ToAbi::to_abi(&self.preferred_volume, arena),
            has_audio_sample_dependencies: crate::binary::ToAbi::to_abi(
                &self.has_audio_sample_dependencies,
                arena,
            ),
            nominal_frame_rate: crate::binary::ToAbi::to_abi(&self.nominal_frame_rate, arena),
            min_frame_duration: crate::binary::ToAbi::to_abi(&self.min_frame_duration, arena),
            requires_frame_reordering: crate::binary::ToAbi::to_abi(
                &self.requires_frame_reordering,
                arena,
            ),
            segment_count: crate::binary::ToAbi::to_abi(&self.segment_count, arena),
            format_description_count: crate::binary::ToAbi::to_abi(
                &self.format_description_count,
                arena,
            ),
            format_description_replacement_count: crate::binary::ToAbi::to_abi(
                &self.format_description_replacement_count,
                arena,
            ),
        }
    }
}

impl crate::binary::FromAbi for CompositionTrackInfoPayload {
    fn from_abi(raw: &Self::Abi) -> Result<Self, crate::error::AVWriterError> {
        Ok(Self {
            track_id: crate::binary::FromAbi::from_abi(&raw.track_id)?,
            media_type: crate::binary::FromAbi::from_abi(&raw.media_type)?,
            is_playable: crate::binary::FromAbi::from_abi(&raw.is_playable)?,
            is_decodable: crate::binary::FromAbi::from_abi(&raw.is_decodable)?,
            is_enabled: crate::binary::FromAbi::from_abi(&raw.is_enabled)?,
            is_self_contained: crate::binary::FromAbi::from_abi(&raw.is_self_contained)?,
            total_sample_data_length: crate::binary::FromAbi::from_abi(
                &raw.total_sample_data_length,
            )?,
            time_range: crate::binary::FromAbi::from_abi(&raw.time_range)?,
            natural_time_scale: crate::binary::FromAbi::from_abi(&raw.natural_time_scale)?,
            estimated_data_rate: crate::binary::FromAbi::from_abi(&raw.estimated_data_rate)?,
            language_code: crate::binary::FromAbi::from_abi(&raw.language_code)?,
            extended_language_tag: crate::binary::FromAbi::from_abi(&raw.extended_language_tag)?,
            natural_size: crate::binary::FromAbi::from_abi(&raw.natural_size)?,
            preferred_transform: crate::binary::FromAbi::from_abi(&raw.preferred_transform)?,
            preferred_volume: crate::binary::FromAbi::from_abi(&raw.preferred_volume)?,
            has_audio_sample_dependencies: crate::binary::FromAbi::from_abi(
                &raw.has_audio_sample_dependencies,
            )?,
            nominal_frame_rate: crate::binary::FromAbi::from_abi(&raw.nominal_frame_rate)?,
            min_frame_duration: crate::binary::FromAbi::from_abi(&raw.min_frame_duration)?,
            requires_frame_reordering: crate::binary::FromAbi::from_abi(
                &raw.requires_frame_reordering,
            )?,
            segment_count: crate::binary::FromAbi::from_abi(&raw.segment_count)?,
            format_description_count: crate::binary::FromAbi::from_abi(
                &raw.format_description_count,
            )?,
            format_description_replacement_count: crate::binary::FromAbi::from_abi(
                &raw.format_description_replacement_count,
            )?,
        })
    }
}
