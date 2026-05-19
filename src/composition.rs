#![allow(
    clippy::missing_errors_doc,
    clippy::must_use_candidate,
    clippy::needless_pass_by_value,
    clippy::struct_excessive_bools
)]

use core::ffi::{c_char, c_void};
use core::ptr;
use std::path::Path;

use apple_cf::cm::CMFormatDescription;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::asset::Asset;
use crate::bridge_support::{
    cstring_arg, parse_json_ptr, parse_optional_json_string, time_kind, time_scale, time_value,
};
use crate::error::{from_swift, AVWriterError};
use crate::ffi;
use crate::time::{Time, TimeRange};
use crate::writer::MediaType;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositionSize {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositionTransform {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub tx: f64,
    pub ty: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositionTimeMapping {
    pub source: TimeRange,
    pub target: TimeRange,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatDescriptionSummary {
    pub media_type_raw: u32,
    pub media_type: String,
    pub media_subtype_raw: u32,
    pub media_subtype: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositionInfo {
    pub duration: Time,
    pub natural_size: CompositionSize,
    pub is_playable: bool,
    pub is_exportable: bool,
    pub is_readable: bool,
    pub is_composable: bool,
    pub url_asset_initialization_options: Option<JsonValue>,
    pub track_ids: Vec<i32>,
    pub unused_track_id: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositionTrackInfo {
    pub track_id: i32,
    pub media_type: MediaType,
    pub is_playable: bool,
    pub is_decodable: bool,
    pub is_enabled: bool,
    pub is_self_contained: bool,
    pub total_sample_data_length: i64,
    pub time_range: TimeRange,
    pub natural_time_scale: i32,
    pub estimated_data_rate: f32,
    pub language_code: Option<String>,
    pub extended_language_tag: Option<String>,
    pub natural_size: CompositionSize,
    pub preferred_transform: CompositionTransform,
    pub preferred_volume: f32,
    pub has_audio_sample_dependencies: bool,
    pub nominal_frame_rate: f32,
    pub min_frame_duration: Time,
    pub requires_frame_reordering: bool,
    pub segment_count: usize,
    pub format_description_count: usize,
    pub format_description_replacement_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetTrackSegmentInfo {
    pub time_mapping: CompositionTimeMapping,
    pub is_empty: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositionTrackSegmentInfo {
    pub time_mapping: CompositionTimeMapping,
    pub is_empty: bool,
    pub source_url: Option<String>,
    pub source_track_id: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositionTrackFormatDescriptionReplacementInfo {
    pub original_format_description: FormatDescriptionSummary,
    pub replacement_format_description: FormatDescriptionSummary,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompositionInfoPayload {
    duration: Time,
    natural_size: CompositionSize,
    is_playable: bool,
    is_exportable: bool,
    is_readable: bool,
    is_composable: bool,
    url_asset_initialization_options_json: Option<String>,
    track_ids: Vec<i32>,
    unused_track_id: i32,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompositionTrackInfoPayload {
    track_id: i32,
    media_type: String,
    is_playable: bool,
    is_decodable: bool,
    is_enabled: bool,
    is_self_contained: bool,
    total_sample_data_length: i64,
    time_range: TimeRange,
    natural_time_scale: i32,
    estimated_data_rate: f32,
    language_code: Option<String>,
    extended_language_tag: Option<String>,
    natural_size: CompositionSize,
    preferred_transform: CompositionTransform,
    preferred_volume: f32,
    has_audio_sample_dependencies: bool,
    nominal_frame_rate: f32,
    min_frame_duration: Time,
    requires_frame_reordering: bool,
    segment_count: usize,
    format_description_count: usize,
    format_description_replacement_count: usize,
}

fn missing_payload(context: &str) -> AVWriterError {
    AVWriterError::InvalidState(format!("swift bridge returned no {context}"))
}

fn decode_composition_info(
    payload: CompositionInfoPayload,
) -> Result<CompositionInfo, AVWriterError> {
    Ok(CompositionInfo {
        duration: payload.duration,
        natural_size: payload.natural_size,
        is_playable: payload.is_playable,
        is_exportable: payload.is_exportable,
        is_readable: payload.is_readable,
        is_composable: payload.is_composable,
        url_asset_initialization_options: parse_optional_json_string(
            payload.url_asset_initialization_options_json,
        )?,
        track_ids: payload.track_ids,
        unused_track_id: payload.unused_track_id,
    })
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
    CMFormatDescription::from_raw(ptr).ok_or_else(|| missing_payload(context))
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
        let mut err_msg: *mut c_char = ptr::null_mut();
        let ptr = unsafe { ffi::av_composition_create_from_asset(asset.as_ptr(), &mut err_msg) };
        Self::from_raw(ptr)
            .ok_or_else(|| unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) })
    }

    pub fn from_file_path(path: impl AsRef<Path>) -> Result<Self, AVWriterError> {
        let asset = Asset::from_file_path(path)?;
        Self::from_asset(&asset)
    }

    pub fn snapshot(&self) -> Result<CompositionInfo, AVWriterError> {
        decode_composition_info(self.info_payload()?)
    }

    pub fn duration(&self) -> Result<Time, AVWriterError> {
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

    pub fn url_asset_initialization_options(&self) -> Result<Option<JsonValue>, AVWriterError> {
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
        let ptr = unsafe { ffi::av_composition_info_json(self.ptr) };
        parse_json_ptr(ptr, "composition info")
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

    pub fn time_range(&self) -> Result<TimeRange, AVWriterError> {
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

    pub fn min_frame_duration(&self) -> Result<Time, AVWriterError> {
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
        track_time: impl Into<Time>,
    ) -> Result<Option<CompositionTrackSegment>, AVWriterError> {
        let track_time = track_time.into();
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
        let media_characteristic_c = cstring_arg(media_characteristic, "media characteristic")?;
        Ok(unsafe {
            ffi::av_composition_track_has_media_characteristic(
                self.ptr,
                media_characteristic_c.as_ptr(),
            )
        })
    }

    pub fn sample_presentation_time_for_track_time(
        &self,
        track_time: impl Into<Time>,
    ) -> Result<Time, AVWriterError> {
        let track_time = track_time.into();
        let ptr = unsafe {
            ffi::av_composition_track_sample_presentation_time_json(
                self.ptr,
                time_value(&track_time),
                time_scale(&track_time),
                time_kind(&track_time),
            )
        };
        parse_json_ptr(ptr, "composition track sample presentation time")
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
        let ptr = unsafe { ffi::av_composition_track_info_json(self.ptr) };
        parse_json_ptr(ptr, "composition track info")
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
        source_time_range: TimeRange,
        target_time_range: TimeRange,
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
        source_time_range: TimeRange,
        target_time_range: TimeRange,
    ) -> Result<Self, AVWriterError> {
        Self::from_raw_url(
            url.as_ref(),
            false,
            track_id,
            source_time_range,
            target_time_range,
        )
    }

    pub fn empty(time_range: TimeRange) -> Result<Self, AVWriterError> {
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
        let ptr = unsafe { ffi::av_composition_track_segment_info_json(self.ptr) };
        parse_json_ptr(ptr, "composition track segment info")
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
        source_time_range: TimeRange,
        target_time_range: TimeRange,
    ) -> Result<Self, AVWriterError> {
        let url_c = cstring_arg(url, "segment source URL")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
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
                &mut err_msg,
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
        let ptr = unsafe { ffi::av_asset_track_segment_info_json(self.ptr) };
        parse_json_ptr(ptr, "asset track segment info")
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
        let ptr =
            unsafe { ffi::av_composition_track_format_description_replacement_info_json(self.ptr) };
        parse_json_ptr(ptr, "composition track format description replacement info")
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
