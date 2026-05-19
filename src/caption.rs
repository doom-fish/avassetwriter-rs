#![allow(
    clippy::derivable_impls,
    clippy::derive_partial_eq_without_eq,
    clippy::missing_const_for_fn,
    clippy::missing_errors_doc,
    clippy::semicolon_if_nothing_returned,
    clippy::unsafe_derive_deserialize
)]

use core::ffi::{c_char, c_void};
use core::ptr;
use std::ops::Range;

use serde::{Deserialize, Serialize};

use crate::bridge_support::{
    cstring_arg, parse_json_ptr, serialize_json, time_kind, time_scale, time_value,
};
use crate::error::{from_swift, AVWriterError};
use crate::ffi;
use crate::time::{Time, TimeRange};

fn missing_payload(what: &str) -> AVWriterError {
    AVWriterError::InvalidState(format!("swift bridge returned no {what} payload"))
}

/// Geometry unit for caption dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptionUnitsType {
    Unspecified,
    Cells,
    Percent,
}

impl Default for CaptionUnitsType {
    fn default() -> Self {
        Self::Unspecified
    }
}

/// A single caption dimension.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptionDimension {
    pub value: f64,
    #[serde(default)]
    pub units: CaptionUnitsType,
}

impl CaptionDimension {
    #[must_use]
    pub const fn new(value: f64, units: CaptionUnitsType) -> Self {
        Self { value, units }
    }
}

impl Default for CaptionDimension {
    fn default() -> Self {
        Self::new(0.0, CaptionUnitsType::Unspecified)
    }
}

/// A caption point.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CaptionPoint {
    pub x: CaptionDimension,
    pub y: CaptionDimension,
}

impl CaptionPoint {
    #[must_use]
    pub const fn new(x: CaptionDimension, y: CaptionDimension) -> Self {
        Self { x, y }
    }
}

/// A caption size.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CaptionSize {
    pub width: CaptionDimension,
    pub height: CaptionDimension,
}

impl CaptionSize {
    #[must_use]
    pub const fn new(width: CaptionDimension, height: CaptionDimension) -> Self {
        Self { width, height }
    }
}

/// Region line stacking alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptionRegionDisplayAlignment {
    Before,
    Center,
    After,
}

impl Default for CaptionRegionDisplayAlignment {
    fn default() -> Self {
        Self::Before
    }
}

/// Region writing mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptionRegionWritingMode {
    LeftToRightAndTopToBottom,
    TopToBottomAndRightToLeft,
}

impl Default for CaptionRegionWritingMode {
    fn default() -> Self {
        Self::LeftToRightAndTopToBottom
    }
}

/// Region scroll behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptionRegionScroll {
    None,
    RollUp,
}

impl Default for CaptionRegionScroll {
    fn default() -> Self {
        Self::None
    }
}

/// Safe Rust value model for `AVCaptionRegion`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptionRegion {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    #[serde(default)]
    pub origin: CaptionPoint,
    #[serde(default)]
    pub size: CaptionSize,
    #[serde(default)]
    pub scroll: CaptionRegionScroll,
    #[serde(default)]
    pub display_alignment: CaptionRegionDisplayAlignment,
    #[serde(default)]
    pub writing_mode: CaptionRegionWritingMode,
}

impl CaptionRegion {
    #[must_use]
    pub fn new(
        identifier: Option<String>,
        origin: CaptionPoint,
        size: CaptionSize,
        scroll: CaptionRegionScroll,
        display_alignment: CaptionRegionDisplayAlignment,
        writing_mode: CaptionRegionWritingMode,
    ) -> Self {
        Self {
            identifier,
            origin,
            size,
            scroll,
            display_alignment,
            writing_mode,
        }
    }

    pub fn apple_itt_top() -> Result<Self, AVWriterError> {
        Self::from_predefined("apple_itt_top")
    }

    pub fn apple_itt_bottom() -> Result<Self, AVWriterError> {
        Self::from_predefined("apple_itt_bottom")
    }

    pub fn apple_itt_left() -> Result<Self, AVWriterError> {
        Self::from_predefined("apple_itt_left")
    }

    pub fn apple_itt_right() -> Result<Self, AVWriterError> {
        Self::from_predefined("apple_itt_right")
    }

    pub fn sub_rip_text_bottom() -> Result<Self, AVWriterError> {
        Self::from_predefined("sub_rip_text_bottom")
    }

    fn from_predefined(kind: &str) -> Result<Self, AVWriterError> {
        let kind_c = cstring_arg(kind, "caption region kind")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let ptr = unsafe { ffi::av_caption_region_predefined_json(kind_c.as_ptr(), &mut err_msg) };
        if ptr.is_null() {
            return if err_msg.is_null() {
                Err(missing_payload("caption region"))
            } else {
                Err(unsafe { from_swift(ffi::status::INVALID_ARGUMENT, err_msg) })
            };
        }
        parse_json_ptr(ptr, "caption region")
    }
}

/// Mutable builder for caption regions.
#[derive(Debug, Clone, PartialEq)]
pub struct MutableCaptionRegion {
    region: CaptionRegion,
}

impl MutableCaptionRegion {
    #[must_use]
    pub fn new() -> Self {
        Self {
            region: CaptionRegion::new(
                None,
                CaptionPoint::default(),
                CaptionSize::default(),
                CaptionRegionScroll::None,
                CaptionRegionDisplayAlignment::Before,
                CaptionRegionWritingMode::LeftToRightAndTopToBottom,
            ),
        }
    }

    #[must_use]
    pub fn with_identifier(identifier: impl Into<String>) -> Self {
        let mut region = Self::new();
        region.region.identifier = Some(identifier.into());
        region
    }

    pub fn set_origin(&mut self, origin: CaptionPoint) {
        self.region.origin = origin;
    }

    pub fn set_size(&mut self, size: CaptionSize) {
        self.region.size = size;
    }

    pub fn set_scroll(&mut self, scroll: CaptionRegionScroll) {
        self.region.scroll = scroll;
    }

    pub fn set_display_alignment(&mut self, display_alignment: CaptionRegionDisplayAlignment) {
        self.region.display_alignment = display_alignment;
    }

    pub fn set_writing_mode(&mut self, writing_mode: CaptionRegionWritingMode) {
        self.region.writing_mode = writing_mode;
    }

    pub fn set_identifier(&mut self, identifier: Option<String>) {
        self.region.identifier = identifier;
    }

    #[must_use]
    pub const fn as_region(&self) -> &CaptionRegion {
        &self.region
    }

    #[must_use]
    pub fn into_region(self) -> CaptionRegion {
        self.region
    }
}

impl Default for MutableCaptionRegion {
    fn default() -> Self {
        Self::new()
    }
}

/// Caption text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptionTextAlignment {
    Start,
    End,
    Center,
    Left,
    Right,
}

/// Caption animation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptionAnimation {
    None,
    CharacterReveal,
}

impl Default for CaptionAnimation {
    fn default() -> Self {
        Self::None
    }
}

/// Ruby text position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptionRubyPosition {
    Before,
    After,
}

impl Default for CaptionRubyPosition {
    fn default() -> Self {
        Self::Before
    }
}

/// Ruby text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptionRubyAlignment {
    Start,
    Center,
    DistributeSpaceBetween,
    DistributeSpaceAround,
}

impl Default for CaptionRubyAlignment {
    fn default() -> Self {
        Self::DistributeSpaceBetween
    }
}

/// Rust value model for `AVCaptionRuby`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptionRuby {
    pub text: String,
    #[serde(default)]
    pub position: CaptionRubyPosition,
    #[serde(default)]
    pub alignment: CaptionRubyAlignment,
}

impl CaptionRuby {
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            position: CaptionRubyPosition::Before,
            alignment: CaptionRubyAlignment::DistributeSpaceBetween,
        }
    }

    #[must_use]
    pub fn with_position_alignment(
        text: impl Into<String>,
        position: CaptionRubyPosition,
        alignment: CaptionRubyAlignment,
    ) -> Self {
        Self {
            text: text.into(),
            position,
            alignment,
        }
    }
}

/// A ruby annotation span expressed in UTF-16 offsets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptionRubySpan {
    pub start: usize,
    pub length: usize,
    pub ruby: CaptionRuby,
}

impl CaptionRubySpan {
    #[must_use]
    pub fn new(range: Range<usize>, ruby: CaptionRuby) -> Self {
        Self {
            start: range.start,
            length: range.end.saturating_sub(range.start),
            ruby,
        }
    }

    #[must_use]
    pub fn range(&self) -> Range<usize> {
        self.start..self.start.saturating_add(self.length)
    }
}

/// Rust value model for `AVCaption`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Caption {
    pub text: String,
    pub time_range: TimeRange,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<CaptionRegion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_alignment: Option<CaptionTextAlignment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub animation: Option<CaptionAnimation>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ruby_spans: Vec<CaptionRubySpan>,
}

impl Caption {
    #[must_use]
    pub fn new(text: impl Into<String>, time_range: TimeRange) -> Self {
        Self {
            text: text.into(),
            time_range,
            region: None,
            text_alignment: None,
            animation: None,
            ruby_spans: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_region(mut self, region: CaptionRegion) -> Self {
        self.region = Some(region);
        self
    }

    #[must_use]
    pub fn with_text_alignment(mut self, text_alignment: CaptionTextAlignment) -> Self {
        self.text_alignment = Some(text_alignment);
        self
    }

    #[must_use]
    pub fn with_animation(mut self, animation: CaptionAnimation) -> Self {
        self.animation = Some(animation);
        self
    }

    #[must_use]
    pub fn with_ruby(mut self, range: Range<usize>, ruby: CaptionRuby) -> Self {
        self.ruby_spans.push(CaptionRubySpan::new(range, ruby));
        self
    }
}

/// Mutable builder for captions.
#[derive(Debug, Clone, PartialEq)]
pub struct MutableCaption {
    caption: Caption,
}

impl MutableCaption {
    #[must_use]
    pub fn new(text: impl Into<String>, time_range: TimeRange) -> Self {
        Self {
            caption: Caption::new(text, time_range),
        }
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.caption.text = text.into();
    }

    pub fn set_time_range(&mut self, time_range: TimeRange) {
        self.caption.time_range = time_range;
    }

    pub fn set_region(&mut self, region: Option<CaptionRegion>) {
        self.caption.region = region;
    }

    pub fn set_text_alignment(&mut self, text_alignment: Option<CaptionTextAlignment>) {
        self.caption.text_alignment = text_alignment;
    }

    pub fn set_animation(&mut self, animation: Option<CaptionAnimation>) {
        self.caption.animation = animation;
    }

    pub fn set_ruby(&mut self, range: Range<usize>, ruby: CaptionRuby) {
        self.caption
            .ruby_spans
            .push(CaptionRubySpan::new(range, ruby));
    }

    pub fn clear_ruby(&mut self) {
        self.caption.ruby_spans.clear();
    }

    #[must_use]
    pub const fn as_caption(&self) -> &Caption {
        &self.caption
    }

    #[must_use]
    pub fn into_caption(self) -> Caption {
        self.caption
    }
}

/// A group of captions sharing one enclosing time range.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptionGroup {
    pub captions: Vec<Caption>,
    pub time_range: TimeRange,
}

impl CaptionGroup {
    #[must_use]
    pub fn new(captions: Vec<Caption>, time_range: TimeRange) -> Self {
        Self {
            captions,
            time_range,
        }
    }

    #[must_use]
    pub fn empty(time_range: TimeRange) -> Self {
        Self {
            captions: Vec::new(),
            time_range,
        }
    }
}

/// Bounds used by `CaptionRenderer`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptionBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl CaptionBounds {
    #[must_use]
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// A scene boundary returned by `CaptionRenderer`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptionRendererScene {
    pub time_range: TimeRange,
    pub has_active_captions: bool,
    pub needs_periodic_refresh: bool,
}

/// Conversion settings built from `AVCaptionSettings` keys.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptionConversionSettings {
    pub media_subtype: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub time_code_frame_duration: Option<Time>,
    #[serde(default)]
    pub use_drop_frame_time_code: bool,
}

impl CaptionConversionSettings {
    #[must_use]
    pub fn new(media_subtype: impl Into<String>) -> Self {
        Self {
            media_subtype: media_subtype.into(),
            time_code_frame_duration: None,
            use_drop_frame_time_code: false,
        }
    }

    #[must_use]
    pub fn cea608() -> Self {
        Self::new("c608")
    }
}

/// Status values exposed by `AVCaptionConversionValidator`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptionConversionValidatorStatus {
    Unknown,
    Validating,
    Completed,
    Stopped,
}

impl CaptionConversionValidatorStatus {
    fn from_raw(raw: i32) -> Self {
        match raw {
            1 => Self::Validating,
            2 => Self::Completed,
            3 => Self::Stopped,
            _ => Self::Unknown,
        }
    }
}

/// A generic caption conversion adjustment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptionConversionAdjustment {
    pub adjustment_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_time_offset: Option<Time>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_offset: Option<Time>,
}

impl CaptionConversionAdjustment {
    #[must_use]
    pub fn as_time_range_adjustment(&self) -> Option<CaptionConversionTimeRangeAdjustment> {
        Some(CaptionConversionTimeRangeAdjustment {
            adjustment_type: self.adjustment_type.clone(),
            start_time_offset: self.start_time_offset?,
            duration_offset: self.duration_offset?,
        })
    }
}

/// A time-range adjustment suggestion returned by the validator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptionConversionTimeRangeAdjustment {
    pub adjustment_type: String,
    pub start_time_offset: Time,
    pub duration_offset: Time,
}

/// One validator warning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptionConversionWarning {
    pub warning_type: String,
    pub range_of_captions_start: usize,
    pub range_of_captions_length: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adjustment: Option<CaptionConversionAdjustment>,
}

impl CaptionConversionWarning {
    #[must_use]
    pub fn range_of_captions(&self) -> Range<usize> {
        self.range_of_captions_start
            ..self
                .range_of_captions_start
                .saturating_add(self.range_of_captions_length)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CaptionRendererInfoPayload {
    captions: Vec<Caption>,
    bounds: CaptionBounds,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CaptionConversionValidatorInfoPayload {
    status: i32,
    captions: Vec<Caption>,
    time_range: TimeRange,
    warnings: Vec<CaptionConversionWarning>,
}

/// Safe wrapper around `AVCaptionGrouper`.
pub struct CaptionGrouper {
    ptr: *mut c_void,
}

unsafe impl Send for CaptionGrouper {}

impl CaptionGrouper {
    pub fn new() -> Result<Self, AVWriterError> {
        let ptr = unsafe { ffi::av_caption_grouper_create() };
        Self::from_raw(ptr).ok_or_else(|| missing_payload("caption grouper"))
    }

    pub fn add_caption(&self, caption: &Caption) -> Result<(), AVWriterError> {
        let payload = serialize_json(caption)?;
        let payload_c = cstring_arg(&payload, "caption json")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_caption_grouper_add_caption_json(self.ptr, payload_c.as_ptr(), &mut err_msg)
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn flush_added_captions_up_to_time(
        &self,
        up_to_time: impl Into<Time>,
    ) -> Result<Vec<CaptionGroup>, AVWriterError> {
        let up_to_time = up_to_time.into();
        let mut err_msg: *mut c_char = ptr::null_mut();
        let ptr = unsafe {
            ffi::av_caption_grouper_flush_groups_json(
                self.ptr,
                time_value(&up_to_time),
                time_scale(&up_to_time),
                time_kind(&up_to_time),
                &mut err_msg,
            )
        };
        if ptr.is_null() {
            return if err_msg.is_null() {
                Err(missing_payload("caption groups"))
            } else {
                Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) })
            };
        }
        parse_json_ptr(ptr, "caption groups")
    }

    fn from_raw(ptr: *mut c_void) -> Option<Self> {
        (!ptr.is_null()).then_some(Self { ptr })
    }
}

impl Drop for CaptionGrouper {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ffi::av_caption_grouper_release(self.ptr) };
        }
    }
}

impl core::fmt::Debug for CaptionGrouper {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CaptionGrouper")
            .field("ptr", &self.ptr)
            .finish()
    }
}

/// Safe wrapper around `AVCaptionFormatConformer`.
pub struct CaptionFormatConformer {
    ptr: *mut c_void,
}

unsafe impl Send for CaptionFormatConformer {}

impl CaptionFormatConformer {
    pub fn new(settings: &CaptionConversionSettings) -> Result<Self, AVWriterError> {
        let payload = serialize_json(settings)?;
        let payload_c = cstring_arg(&payload, "caption conversion settings json")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let ptr =
            unsafe { ffi::av_caption_format_conformer_create(payload_c.as_ptr(), &mut err_msg) };
        if ptr.is_null() {
            return if err_msg.is_null() {
                Err(missing_payload("caption format conformer"))
            } else {
                Err(unsafe { from_swift(ffi::status::INVALID_ARGUMENT, err_msg) })
            };
        }
        Ok(Self { ptr })
    }

    #[must_use]
    pub fn conforms_captions_to_time_range(&self) -> bool {
        unsafe { ffi::av_caption_format_conformer_conforms_captions_to_time_range(self.ptr) }
    }

    pub fn set_conforms_captions_to_time_range(&self, conforms: bool) {
        unsafe {
            ffi::av_caption_format_conformer_set_conforms_captions_to_time_range(self.ptr, conforms)
        };
    }

    pub fn conformed_caption(&self, caption: &Caption) -> Result<Caption, AVWriterError> {
        let payload = serialize_json(caption)?;
        let payload_c = cstring_arg(&payload, "caption json")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let ptr = unsafe {
            ffi::av_caption_format_conformer_conformed_caption_json(
                self.ptr,
                payload_c.as_ptr(),
                &mut err_msg,
            )
        };
        if ptr.is_null() {
            return if err_msg.is_null() {
                Err(missing_payload("conformed caption"))
            } else {
                Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) })
            };
        }
        parse_json_ptr(ptr, "conformed caption")
    }
}

impl Drop for CaptionFormatConformer {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ffi::av_caption_format_conformer_release(self.ptr) };
        }
    }
}

impl core::fmt::Debug for CaptionFormatConformer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CaptionFormatConformer")
            .field("ptr", &self.ptr)
            .finish()
    }
}

/// Safe wrapper around `AVCaptionRenderer`.
pub struct CaptionRenderer {
    ptr: *mut c_void,
}

unsafe impl Send for CaptionRenderer {}

impl CaptionRenderer {
    pub fn new() -> Result<Self, AVWriterError> {
        let ptr = unsafe { ffi::av_caption_renderer_create() };
        (!ptr.is_null())
            .then_some(Self { ptr })
            .ok_or_else(|| missing_payload("caption renderer"))
    }

    pub fn captions(&self) -> Result<Vec<Caption>, AVWriterError> {
        Ok(self.info()?.captions)
    }

    pub fn set_captions(&self, captions: &[Caption]) -> Result<(), AVWriterError> {
        let payload = serialize_json(captions)?;
        let payload_c = cstring_arg(&payload, "captions json")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_caption_renderer_set_captions_json(self.ptr, payload_c.as_ptr(), &mut err_msg)
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn bounds(&self) -> Result<CaptionBounds, AVWriterError> {
        Ok(self.info()?.bounds)
    }

    pub fn set_bounds(&self, bounds: CaptionBounds) {
        unsafe {
            ffi::av_caption_renderer_set_bounds(
                self.ptr,
                bounds.x,
                bounds.y,
                bounds.width,
                bounds.height,
            );
        };
    }

    pub fn caption_scene_changes(
        &self,
        considered_time_range: TimeRange,
    ) -> Result<Vec<CaptionRendererScene>, AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let ptr = unsafe {
            ffi::av_caption_renderer_scene_changes_json(
                self.ptr,
                time_value(&considered_time_range.start),
                time_scale(&considered_time_range.start),
                time_kind(&considered_time_range.start),
                time_value(&considered_time_range.duration),
                time_scale(&considered_time_range.duration),
                time_kind(&considered_time_range.duration),
                &mut err_msg,
            )
        };
        if ptr.is_null() {
            return if err_msg.is_null() {
                Err(missing_payload("caption renderer scenes"))
            } else {
                Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) })
            };
        }
        parse_json_ptr(ptr, "caption renderer scenes")
    }

    fn info(&self) -> Result<CaptionRendererInfoPayload, AVWriterError> {
        let ptr = unsafe { ffi::av_caption_renderer_info_json(self.ptr) };
        parse_json_ptr(ptr, "caption renderer info")
    }
}

impl Drop for CaptionRenderer {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ffi::av_caption_renderer_release(self.ptr) };
        }
    }
}

impl core::fmt::Debug for CaptionRenderer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CaptionRenderer")
            .field("ptr", &self.ptr)
            .finish()
    }
}

/// Safe wrapper around `AVCaptionConversionValidator`.
pub struct CaptionConversionValidator {
    ptr: *mut c_void,
}

unsafe impl Send for CaptionConversionValidator {}

impl CaptionConversionValidator {
    pub fn new(
        captions: &[Caption],
        time_range: TimeRange,
        settings: &CaptionConversionSettings,
    ) -> Result<Self, AVWriterError> {
        let captions_json = serialize_json(captions)?;
        let captions_c = cstring_arg(&captions_json, "captions json")?;
        let settings_json = serialize_json(settings)?;
        let settings_c = cstring_arg(&settings_json, "caption conversion settings json")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let ptr = unsafe {
            ffi::av_caption_conversion_validator_create(
                captions_c.as_ptr(),
                time_value(&time_range.start),
                time_scale(&time_range.start),
                time_kind(&time_range.start),
                time_value(&time_range.duration),
                time_scale(&time_range.duration),
                time_kind(&time_range.duration),
                settings_c.as_ptr(),
                &mut err_msg,
            )
        };
        if ptr.is_null() {
            return if err_msg.is_null() {
                Err(missing_payload("caption conversion validator"))
            } else {
                Err(unsafe { from_swift(ffi::status::INVALID_ARGUMENT, err_msg) })
            };
        }
        Ok(Self { ptr })
    }

    #[must_use]
    pub fn status(&self) -> CaptionConversionValidatorStatus {
        CaptionConversionValidatorStatus::from_raw(self.info().map_or(0, |info| info.status))
    }

    pub fn captions(&self) -> Result<Vec<Caption>, AVWriterError> {
        Ok(self.info()?.captions)
    }

    pub fn time_range(&self) -> Result<TimeRange, AVWriterError> {
        Ok(self.info()?.time_range)
    }

    pub fn warnings(&self) -> Result<Vec<CaptionConversionWarning>, AVWriterError> {
        Ok(self.info()?.warnings)
    }

    pub fn validate(&self) -> Result<Vec<CaptionConversionWarning>, AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status =
            unsafe { ffi::av_caption_conversion_validator_validate(self.ptr, &mut err_msg) };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        self.warnings()
    }

    pub fn stop_validating(&self) {
        unsafe { ffi::av_caption_conversion_validator_stop_validating(self.ptr) };
    }

    fn info(&self) -> Result<CaptionConversionValidatorInfoPayload, AVWriterError> {
        let ptr = unsafe { ffi::av_caption_conversion_validator_info_json(self.ptr) };
        parse_json_ptr(ptr, "caption conversion validator info")
    }
}

impl Drop for CaptionConversionValidator {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ffi::av_caption_conversion_validator_release(self.ptr) };
        }
    }
}

impl core::fmt::Debug for CaptionConversionValidator {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CaptionConversionValidator")
            .field("ptr", &self.ptr)
            .finish()
    }
}
