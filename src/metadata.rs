#![allow(
    clippy::doc_markdown,
    clippy::missing_const_for_fn,
    clippy::missing_errors_doc,
    clippy::missing_fields_in_debug,
    clippy::must_use_candidate,
    clippy::needless_pass_by_value,
    clippy::too_many_lines
)]

use core::cell::Cell;
use core::ffi::{c_char, c_void};
use core::ptr;

use doom_fish_utils::panic_safe::catch_user_panic;
use serde::{Deserialize, Serialize};

use crate::bridge_support::{cstring_arg, parse_json_ptr, serialize_json};
use crate::error::{from_swift, AVWriterError};
use crate::ffi;
use crate::media_processing::MetadataItemFilter;
use crate::time::TimeRange;

/// A metadata value supported by the AVFoundation bridge payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
#[non_exhaustive]
pub enum MetadataValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Data(Vec<u8>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MetadataItemPayload {
    pub identifier: String,
    pub value: MetadataValue,
    pub data_type: Option<String>,
    pub extended_language_tag: Option<String>,
    pub locale_identifier: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MetadataGroupPayload {
    pub items: Vec<MetadataItemPayload>,
    pub classifying_label: Option<String>,
    pub unique_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TimedMetadataGroupPayload {
    pub items: Vec<MetadataItemPayload>,
    pub time_range: TimeRange,
    pub classifying_label: Option<String>,
    pub unique_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DateRangeMetadataGroupPayload {
    pub items: Vec<MetadataItemPayload>,
    pub start_date: String,
    pub end_date: Option<String>,
    pub classifying_label: Option<String>,
    pub unique_id: Option<String>,
}

/// A CoreMedia metadata specification used to build a metadata-track format hint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetadataSpecification {
    pub identifier: String,
    pub data_type: String,
    pub extended_language_tag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KeyLoadStatusPayload {
    pub key: String,
    pub status: i32,
    pub error_message: Option<String>,
}

/// Per-key loading state returned by `AVAsynchronousKeyValueLoading`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum KeyValueStatus {
    Unknown,
    Loading,
    Loaded,
    Failed,
    Cancelled,
}

impl KeyValueStatus {
    #[must_use]
    pub const fn from_raw(raw: i32) -> Self {
        match raw {
            1 => Self::Loading,
            2 => Self::Loaded,
            3 => Self::Failed,
            4 => Self::Cancelled,
            _ => Self::Unknown,
        }
    }
}

/// Result for a single asynchronously-loaded key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyLoadStatus {
    pub key: String,
    pub status: KeyValueStatus,
    pub error_message: Option<String>,
}

/// Safe wrapper around `AVMetadataItem`.
pub struct MetadataItem {
    identifier: String,
    value: MetadataValue,
    data_type: Option<String>,
    extended_language_tag: Option<String>,
    locale_identifier: Option<String>,
    ptr: Cell<*mut c_void>,
}

// SAFETY: `MetadataItem` lazily materializes an ARC-retained Objective-C object.
// Ownership can be moved across threads, but concurrent shared access is not
// guaranteed by AVFoundation, so `Sync` is intentionally not implemented.
unsafe impl Send for MetadataItem {}

impl Clone for MetadataItem {
    fn clone(&self) -> Self {
        Self::from_payload(self.payload())
    }
}

impl PartialEq for MetadataItem {
    fn eq(&self, other: &Self) -> bool {
        self.payload() == other.payload()
    }
}

impl core::fmt::Debug for MetadataItem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MetadataItem")
            .field("identifier", &self.identifier)
            .field("value", &self.value)
            .field("data_type", &self.data_type)
            .field("extended_language_tag", &self.extended_language_tag)
            .field("locale_identifier", &self.locale_identifier)
            .finish()
    }
}

impl Drop for MetadataItem {
    fn drop(&mut self) {
        self.release_ptr();
    }
}

impl MetadataItem {
    #[must_use]
    pub fn new(identifier: impl Into<String>, value: MetadataValue) -> Self {
        Self {
            identifier: identifier.into(),
            value,
            data_type: None,
            extended_language_tag: None,
            locale_identifier: None,
            ptr: Cell::new(ptr::null_mut()),
        }
    }

    #[must_use]
    pub fn string(identifier: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(identifier, MetadataValue::String(value.into()))
    }

    #[must_use]
    pub fn integer(identifier: impl Into<String>, value: i64) -> Self {
        Self::new(identifier, MetadataValue::Integer(value))
    }

    #[must_use]
    pub fn float(identifier: impl Into<String>, value: f64) -> Self {
        Self::new(identifier, MetadataValue::Float(value))
    }

    #[must_use]
    pub fn boolean(identifier: impl Into<String>, value: bool) -> Self {
        Self::new(identifier, MetadataValue::Boolean(value))
    }

    #[must_use]
    pub fn data(identifier: impl Into<String>, value: Vec<u8>) -> Self {
        Self::new(identifier, MetadataValue::Data(value))
    }

    #[must_use]
    pub fn with_data_type(mut self, data_type: impl Into<String>) -> Self {
        self.release_ptr();
        self.data_type = Some(data_type.into());
        self
    }

    #[must_use]
    pub fn with_extended_language_tag(mut self, tag: impl Into<String>) -> Self {
        self.release_ptr();
        self.extended_language_tag = Some(tag.into());
        self
    }

    #[must_use]
    pub fn with_locale_identifier(mut self, identifier: impl Into<String>) -> Self {
        self.release_ptr();
        self.locale_identifier = Some(identifier.into());
        self
    }

    #[must_use]
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    pub fn value(&self) -> Result<MetadataValue, AVWriterError> {
        Ok(self.current_payload()?.value)
    }

    #[must_use]
    pub fn data_type(&self) -> Option<&str> {
        self.data_type.as_deref()
    }

    #[must_use]
    pub fn extended_language_tag(&self) -> Option<&str> {
        self.extended_language_tag.as_deref()
    }

    #[must_use]
    pub fn locale_identifier(&self) -> Option<&str> {
        self.locale_identifier.as_deref()
    }

    pub fn status_of_value(&self, key: &str) -> Result<KeyValueStatus, AVWriterError> {
        let key_c = cstring_arg(key, "metadata item key")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let raw = unsafe {
            ffi::av_metadata_item_status_of_value(self.ensure_ptr()?, key_c.as_ptr(), &mut err_msg)
        };
        if raw < 0 {
            return Err(unsafe { from_swift(raw, err_msg) });
        }
        Ok(KeyValueStatus::from_raw(raw))
    }

    pub fn load_values_asynchronously<I, S>(
        &self,
        keys: I,
    ) -> Result<Vec<KeyLoadStatus>, AVWriterError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let keys = keys
            .into_iter()
            .map(|key| key.as_ref().to_owned())
            .collect::<Vec<_>>();
        let payload = serialize_json(&keys)?;
        let payload_c = cstring_arg(&payload, "metadata item keys json")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let statuses_ptr = unsafe {
            ffi::av_metadata_item_load_values_json(
                self.ensure_ptr()?,
                payload_c.as_ptr(),
                30,
                &mut err_msg,
            )
        };
        if statuses_ptr.is_null() {
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        let raw_statuses: Vec<KeyLoadStatusPayload> =
            parse_json_ptr(statuses_ptr, "metadata item key statuses")?;
        Ok(raw_statuses
            .into_iter()
            .map(|status| KeyLoadStatus {
                key: status.key,
                status: KeyValueStatus::from_raw(status.status),
                error_message: status.error_message,
            })
            .collect())
    }

    pub fn with_lazy_value_loader<F>(base: &Self, callback: F) -> Result<Self, AVWriterError>
    where
        F: Fn(MetadataItemValueRequest) + Send + 'static,
    {
        let payload = serialize_json(&base.payload())?;
        let payload_c = cstring_arg(&payload, "metadata item base json")?;
        let state = Box::new(MetadataValueLoaderState {
            callback: Box::new(callback),
        });
        let userdata = Box::into_raw(state).cast::<c_void>();
        let mut err_msg: *mut c_char = ptr::null_mut();
        let ptr = unsafe {
            ffi::av_metadata_item_create_lazy_json(
                payload_c.as_ptr(),
                Some(metadata_value_request_trampoline),
                userdata,
                Some(metadata_value_request_drop),
                &mut err_msg,
            )
        };
        if ptr.is_null() {
            unsafe { metadata_value_request_drop(userdata) };
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        let item = base.clone();
        item.ptr.set(ptr);
        Ok(item)
    }

    pub fn filtered_and_sorted_according_to_preferred_languages<I, S>(
        items: &[Self],
        preferred_languages: I,
    ) -> Result<Vec<Self>, AVWriterError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let items_payload = items.iter().map(Self::payload).collect::<Vec<_>>();
        let languages_payload = preferred_languages
            .into_iter()
            .map(|value| value.as_ref().to_owned())
            .collect::<Vec<_>>();
        let items_json = serialize_json(&items_payload)?;
        let languages_json = serialize_json(&languages_payload)?;
        let items_c = cstring_arg(&items_json, "metadata items json")?;
        let languages_c = cstring_arg(&languages_json, "preferred languages json")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let ptr = unsafe {
            ffi::av_metadata_item_filter_preferred_languages_json(
                items_c.as_ptr(),
                languages_c.as_ptr(),
                &mut err_msg,
            )
        };
        if ptr.is_null() {
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        let payloads: Vec<MetadataItemPayload> =
            parse_json_ptr(ptr, "preferred-language metadata items")?;
        Ok(payloads.into_iter().map(Self::from_payload).collect())
    }

    pub fn filtered_by_identifier(
        items: &[Self],
        identifier: impl AsRef<str>,
    ) -> Result<Vec<Self>, AVWriterError> {
        let items_payload = items.iter().map(Self::payload).collect::<Vec<_>>();
        let items_json = serialize_json(&items_payload)?;
        let items_c = cstring_arg(&items_json, "metadata items json")?;
        let identifier_c = cstring_arg(identifier.as_ref(), "metadata identifier")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let ptr = unsafe {
            ffi::av_metadata_item_filter_identifier_json(
                items_c.as_ptr(),
                identifier_c.as_ptr(),
                &mut err_msg,
            )
        };
        if ptr.is_null() {
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        let payloads: Vec<MetadataItemPayload> =
            parse_json_ptr(ptr, "identifier-filtered metadata items")?;
        Ok(payloads.into_iter().map(Self::from_payload).collect())
    }

    pub fn filtered_by_metadata_item_filter(
        items: &[Self],
        filter: &MetadataItemFilter,
    ) -> Result<Vec<Self>, AVWriterError> {
        let items_payload = items.iter().map(Self::payload).collect::<Vec<_>>();
        let items_json = serialize_json(&items_payload)?;
        let items_c = cstring_arg(&items_json, "metadata items json")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let ptr = unsafe {
            ffi::av_metadata_item_filter_metadata_item_filter_json(
                items_c.as_ptr(),
                filter.as_ptr(),
                &mut err_msg,
            )
        };
        if ptr.is_null() {
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        let payloads: Vec<MetadataItemPayload> = parse_json_ptr(ptr, "metadata-filtered items")?;
        Ok(payloads.into_iter().map(Self::from_payload).collect())
    }

    pub(crate) fn from_payload(payload: MetadataItemPayload) -> Self {
        Self {
            identifier: payload.identifier,
            value: payload.value,
            data_type: payload.data_type,
            extended_language_tag: payload.extended_language_tag,
            locale_identifier: payload.locale_identifier,
            ptr: Cell::new(ptr::null_mut()),
        }
    }

    pub(crate) fn payload(&self) -> MetadataItemPayload {
        self.current_payload()
            .unwrap_or_else(|_| MetadataItemPayload {
                identifier: self.identifier.clone(),
                value: self.value.clone(),
                data_type: self.data_type.clone(),
                extended_language_tag: self.extended_language_tag.clone(),
                locale_identifier: self.locale_identifier.clone(),
            })
    }

    fn ensure_ptr(&self) -> Result<*mut c_void, AVWriterError> {
        let current = self.ptr.get();
        if !current.is_null() {
            return Ok(current);
        }
        let payload = serialize_json(&MetadataItemPayload {
            identifier: self.identifier.clone(),
            value: self.value.clone(),
            data_type: self.data_type.clone(),
            extended_language_tag: self.extended_language_tag.clone(),
            locale_identifier: self.locale_identifier.clone(),
        })?;
        let payload_c = cstring_arg(&payload, "metadata item json")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let ptr = unsafe { ffi::av_metadata_item_create_json(payload_c.as_ptr(), &mut err_msg) };
        if ptr.is_null() {
            return Err(unsafe { from_swift(ffi::status::INVALID_ARGUMENT, err_msg) });
        }
        self.ptr.set(ptr);
        Ok(ptr)
    }

    fn current_payload(&self) -> Result<MetadataItemPayload, AVWriterError> {
        let current = self.ptr.get();
        if current.is_null() {
            return Ok(MetadataItemPayload {
                identifier: self.identifier.clone(),
                value: self.value.clone(),
                data_type: self.data_type.clone(),
                extended_language_tag: self.extended_language_tag.clone(),
                locale_identifier: self.locale_identifier.clone(),
            });
        }
        let mut err_msg: *mut c_char = ptr::null_mut();
        let ptr = unsafe { ffi::av_metadata_item_info_json(current, &mut err_msg) };
        if ptr.is_null() {
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        parse_json_ptr(ptr, "metadata item info")
    }

    fn release_ptr(&self) {
        let current = self.ptr.replace(ptr::null_mut());
        if !current.is_null() {
            unsafe { ffi::av_metadata_item_release(current) };
        }
    }
}

/// Common superclass readback for `AVMetadataGroup` subclasses.
#[derive(Debug, Clone, PartialEq)]
pub struct MetadataGroup {
    items: Vec<MetadataItem>,
    classifying_label: Option<String>,
    unique_id: Option<String>,
}

impl MetadataGroup {
    #[must_use]
    pub fn items(&self) -> &[MetadataItem] {
        &self.items
    }

    #[must_use]
    pub fn classifying_label(&self) -> Option<&str> {
        self.classifying_label.as_deref()
    }

    #[must_use]
    pub fn unique_id(&self) -> Option<&str> {
        self.unique_id.as_deref()
    }

    #[allow(dead_code)]
    fn from_payload(payload: MetadataGroupPayload) -> Self {
        Self {
            items: payload
                .items
                .into_iter()
                .map(MetadataItem::from_payload)
                .collect(),
            classifying_label: payload.classifying_label,
            unique_id: payload.unique_id,
        }
    }
}

/// Safe wrapper around `AVTimedMetadataGroup`.
pub struct TimedMetadataGroup {
    items: Vec<MetadataItem>,
    time_range: TimeRange,
    classifying_label: Option<String>,
    unique_id: Option<String>,
    ptr: Cell<*mut c_void>,
}

// SAFETY: `TimedMetadataGroup` lazily materializes an ARC-retained Objective-C
// object whose ownership may move across threads.
unsafe impl Send for TimedMetadataGroup {}

impl Clone for TimedMetadataGroup {
    fn clone(&self) -> Self {
        Self::from_payload(self.payload())
    }
}

impl PartialEq for TimedMetadataGroup {
    fn eq(&self, other: &Self) -> bool {
        self.payload() == other.payload()
    }
}

impl core::fmt::Debug for TimedMetadataGroup {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TimedMetadataGroup")
            .field("items", &self.items)
            .field("time_range", &self.time_range)
            .field("classifying_label", &self.classifying_label)
            .field("unique_id", &self.unique_id)
            .finish()
    }
}

impl Drop for TimedMetadataGroup {
    fn drop(&mut self) {
        let ptr = self.ptr.replace(ptr::null_mut());
        if !ptr.is_null() {
            unsafe { ffi::av_timed_metadata_group_release(ptr) };
        }
    }
}

impl TimedMetadataGroup {
    #[must_use]
    pub fn new(items: Vec<MetadataItem>, time_range: TimeRange) -> Self {
        Self {
            items,
            time_range,
            classifying_label: None,
            unique_id: None,
            ptr: Cell::new(ptr::null_mut()),
        }
    }

    #[must_use]
    pub fn items(&self) -> &[MetadataItem] {
        &self.items
    }

    #[must_use]
    pub const fn time_range(&self) -> TimeRange {
        self.time_range
    }

    #[must_use]
    pub fn as_metadata_group(&self) -> MetadataGroup {
        MetadataGroup {
            items: self.items.clone(),
            classifying_label: self.classifying_label.clone(),
            unique_id: self.unique_id.clone(),
        }
    }

    pub(crate) fn from_payload(payload: TimedMetadataGroupPayload) -> Self {
        Self {
            items: payload
                .items
                .into_iter()
                .map(MetadataItem::from_payload)
                .collect(),
            time_range: payload.time_range,
            classifying_label: payload.classifying_label,
            unique_id: payload.unique_id,
            ptr: Cell::new(ptr::null_mut()),
        }
    }

    pub(crate) fn payload(&self) -> TimedMetadataGroupPayload {
        TimedMetadataGroupPayload {
            items: self.items.iter().map(MetadataItem::payload).collect(),
            time_range: self.time_range,
            classifying_label: self.classifying_label.clone(),
            unique_id: self.unique_id.clone(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn as_ptr(&self) -> Result<*mut c_void, AVWriterError> {
        let current = self.ptr.get();
        if !current.is_null() {
            return Ok(current);
        }
        let payload = serialize_json(&self.payload())?;
        let payload_c = cstring_arg(&payload, "timed metadata group json")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let ptr =
            unsafe { ffi::av_timed_metadata_group_create_json(payload_c.as_ptr(), &mut err_msg) };
        if ptr.is_null() {
            return Err(unsafe { from_swift(ffi::status::INVALID_ARGUMENT, err_msg) });
        }
        self.ptr.set(ptr);
        Ok(ptr)
    }
}

/// Safe wrapper around `AVDateRangeMetadataGroup`.
pub struct DateRangeMetadataGroup {
    items: Vec<MetadataItem>,
    start_date: String,
    end_date: Option<String>,
    classifying_label: Option<String>,
    unique_id: Option<String>,
    ptr: Cell<*mut c_void>,
}

// SAFETY: `DateRangeMetadataGroup` lazily materializes an ARC-retained
// Objective-C object whose ownership may move across threads.
unsafe impl Send for DateRangeMetadataGroup {}

impl Clone for DateRangeMetadataGroup {
    fn clone(&self) -> Self {
        Self::from_payload(self.payload())
    }
}

impl PartialEq for DateRangeMetadataGroup {
    fn eq(&self, other: &Self) -> bool {
        self.payload() == other.payload()
    }
}

impl core::fmt::Debug for DateRangeMetadataGroup {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DateRangeMetadataGroup")
            .field("items", &self.items)
            .field("start_date", &self.start_date)
            .field("end_date", &self.end_date)
            .field("classifying_label", &self.classifying_label)
            .field("unique_id", &self.unique_id)
            .finish()
    }
}

impl Drop for DateRangeMetadataGroup {
    fn drop(&mut self) {
        let ptr = self.ptr.replace(ptr::null_mut());
        if !ptr.is_null() {
            unsafe { ffi::av_date_range_metadata_group_release(ptr) };
        }
    }
}

impl DateRangeMetadataGroup {
    #[must_use]
    pub fn new(
        items: Vec<MetadataItem>,
        start_date: impl Into<String>,
        end_date: Option<String>,
    ) -> Self {
        Self {
            items,
            start_date: start_date.into(),
            end_date,
            classifying_label: None,
            unique_id: None,
            ptr: Cell::new(ptr::null_mut()),
        }
    }

    #[must_use]
    pub fn items(&self) -> &[MetadataItem] {
        &self.items
    }

    #[must_use]
    pub fn start_date(&self) -> &str {
        &self.start_date
    }

    #[must_use]
    pub fn end_date(&self) -> Option<&str> {
        self.end_date.as_deref()
    }

    #[must_use]
    pub fn as_metadata_group(&self) -> MetadataGroup {
        MetadataGroup {
            items: self.items.clone(),
            classifying_label: self.classifying_label.clone(),
            unique_id: self.unique_id.clone(),
        }
    }

    pub(crate) fn from_payload(payload: DateRangeMetadataGroupPayload) -> Self {
        Self {
            items: payload
                .items
                .into_iter()
                .map(MetadataItem::from_payload)
                .collect(),
            start_date: payload.start_date,
            end_date: payload.end_date,
            classifying_label: payload.classifying_label,
            unique_id: payload.unique_id,
            ptr: Cell::new(ptr::null_mut()),
        }
    }

    pub(crate) fn payload(&self) -> DateRangeMetadataGroupPayload {
        DateRangeMetadataGroupPayload {
            items: self.items.iter().map(MetadataItem::payload).collect(),
            start_date: self.start_date.clone(),
            end_date: self.end_date.clone(),
            classifying_label: self.classifying_label.clone(),
            unique_id: self.unique_id.clone(),
        }
    }
}

/// Safe wrapper around `AVMutableDateRangeMetadataGroup`.
pub struct MutableDateRangeMetadataGroup {
    items: Vec<MetadataItem>,
    start_date: String,
    end_date: Option<String>,
    classifying_label: Option<String>,
    unique_id: Option<String>,
    ptr: Cell<*mut c_void>,
}

// SAFETY: `MutableDateRangeMetadataGroup` lazily materializes an ARC-retained
// Objective-C object whose ownership may move across threads.
unsafe impl Send for MutableDateRangeMetadataGroup {}

impl Clone for MutableDateRangeMetadataGroup {
    fn clone(&self) -> Self {
        Self::from_payload(self.payload())
    }
}

impl PartialEq for MutableDateRangeMetadataGroup {
    fn eq(&self, other: &Self) -> bool {
        self.payload() == other.payload()
    }
}

impl core::fmt::Debug for MutableDateRangeMetadataGroup {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("MutableDateRangeMetadataGroup")
            .field("items", &self.items)
            .field("start_date", &self.start_date)
            .field("end_date", &self.end_date)
            .field("classifying_label", &self.classifying_label)
            .field("unique_id", &self.unique_id)
            .finish()
    }
}

impl Drop for MutableDateRangeMetadataGroup {
    fn drop(&mut self) {
        let ptr = self.ptr.replace(ptr::null_mut());
        if !ptr.is_null() {
            unsafe { ffi::av_date_range_metadata_group_release(ptr) };
        }
    }
}

impl MutableDateRangeMetadataGroup {
    #[must_use]
    pub fn new(
        items: Vec<MetadataItem>,
        start_date: impl Into<String>,
        end_date: Option<String>,
    ) -> Self {
        Self {
            items,
            start_date: start_date.into(),
            end_date,
            classifying_label: None,
            unique_id: None,
            ptr: Cell::new(ptr::null_mut()),
        }
    }

    #[must_use]
    pub fn items(&self) -> &[MetadataItem] {
        &self.items
    }

    pub fn set_items(&mut self, items: Vec<MetadataItem>) {
        self.release_ptr();
        self.items = items;
    }

    #[must_use]
    pub fn start_date(&self) -> &str {
        &self.start_date
    }

    pub fn set_start_date(&mut self, start_date: impl Into<String>) {
        self.release_ptr();
        self.start_date = start_date.into();
    }

    #[must_use]
    pub fn end_date(&self) -> Option<&str> {
        self.end_date.as_deref()
    }

    pub fn set_end_date(&mut self, end_date: Option<String>) {
        self.release_ptr();
        self.end_date = end_date;
    }

    #[must_use]
    pub fn as_date_range_metadata_group(&self) -> DateRangeMetadataGroup {
        DateRangeMetadataGroup::from_payload(self.payload())
    }

    pub(crate) fn from_payload(payload: DateRangeMetadataGroupPayload) -> Self {
        Self {
            items: payload
                .items
                .into_iter()
                .map(MetadataItem::from_payload)
                .collect(),
            start_date: payload.start_date,
            end_date: payload.end_date,
            classifying_label: payload.classifying_label,
            unique_id: payload.unique_id,
            ptr: Cell::new(ptr::null_mut()),
        }
    }

    pub(crate) fn payload(&self) -> DateRangeMetadataGroupPayload {
        DateRangeMetadataGroupPayload {
            items: self.items.iter().map(MetadataItem::payload).collect(),
            start_date: self.start_date.clone(),
            end_date: self.end_date.clone(),
            classifying_label: self.classifying_label.clone(),
            unique_id: self.unique_id.clone(),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn as_ptr(&self) -> Result<*mut c_void, AVWriterError> {
        let current = self.ptr.get();
        if !current.is_null() {
            return Ok(current);
        }
        let payload = serialize_json(&self.payload())?;
        let payload_c = cstring_arg(&payload, "mutable date-range metadata group json")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let ptr = unsafe {
            ffi::av_date_range_metadata_group_create_json(payload_c.as_ptr(), true, &mut err_msg)
        };
        if ptr.is_null() {
            return Err(unsafe { from_swift(ffi::status::INVALID_ARGUMENT, err_msg) });
        }
        self.ptr.set(ptr);
        Ok(ptr)
    }

    fn release_ptr(&self) {
        let current = self.ptr.replace(ptr::null_mut());
        if !current.is_null() {
            unsafe { ffi::av_date_range_metadata_group_release(current) };
        }
    }
}

/// Safe wrapper around `AVMetadataItemValueRequest`.
#[derive(Debug)]
pub struct MetadataItemValueRequest {
    ptr: *mut c_void,
}

// SAFETY: `MetadataItemValueRequest` is an ARC-retained Objective-C object.
unsafe impl Send for MetadataItemValueRequest {}

impl Drop for MetadataItemValueRequest {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ffi::av_metadata_item_value_request_release(self.ptr) };
            self.ptr = ptr::null_mut();
        }
    }
}

impl MetadataItemValueRequest {
    pub fn metadata_item(&self) -> Result<Option<MetadataItem>, AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let ptr = unsafe {
            ffi::av_metadata_item_value_request_metadata_item_json(self.ptr, &mut err_msg)
        };
        if ptr.is_null() {
            if err_msg.is_null() {
                return Ok(None);
            }
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        let payload: MetadataItemPayload = parse_json_ptr(ptr, "metadata-item value request item")?;
        Ok(Some(MetadataItem::from_payload(payload)))
    }

    pub fn respond_with_value(&self, value: &MetadataValue) -> Result<(), AVWriterError> {
        let payload = serialize_json(value)?;
        let payload_c = cstring_arg(&payload, "metadata item value json")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_metadata_item_value_request_respond_with_value_json(
                self.ptr,
                payload_c.as_ptr(),
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn respond_with_error(&self, message: &str) -> Result<(), AVWriterError> {
        let message_c = cstring_arg(message, "metadata item value request error")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_metadata_item_value_request_respond_with_error(
                self.ptr,
                message_c.as_ptr(),
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub(crate) fn from_raw(ptr: *mut c_void) -> Option<Self> {
        (!ptr.is_null()).then_some(Self { ptr })
    }
}

struct MetadataValueLoaderState {
    callback: Box<dyn Fn(MetadataItemValueRequest) + Send + 'static>,
}

unsafe extern "C" fn metadata_value_request_trampoline(
    request: *mut c_void,
    userdata: *mut c_void,
) {
    if request.is_null() || userdata.is_null() {
        return;
    }
    catch_user_panic("metadata_value_request_trampoline", || unsafe {
        let state = &*(userdata.cast::<MetadataValueLoaderState>());
        let retained = ffi::av_metadata_item_value_request_retain(request);
        if let Some(request) = MetadataItemValueRequest::from_raw(retained) {
            (state.callback)(request);
        }
    });
}

unsafe extern "C" fn metadata_value_request_drop(userdata: *mut c_void) {
    if userdata.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(userdata.cast::<MetadataValueLoaderState>()));
    }
}
