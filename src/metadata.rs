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
use core::ffi::c_void;
use core::ptr;

use doom_fish_utils::panic_safe::catch_user_panic;

use crate::bridge_support::{
    payload_arg, string_arg, take_payload, time_kind, time_scale, time_value,
};
use crate::error::{from_swift, AVWriterError};
use crate::ffi;
use crate::media_processing::MetadataItemFilter;
use apple_cf::cm::CMTimeRange;

/// A metadata value supported by the AVFoundation bridge payloads.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum MetadataValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Data(Vec<u8>),
}

impl crate::binary::ToAbi for MetadataValue {
    type Abi = ffi::AVWMetadataValue;

    fn to_abi(&self, arena: &mut crate::binary::Arena) -> Self::Abi {
        let mut raw = Self::Abi {
            kind: 0,
            string_value: ffi::AVWString {
                data: ptr::null(),
                length: 0,
            },
            integer_value: 0,
            float_value: 0.0,
            boolean_value: 0,
            data_value: ffi::AVWArray {
                data: ptr::null(),
                length: 0,
            },
        };
        match self {
            Self::String(value) => raw.string_value = crate::binary::ToAbi::to_abi(value, arena),
            Self::Integer(value) => {
                raw.kind = 1;
                raw.integer_value = *value;
            }
            Self::Float(value) => {
                raw.kind = 2;
                raw.float_value = *value;
            }
            Self::Boolean(value) => {
                raw.kind = 3;
                raw.boolean_value = u8::from(*value);
            }
            Self::Data(value) => {
                raw.kind = 4;
                raw.data_value = crate::binary::ToAbi::to_abi(value, arena);
            }
        }
        raw
    }
}

impl crate::binary::FromAbi for MetadataValue {
    fn from_abi(raw: &Self::Abi) -> Result<Self, AVWriterError> {
        match raw.kind {
            0 => Ok(Self::String(String::from_abi(&raw.string_value)?)),
            1 => Ok(Self::Integer(raw.integer_value)),
            2 => Ok(Self::Float(<f64 as crate::binary::FromAbi>::from_abi(
                &raw.float_value,
            )?)),
            3 => Ok(Self::Boolean(bool::from_abi(&raw.boolean_value)?)),
            4 => Ok(Self::Data(Vec::<u8>::from_abi(&raw.data_value)?)),
            _ => Err(AVWriterError::InvalidState(
                "invalid metadata value kind".into(),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct MetadataItemPayload {
    identifier: String,
    value: MetadataValue,
    data_type: Option<String>,
    extended_language_tag: Option<String>,
    locale_identifier: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MetadataItemPropertiesPayload {
    identifier: String,
    data_type: Option<String>,
    extended_language_tag: Option<String>,
    locale_identifier: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct TimedMetadataGroupPropertiesPayload {
    time_range: CMTimeRange,
    classifying_label: Option<String>,
    unique_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
struct DateRangeMetadataGroupPropertiesPayload {
    start_date: String,
    end_date: Option<String>,
    classifying_label: Option<String>,
    unique_id: Option<String>,
}

/// A CoreMedia metadata specification used to build a metadata-track format hint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataSpecification {
    pub identifier: String,
    pub data_type: String,
    pub extended_language_tag: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
///
/// Cloning a native-backed item retains the same native object without
/// resolving a lazy value.
pub struct MetadataItem {
    identifier: String,
    detached_value: Option<MetadataValue>,
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
        let current = self.ptr.get();
        let ptr = if current.is_null() {
            ptr::null_mut()
        } else {
            unsafe { ffi::av_metadata_item_retain(current) }
        };
        Self {
            identifier: self.identifier.clone(),
            detached_value: self.detached_value.clone(),
            data_type: self.data_type.clone(),
            extended_language_tag: self.extended_language_tag.clone(),
            locale_identifier: self.locale_identifier.clone(),
            ptr: Cell::new(ptr),
        }
    }
}

impl PartialEq for MetadataItem {
    fn eq(&self, other: &Self) -> bool {
        let self_ptr = self.ptr.get();
        let other_ptr = other.ptr.get();
        if !self_ptr.is_null() && self_ptr == other_ptr {
            return true;
        }
        if self.identifier != other.identifier
            || self.data_type != other.data_type
            || self.extended_language_tag != other.extended_language_tag
            || self.locale_identifier != other.locale_identifier
        {
            return false;
        }
        matches!((self.value(), other.value()), (Ok(left), Ok(right)) if left == right)
    }
}

impl core::fmt::Debug for MetadataItem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut debug = f.debug_struct("MetadataItem");
        debug
            .field("identifier", &self.identifier)
            .field("data_type", &self.data_type)
            .field("extended_language_tag", &self.extended_language_tag)
            .field("locale_identifier", &self.locale_identifier);
        if self.ptr.get().is_null() {
            debug.field("value", &self.detached_value);
        } else {
            debug.field("native_backed", &true);
        }
        debug.finish()
    }
}

crate::utils::retained::av_retained!(
    MetadataItem,
    drop_only,
    release = ffi::av_metadata_item_release
);

impl MetadataItem {
    #[must_use]
    pub fn new(identifier: impl Into<String>, value: MetadataValue) -> Self {
        Self {
            identifier: identifier.into(),
            detached_value: Some(value),
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

    /// Set the native data type on a copy of this item.
    ///
    /// Native-backed items use the framework's synchronous
    /// `mutableCopy`/`copy` path. That path may invoke a lazy value loader
    /// before this method returns.
    #[must_use]
    pub fn with_data_type(mut self, data_type: impl Into<String>) -> Self {
        let data_type = data_type.into();
        let current = self.ptr.get();
        if !current.is_null() {
            let replacement = unsafe {
                ffi::av_metadata_item_copy_with_data_type(
                    current,
                    data_type.as_ptr(),
                    data_type.len(),
                )
            };
            self.replace_native_ptr(current, replacement);
        }
        self.data_type = Some(data_type);
        self
    }

    /// Set the extended language tag on a copy of this item.
    ///
    /// Native-backed items use the framework's synchronous
    /// `mutableCopy`/`copy` path. That path may invoke a lazy value loader
    /// before this method returns.
    #[must_use]
    pub fn with_extended_language_tag(mut self, tag: impl Into<String>) -> Self {
        let tag = tag.into();
        let current = self.ptr.get();
        if !current.is_null() {
            let replacement = unsafe {
                ffi::av_metadata_item_copy_with_extended_language_tag(
                    current,
                    tag.as_ptr(),
                    tag.len(),
                )
            };
            self.replace_native_ptr(current, replacement);
        }
        self.extended_language_tag = Some(tag);
        self
    }

    /// Set the locale on a copy of this item.
    ///
    /// Native-backed items use the framework's synchronous
    /// `mutableCopy`/`copy` path. That path may invoke a lazy value loader
    /// before this method returns.
    #[must_use]
    pub fn with_locale_identifier(mut self, identifier: impl Into<String>) -> Self {
        let identifier = identifier.into();
        let current = self.ptr.get();
        if !current.is_null() {
            let replacement = unsafe {
                ffi::av_metadata_item_copy_with_locale_identifier(
                    current,
                    identifier.as_ptr(),
                    identifier.len(),
                )
            };
            self.replace_native_ptr(current, replacement);
        }
        self.locale_identifier = Some(identifier);
        self
    }

    #[must_use]
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    /// Return the authoritative value.
    ///
    /// For a lazy native item this synchronously invokes its value loader on
    /// first access.
    pub fn value(&self) -> Result<MetadataValue, AVWriterError> {
        let current = self.ptr.get();
        if current.is_null() {
            return self.detached_value.clone().ok_or_else(|| {
                AVWriterError::InvalidState("detached metadata item has no value".into())
            });
        }
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let value_ptr = unsafe { ffi::av_metadata_item_value_payload(current, &raw mut err_msg) };
        if value_ptr.is_null() {
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        take_payload(value_ptr, "metadata item value")
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
        let key_c = string_arg(key, "metadata item key")?;
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let raw = unsafe {
            ffi::av_metadata_item_status_of_value(self.as_ptr()?, key_c.as_ptr(), &raw mut err_msg)
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
        let payload_c = payload_arg(&keys);
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let statuses_ptr = unsafe {
            ffi::av_metadata_item_load_values_payload(
                self.as_ptr()?,
                payload_c.as_ptr(),
                30,
                &raw mut err_msg,
            )
        };
        if statuses_ptr.is_null() {
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        let raw_statuses: Vec<KeyLoadStatusPayload> =
            take_payload(statuses_ptr, "metadata item key statuses")?;
        Ok(raw_statuses
            .into_iter()
            .map(|status| KeyLoadStatus {
                key: status.key,
                status: KeyValueStatus::from_raw(status.status),
                error_message: status.error_message,
            })
            .collect())
    }

    /// Create a lazy native item from the exact native representation of
    /// `base`. Construction and cloning do not invoke `callback`.
    pub fn with_lazy_value_loader<F>(base: &Self, callback: F) -> Result<Self, AVWriterError>
    where
        F: Fn(MetadataItemValueRequest) + Send + Sync + 'static,
    {
        let base_ptr = base.as_ptr()?;
        let state = Box::new(MetadataValueLoaderState {
            callback: Box::new(callback),
        });
        let userdata = Box::into_raw(state).cast::<c_void>();
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let ptr = unsafe {
            ffi::av_metadata_item_create_lazy(
                base_ptr,
                Some(metadata_value_request_trampoline),
                userdata,
                Some(metadata_value_request_drop),
                &raw mut err_msg,
            )
        };
        if ptr.is_null() {
            unsafe { metadata_value_request_drop(userdata) };
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        Ok(Self {
            identifier: base.identifier.clone(),
            detached_value: base.detached_value.clone(),
            data_type: base.data_type.clone(),
            extended_language_tag: base.extended_language_tag.clone(),
            locale_identifier: base.locale_identifier.clone(),
            ptr: Cell::new(ptr),
        })
    }

    pub fn filtered_and_sorted_according_to_preferred_languages<I, S>(
        items: &[Self],
        preferred_languages: I,
    ) -> Result<Vec<Self>, AVWriterError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let item_ptrs = Self::native_ptrs(items)?;
        let languages_payload = preferred_languages
            .into_iter()
            .map(|value| value.as_ref().to_owned())
            .collect::<Vec<_>>();
        let languages_c = payload_arg(&languages_payload);
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let array = unsafe {
            ffi::av_metadata_item_filter_preferred_languages(
                item_ptrs.as_ptr(),
                item_ptrs.len(),
                languages_c.as_ptr(),
                &raw mut err_msg,
            )
        };
        if array.is_null() {
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        Self::from_retained_array(array, "preferred-language metadata items")
    }

    pub fn filtered_by_identifier(
        items: &[Self],
        identifier: impl AsRef<str>,
    ) -> Result<Vec<Self>, AVWriterError> {
        let item_ptrs = Self::native_ptrs(items)?;
        let identifier_c = string_arg(identifier.as_ref(), "metadata identifier")?;
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let array = unsafe {
            ffi::av_metadata_item_filter_identifier(
                item_ptrs.as_ptr(),
                item_ptrs.len(),
                identifier_c.as_ptr(),
                &raw mut err_msg,
            )
        };
        if array.is_null() {
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        Self::from_retained_array(array, "identifier-filtered metadata items")
    }

    pub fn filtered_by_metadata_item_filter(
        items: &[Self],
        filter: &MetadataItemFilter,
    ) -> Result<Vec<Self>, AVWriterError> {
        let item_ptrs = Self::native_ptrs(items)?;
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let array = unsafe {
            ffi::av_metadata_item_filter_metadata_item_filter(
                item_ptrs.as_ptr(),
                item_ptrs.len(),
                filter.as_ptr(),
                &raw mut err_msg,
            )
        };
        if array.is_null() {
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        Self::from_retained_array(array, "metadata-filtered items")
    }

    fn payload(&self) -> Result<MetadataItemPayload, AVWriterError> {
        Ok(MetadataItemPayload {
            identifier: self.identifier.clone(),
            value: self.value()?,
            data_type: self.data_type.clone(),
            extended_language_tag: self.extended_language_tag.clone(),
            locale_identifier: self.locale_identifier.clone(),
        })
    }

    pub(crate) fn as_ptr(&self) -> Result<*mut c_void, AVWriterError> {
        let current = self.ptr.get();
        if !current.is_null() {
            return Ok(current);
        }
        let payload_c = payload_arg(&self.payload()?);
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let ptr =
            unsafe { ffi::av_metadata_item_create_payload(payload_c.as_ptr(), &raw mut err_msg) };
        if ptr.is_null() {
            return Err(unsafe { from_swift(ffi::status::INVALID_ARGUMENT, err_msg) });
        }
        self.ptr.set(ptr);
        Ok(ptr)
    }

    pub(crate) fn native_ptrs(items: &[Self]) -> Result<Vec<*mut c_void>, AVWriterError> {
        items.iter().map(Self::as_ptr).collect()
    }

    pub(crate) fn from_retained_array(
        array: *mut c_void,
        context: &str,
    ) -> Result<Vec<Self>, AVWriterError> {
        let array = RetainedMetadataItemArray(array);
        let count = unsafe { ffi::av_metadata_item_array_count(array.0) };
        let mut items = Vec::with_capacity(count);
        for index in 0..count {
            let item = unsafe { ffi::av_metadata_item_array_item_retain(array.0, index) };
            if item.is_null() {
                return Err(AVWriterError::InvalidState(format!(
                    "{context} contained an invalid item at index {index}"
                )));
            }
            items.push(Self::from_retained_ptr(item)?);
        }
        Ok(items)
    }

    fn from_retained_ptr(item_ptr: *mut c_void) -> Result<Self, AVWriterError> {
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let properties_ptr =
            unsafe { ffi::av_metadata_item_properties_payload(item_ptr, &raw mut err_msg) };
        if properties_ptr.is_null() {
            unsafe { ffi::av_metadata_item_release(item_ptr) };
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        let properties: MetadataItemPropertiesPayload =
            match take_payload(properties_ptr, "metadata item properties") {
                Ok(properties) => properties,
                Err(error) => {
                    unsafe { ffi::av_metadata_item_release(item_ptr) };
                    return Err(error);
                }
            };
        Ok(Self {
            identifier: properties.identifier,
            detached_value: None,
            data_type: properties.data_type,
            extended_language_tag: properties.extended_language_tag,
            locale_identifier: properties.locale_identifier,
            ptr: Cell::new(item_ptr),
        })
    }

    fn replace_native_ptr(&self, current: *mut c_void, replacement: *mut c_void) {
        assert!(
            !replacement.is_null(),
            "AVMetadataItem native copy unexpectedly failed"
        );
        self.ptr.set(replacement);
        unsafe { ffi::av_metadata_item_release(current) };
    }
}

struct RetainedMetadataItemArray(*mut c_void);

impl Drop for RetainedMetadataItemArray {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { ffi::av_metadata_item_array_release(self.0) };
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
}

/// Safe wrapper around `AVTimedMetadataGroup`.
pub struct TimedMetadataGroup {
    items: Vec<MetadataItem>,
    time_range: CMTimeRange,
    classifying_label: Option<String>,
    unique_id: Option<String>,
    ptr: Cell<*mut c_void>,
}

// SAFETY: `TimedMetadataGroup` lazily materializes an ARC-retained Objective-C
// object whose ownership may move across threads.
unsafe impl Send for TimedMetadataGroup {}

impl Clone for TimedMetadataGroup {
    fn clone(&self) -> Self {
        let current = self.ptr.get();
        let ptr = if current.is_null() {
            ptr::null_mut()
        } else {
            unsafe { ffi::av_timed_metadata_group_retain(current) }
        };
        Self {
            items: self.items.clone(),
            time_range: self.time_range,
            classifying_label: self.classifying_label.clone(),
            unique_id: self.unique_id.clone(),
            ptr: Cell::new(ptr),
        }
    }
}

impl PartialEq for TimedMetadataGroup {
    fn eq(&self, other: &Self) -> bool {
        self.items == other.items
            && self.time_range == other.time_range
            && self.classifying_label == other.classifying_label
            && self.unique_id == other.unique_id
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

crate::utils::retained::av_retained!(
    TimedMetadataGroup,
    drop_only,
    release = ffi::av_timed_metadata_group_release
);

impl TimedMetadataGroup {
    #[must_use]
    pub fn new(items: Vec<MetadataItem>, time_range: CMTimeRange) -> Self {
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
    pub const fn time_range(&self) -> CMTimeRange {
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

    pub(crate) fn as_ptr(&self) -> Result<*mut c_void, AVWriterError> {
        let current = self.ptr.get();
        if !current.is_null() {
            return Ok(current);
        }
        let item_ptrs = MetadataItem::native_ptrs(&self.items)?;
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let ptr = unsafe {
            ffi::av_timed_metadata_group_create_items(
                item_ptrs.as_ptr(),
                item_ptrs.len(),
                time_value(&self.time_range.start),
                time_scale(&self.time_range.start),
                time_kind(&self.time_range.start),
                time_value(&self.time_range.duration),
                time_scale(&self.time_range.duration),
                time_kind(&self.time_range.duration),
                &raw mut err_msg,
            )
        };
        if ptr.is_null() {
            return Err(unsafe { from_swift(ffi::status::INVALID_ARGUMENT, err_msg) });
        }
        self.ptr.set(ptr);
        Ok(ptr)
    }

    #[allow(dead_code)]
    pub(crate) fn from_retained_ptr(group_ptr: *mut c_void) -> Result<Self, AVWriterError> {
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let items_ptr = unsafe { ffi::av_metadata_group_items(group_ptr, &raw mut err_msg) };
        if items_ptr.is_null() {
            unsafe { ffi::av_timed_metadata_group_release(group_ptr) };
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        let items = match MetadataItem::from_retained_array(items_ptr, "timed metadata group items")
        {
            Ok(items) => items,
            Err(error) => {
                unsafe { ffi::av_timed_metadata_group_release(group_ptr) };
                return Err(error);
            }
        };
        let properties_ptr =
            unsafe { ffi::av_timed_metadata_group_properties_payload(group_ptr, &raw mut err_msg) };
        if properties_ptr.is_null() {
            unsafe { ffi::av_timed_metadata_group_release(group_ptr) };
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        let properties: TimedMetadataGroupPropertiesPayload =
            match take_payload(properties_ptr, "timed metadata group properties") {
                Ok(properties) => properties,
                Err(error) => {
                    unsafe { ffi::av_timed_metadata_group_release(group_ptr) };
                    return Err(error);
                }
            };
        Ok(Self {
            items,
            time_range: properties.time_range,
            classifying_label: properties.classifying_label,
            unique_id: properties.unique_id,
            ptr: Cell::new(group_ptr),
        })
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
        let current = self.ptr.get();
        let ptr = if current.is_null() {
            ptr::null_mut()
        } else {
            unsafe { ffi::av_date_range_metadata_group_retain(current) }
        };
        Self {
            items: self.items.clone(),
            start_date: self.start_date.clone(),
            end_date: self.end_date.clone(),
            classifying_label: self.classifying_label.clone(),
            unique_id: self.unique_id.clone(),
            ptr: Cell::new(ptr),
        }
    }
}

impl PartialEq for DateRangeMetadataGroup {
    fn eq(&self, other: &Self) -> bool {
        self.items == other.items
            && self.start_date == other.start_date
            && self.end_date == other.end_date
            && self.classifying_label == other.classifying_label
            && self.unique_id == other.unique_id
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

crate::utils::retained::av_retained!(
    DateRangeMetadataGroup,
    drop_only,
    release = ffi::av_date_range_metadata_group_release
);

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

    #[allow(dead_code)]
    pub(crate) fn as_ptr(&self) -> Result<*mut c_void, AVWriterError> {
        let current = self.ptr.get();
        if !current.is_null() {
            return Ok(current);
        }
        let item_ptrs = MetadataItem::native_ptrs(&self.items)?;
        let start_date = string_arg(&self.start_date, "date-range metadata start date")?;
        let end_date = self
            .end_date
            .as_deref()
            .map(|value| string_arg(value, "date-range metadata end date"))
            .transpose()?;
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let ptr = unsafe {
            ffi::av_date_range_metadata_group_create_items(
                item_ptrs.as_ptr(),
                item_ptrs.len(),
                start_date.as_ptr(),
                end_date
                    .as_ref()
                    .map_or(ptr::null(), super::binary::Argument::as_ptr),
                false,
                &raw mut err_msg,
            )
        };
        if ptr.is_null() {
            return Err(unsafe { from_swift(ffi::status::INVALID_ARGUMENT, err_msg) });
        }
        self.ptr.set(ptr);
        Ok(ptr)
    }

    #[allow(dead_code)]
    pub(crate) fn from_retained_ptr(group_ptr: *mut c_void) -> Result<Self, AVWriterError> {
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let items_ptr = unsafe { ffi::av_metadata_group_items(group_ptr, &raw mut err_msg) };
        if items_ptr.is_null() {
            unsafe { ffi::av_date_range_metadata_group_release(group_ptr) };
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        let items =
            match MetadataItem::from_retained_array(items_ptr, "date-range metadata group items") {
                Ok(items) => items,
                Err(error) => {
                    unsafe { ffi::av_date_range_metadata_group_release(group_ptr) };
                    return Err(error);
                }
            };
        let properties_ptr = unsafe {
            ffi::av_date_range_metadata_group_properties_payload(group_ptr, &raw mut err_msg)
        };
        if properties_ptr.is_null() {
            unsafe { ffi::av_date_range_metadata_group_release(group_ptr) };
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        let properties: DateRangeMetadataGroupPropertiesPayload =
            match take_payload(properties_ptr, "date-range metadata group properties") {
                Ok(properties) => properties,
                Err(error) => {
                    unsafe { ffi::av_date_range_metadata_group_release(group_ptr) };
                    return Err(error);
                }
            };
        Ok(Self {
            items,
            start_date: properties.start_date,
            end_date: properties.end_date,
            classifying_label: properties.classifying_label,
            unique_id: properties.unique_id,
            ptr: Cell::new(group_ptr),
        })
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
        let current = self.ptr.get();
        let ptr = if current.is_null() {
            ptr::null_mut()
        } else {
            let copied = unsafe { ffi::av_date_range_metadata_group_mutable_copy(current) };
            assert!(
                !copied.is_null(),
                "AVMutableDateRangeMetadataGroup native copy unexpectedly failed"
            );
            copied
        };
        Self {
            items: self.items.clone(),
            start_date: self.start_date.clone(),
            end_date: self.end_date.clone(),
            classifying_label: self.classifying_label.clone(),
            unique_id: self.unique_id.clone(),
            ptr: Cell::new(ptr),
        }
    }
}

impl PartialEq for MutableDateRangeMetadataGroup {
    fn eq(&self, other: &Self) -> bool {
        self.items == other.items
            && self.start_date == other.start_date
            && self.end_date == other.end_date
            && self.classifying_label == other.classifying_label
            && self.unique_id == other.unique_id
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

crate::utils::retained::av_retained!(
    MutableDateRangeMetadataGroup,
    drop_only,
    release = ffi::av_date_range_metadata_group_release
);

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
    #[allow(clippy::missing_panics_doc)]
    pub fn as_date_range_metadata_group(&self) -> DateRangeMetadataGroup {
        let current = self.ptr.get();
        let ptr = if current.is_null() {
            ptr::null_mut()
        } else {
            let copied = unsafe { ffi::av_date_range_metadata_group_immutable_copy(current) };
            assert!(
                !copied.is_null(),
                "AVDateRangeMetadataGroup native copy unexpectedly failed"
            );
            copied
        };
        DateRangeMetadataGroup {
            items: self.items.clone(),
            start_date: self.start_date.clone(),
            end_date: self.end_date.clone(),
            classifying_label: self.classifying_label.clone(),
            unique_id: self.unique_id.clone(),
            ptr: Cell::new(ptr),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn as_ptr(&self) -> Result<*mut c_void, AVWriterError> {
        let current = self.ptr.get();
        if !current.is_null() {
            return Ok(current);
        }
        let item_ptrs = MetadataItem::native_ptrs(&self.items)?;
        let start_date = string_arg(&self.start_date, "mutable date-range metadata start date")?;
        let end_date = self
            .end_date
            .as_deref()
            .map(|value| string_arg(value, "mutable date-range metadata end date"))
            .transpose()?;
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let ptr = unsafe {
            ffi::av_date_range_metadata_group_create_items(
                item_ptrs.as_ptr(),
                item_ptrs.len(),
                start_date.as_ptr(),
                end_date
                    .as_ref()
                    .map_or(ptr::null(), super::binary::Argument::as_ptr),
                true,
                &raw mut err_msg,
            )
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
    /// Return a retained alias of the native item associated with this request.
    pub fn metadata_item(&self) -> Result<Option<MetadataItem>, AVWriterError> {
        let ptr = unsafe { ffi::av_metadata_item_value_request_metadata_item(self.ptr) };
        if ptr.is_null() {
            return Ok(None);
        }
        MetadataItem::from_retained_ptr(ptr).map(Some)
    }

    pub fn respond_with_value(&self, value: &MetadataValue) -> Result<(), AVWriterError> {
        let payload_c = payload_arg(value);
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let status = unsafe {
            ffi::av_metadata_item_value_request_respond_with_value_payload(
                self.ptr,
                payload_c.as_ptr(),
                &raw mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    pub fn respond_with_error(&self, message: &str) -> Result<(), AVWriterError> {
        let message_c = string_arg(message, "metadata item value request error")?;
        let mut err_msg: *mut ffi::AVWOwnedPayload = ptr::null_mut();
        let status = unsafe {
            ffi::av_metadata_item_value_request_respond_with_error(
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

    pub(crate) fn from_raw(ptr: *mut c_void) -> Option<Self> {
        (!ptr.is_null()).then_some(Self { ptr })
    }
}

struct MetadataValueLoaderState {
    callback: Box<dyn Fn(MetadataItemValueRequest) + Send + Sync + 'static>,
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
    // Dropping the box runs the user closure's destructor, so any panic it
    // raises must be contained before unwinding across the C ABI boundary.
    catch_user_panic("metadata_value_request_drop", || unsafe {
        drop(Box::from_raw(userdata.cast::<MetadataValueLoaderState>()));
    });
}

impl crate::binary::ToAbi for MetadataItemPayload {
    type Abi = crate::ffi::AVWMetadataItem;

    fn to_abi(&self, arena: &mut crate::binary::Arena) -> Self::Abi {
        Self::Abi {
            identifier: crate::binary::ToAbi::to_abi(&self.identifier, arena),
            value: crate::binary::ToAbi::to_abi(&self.value, arena),
            data_type: crate::binary::ToAbi::to_abi(&self.data_type, arena),
            extended_language_tag: crate::binary::ToAbi::to_abi(&self.extended_language_tag, arena),
            locale_identifier: crate::binary::ToAbi::to_abi(&self.locale_identifier, arena),
        }
    }
}

impl crate::binary::FromAbi for MetadataItemPayload {
    fn from_abi(raw: &Self::Abi) -> Result<Self, crate::error::AVWriterError> {
        Ok(Self {
            identifier: crate::binary::FromAbi::from_abi(&raw.identifier)?,
            value: crate::binary::FromAbi::from_abi(&raw.value)?,
            data_type: crate::binary::FromAbi::from_abi(&raw.data_type)?,
            extended_language_tag: crate::binary::FromAbi::from_abi(&raw.extended_language_tag)?,
            locale_identifier: crate::binary::FromAbi::from_abi(&raw.locale_identifier)?,
        })
    }
}

impl crate::binary::ToAbi for MetadataItemPropertiesPayload {
    type Abi = crate::ffi::AVWMetadataItemProperties;

    fn to_abi(&self, arena: &mut crate::binary::Arena) -> Self::Abi {
        Self::Abi {
            identifier: crate::binary::ToAbi::to_abi(&self.identifier, arena),
            data_type: crate::binary::ToAbi::to_abi(&self.data_type, arena),
            extended_language_tag: crate::binary::ToAbi::to_abi(&self.extended_language_tag, arena),
            locale_identifier: crate::binary::ToAbi::to_abi(&self.locale_identifier, arena),
        }
    }
}

impl crate::binary::FromAbi for MetadataItemPropertiesPayload {
    fn from_abi(raw: &Self::Abi) -> Result<Self, crate::error::AVWriterError> {
        Ok(Self {
            identifier: crate::binary::FromAbi::from_abi(&raw.identifier)?,
            data_type: crate::binary::FromAbi::from_abi(&raw.data_type)?,
            extended_language_tag: crate::binary::FromAbi::from_abi(&raw.extended_language_tag)?,
            locale_identifier: crate::binary::FromAbi::from_abi(&raw.locale_identifier)?,
        })
    }
}

impl crate::binary::ToAbi for TimedMetadataGroupPropertiesPayload {
    type Abi = crate::ffi::AVWTimedMetadataGroupProperties;

    fn to_abi(&self, arena: &mut crate::binary::Arena) -> Self::Abi {
        Self::Abi {
            time_range: crate::binary::ToAbi::to_abi(&self.time_range, arena),
            classifying_label: crate::binary::ToAbi::to_abi(&self.classifying_label, arena),
            unique_id: crate::binary::ToAbi::to_abi(&self.unique_id, arena),
        }
    }
}

impl crate::binary::FromAbi for TimedMetadataGroupPropertiesPayload {
    fn from_abi(raw: &Self::Abi) -> Result<Self, crate::error::AVWriterError> {
        Ok(Self {
            time_range: crate::binary::FromAbi::from_abi(&raw.time_range)?,
            classifying_label: crate::binary::FromAbi::from_abi(&raw.classifying_label)?,
            unique_id: crate::binary::FromAbi::from_abi(&raw.unique_id)?,
        })
    }
}

impl crate::binary::ToAbi for DateRangeMetadataGroupPropertiesPayload {
    type Abi = crate::ffi::AVWDateRangeMetadataGroupProperties;

    fn to_abi(&self, arena: &mut crate::binary::Arena) -> Self::Abi {
        Self::Abi {
            start_date: crate::binary::ToAbi::to_abi(&self.start_date, arena),
            end_date: crate::binary::ToAbi::to_abi(&self.end_date, arena),
            classifying_label: crate::binary::ToAbi::to_abi(&self.classifying_label, arena),
            unique_id: crate::binary::ToAbi::to_abi(&self.unique_id, arena),
        }
    }
}

impl crate::binary::FromAbi for DateRangeMetadataGroupPropertiesPayload {
    fn from_abi(raw: &Self::Abi) -> Result<Self, crate::error::AVWriterError> {
        Ok(Self {
            start_date: crate::binary::FromAbi::from_abi(&raw.start_date)?,
            end_date: crate::binary::FromAbi::from_abi(&raw.end_date)?,
            classifying_label: crate::binary::FromAbi::from_abi(&raw.classifying_label)?,
            unique_id: crate::binary::FromAbi::from_abi(&raw.unique_id)?,
        })
    }
}

impl crate::binary::ToAbi for MetadataSpecification {
    type Abi = crate::ffi::AVWMetadataSpecification;

    fn to_abi(&self, arena: &mut crate::binary::Arena) -> Self::Abi {
        Self::Abi {
            identifier: crate::binary::ToAbi::to_abi(&self.identifier, arena),
            data_type: crate::binary::ToAbi::to_abi(&self.data_type, arena),
            extended_language_tag: crate::binary::ToAbi::to_abi(&self.extended_language_tag, arena),
        }
    }
}

impl crate::binary::FromAbi for MetadataSpecification {
    fn from_abi(raw: &Self::Abi) -> Result<Self, crate::error::AVWriterError> {
        Ok(Self {
            identifier: crate::binary::FromAbi::from_abi(&raw.identifier)?,
            data_type: crate::binary::FromAbi::from_abi(&raw.data_type)?,
            extended_language_tag: crate::binary::FromAbi::from_abi(&raw.extended_language_tag)?,
        })
    }
}

impl crate::binary::ToAbi for KeyLoadStatusPayload {
    type Abi = crate::ffi::AVWKeyLoadStatus;

    fn to_abi(&self, arena: &mut crate::binary::Arena) -> Self::Abi {
        Self::Abi {
            key: crate::binary::ToAbi::to_abi(&self.key, arena),
            status: crate::binary::ToAbi::to_abi(&self.status, arena),
            error_message: crate::binary::ToAbi::to_abi(&self.error_message, arena),
        }
    }
}

impl crate::binary::FromAbi for KeyLoadStatusPayload {
    fn from_abi(raw: &Self::Abi) -> Result<Self, crate::error::AVWriterError> {
        Ok(Self {
            key: crate::binary::FromAbi::from_abi(&raw.key)?,
            status: crate::binary::FromAbi::from_abi(&raw.status)?,
            error_message: crate::binary::FromAbi::from_abi(&raw.error_message)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use super::*;
    use apple_cf::cm::CMTime;

    #[test]
    fn value_loader_state_can_be_shared_across_threads() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MetadataValueLoaderState>();
    }

    #[test]
    fn timed_group_materialization_and_clone_keep_lazy_item() {
        let loads = Arc::new(AtomicUsize::new(0));
        let callback_loads = Arc::clone(&loads);
        let base = MetadataItem::string("mdta/com.example.identity", "stale seed");
        let lazy = MetadataItem::with_lazy_value_loader(&base, move |request| {
            callback_loads.fetch_add(1, Ordering::SeqCst);
            request
                .respond_with_value(&MetadataValue::String("loaded".into()))
                .expect("lazy metadata response should succeed");
        })
        .expect("lazy metadata item should be created");
        let group = TimedMetadataGroup::new(
            vec![lazy.clone()],
            CMTimeRange::new(CMTime::new(0, 600), CMTime::new(600, 600)),
        );

        group
            .as_ptr()
            .expect("timed metadata group should materialize");
        let cloned = group.clone();
        drop(group);
        assert_eq!(loads.load(Ordering::SeqCst), 0);
        assert_eq!(
            cloned.items()[0]
                .value()
                .expect("group item value should load"),
            MetadataValue::String("loaded".into())
        );
        assert_eq!(
            lazy.value().expect("original alias should share the value"),
            MetadataValue::String("loaded".into())
        );
        assert_eq!(loads.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn mutable_group_changes_keep_lazy_item() {
        let loads = Arc::new(AtomicUsize::new(0));
        let callback_loads = Arc::clone(&loads);
        let base = MetadataItem::string("mdta/com.example.mutable", "stale seed");
        let lazy = MetadataItem::with_lazy_value_loader(&base, move |request| {
            callback_loads.fetch_add(1, Ordering::SeqCst);
            request
                .respond_with_value(&MetadataValue::String("loaded".into()))
                .expect("lazy metadata response should succeed");
        })
        .expect("lazy metadata item should be created");
        let mut group =
            MutableDateRangeMetadataGroup::new(vec![lazy.clone()], "2026-01-01T00:00:00Z", None);

        group
            .as_ptr()
            .expect("mutable metadata group should materialize");
        group.set_start_date("2026-01-02T00:00:00Z");
        group.set_end_date(Some("2026-01-03T00:00:00Z".into()));
        group.set_items(vec![lazy.clone()]);
        group
            .as_ptr()
            .expect("changed mutable metadata group should materialize");
        let immutable = group.as_date_range_metadata_group();
        assert_eq!(loads.load(Ordering::SeqCst), 0);
        assert_eq!(
            immutable.items()[0]
                .value()
                .expect("mutable group item value should load"),
            MetadataValue::String("loaded".into())
        );
        assert_eq!(
            lazy.value().expect("original alias should share the value"),
            MetadataValue::String("loaded".into())
        );
        assert_eq!(loads.load(Ordering::SeqCst), 1);
    }
}
