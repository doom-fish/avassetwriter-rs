#![allow(clippy::missing_errors_doc, clippy::must_use_candidate)]

use core::ffi::{c_char, c_void};
use core::ptr;
use std::path::Path;

use serde::Deserialize;

use crate::bridge_support::{cstring_arg, parse_json_ptr, serialize_json};
use crate::error::{from_swift, AVWriterError};
use crate::ffi;
use crate::metadata::{
    KeyLoadStatus, KeyLoadStatusPayload, KeyValueStatus, MetadataItem, MetadataItemPayload,
};
use crate::time::Time;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AssetInfoPayload {
    url: Option<String>,
    duration: Time,
    metadata: Vec<MetadataItemPayload>,
}

/// Safe wrapper around `AVAsset` / `AVURLAsset` for file- and URL-backed assets.
#[derive(Debug)]
pub struct Asset {
    ptr: *mut c_void,
}

// SAFETY: `Asset` wraps an ARC-retained Objective-C object. Retain/release are
// atomic, so moving ownership across threads is safe. Concurrent shared access
// is not guaranteed by AVFoundation, so `Sync` is intentionally not implemented.
unsafe impl Send for Asset {}

impl Drop for Asset {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ffi::av_asset_release(self.ptr) };
            self.ptr = ptr::null_mut();
        }
    }
}

impl Asset {
    /// Open a local file-backed asset.
    pub fn from_file_path(path: impl AsRef<Path>) -> Result<Self, AVWriterError> {
        let path = path
            .as_ref()
            .to_str()
            .ok_or_else(|| AVWriterError::InvalidArgument("path is not valid UTF-8".into()))?;
        Self::from_raw_url(path, true, true)
    }

    /// Open a remote or non-file URL-backed asset.
    pub fn from_remote_url(url: impl AsRef<str>) -> Result<Self, AVWriterError> {
        Self::from_raw_url(url.as_ref(), false, true)
    }

    /// Asset duration.
    pub fn duration(&self) -> Result<Time, AVWriterError> {
        Ok(self.info()?.duration)
    }

    /// Static metadata attached to the asset.
    pub fn metadata(&self) -> Result<Vec<MetadataItem>, AVWriterError> {
        Ok(self
            .info()?
            .metadata
            .into_iter()
            .map(MetadataItem::from_payload)
            .collect())
    }

    /// Asset URL when backed by `AVURLAsset`.
    pub fn url(&self) -> Result<Option<String>, AVWriterError> {
        Ok(self.info()?.url)
    }

    /// Query the current load status of a key without triggering loading.
    pub fn status_of_value(&self, key: &str) -> Result<KeyValueStatus, AVWriterError> {
        let key_c = cstring_arg(key, "asset key")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let raw = unsafe { ffi::av_asset_status_of_value(self.ptr, key_c.as_ptr(), &mut err_msg) };
        if raw < 0 {
            return Err(unsafe { from_swift(raw, err_msg) });
        }
        Ok(KeyValueStatus::from_raw(raw))
    }

    /// Trigger asynchronous loading for the given keys and wait for completion.
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
        let payload_c = cstring_arg(&payload, "asset keys json")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let statuses_ptr = unsafe {
            ffi::av_asset_load_values_json(self.ptr, payload_c.as_ptr(), 30, &mut err_msg)
        };
        if statuses_ptr.is_null() {
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        let raw_statuses: Vec<KeyLoadStatusPayload> =
            parse_json_ptr(statuses_ptr, "asset key statuses")?;
        Ok(raw_statuses
            .into_iter()
            .map(|status| KeyLoadStatus {
                key: status.key,
                status: KeyValueStatus::from_raw(status.status),
                error_message: status.error_message,
            })
            .collect())
    }

    #[allow(dead_code)]
    pub(crate) const fn as_ptr(&self) -> *mut c_void {
        self.ptr
    }

    pub(crate) fn from_raw(ptr: *mut c_void) -> Option<Self> {
        (!ptr.is_null()).then_some(Self { ptr })
    }

    fn from_raw_url(
        url: &str,
        is_file_url: bool,
        prefer_precise_duration_and_timing: bool,
    ) -> Result<Self, AVWriterError> {
        let url_c = cstring_arg(url, "asset URL")?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let ptr = unsafe {
            ffi::av_asset_create_url(
                url_c.as_ptr(),
                is_file_url,
                prefer_precise_duration_and_timing,
                &mut err_msg,
            )
        };
        Self::from_raw(ptr)
            .ok_or_else(|| unsafe { from_swift(ffi::status::INVALID_ARGUMENT, err_msg) })
    }

    fn info(&self) -> Result<AssetInfoPayload, AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let ptr = unsafe { ffi::av_asset_info_json(self.ptr, &mut err_msg) };
        if ptr.is_null() {
            return Err(unsafe { from_swift(ffi::status::INVALID_STATE, err_msg) });
        }
        parse_json_ptr(ptr, "asset info")
    }
}
