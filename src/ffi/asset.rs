use super::abi::{AVWArray, AVWOwnedPayload, AVWString};
use core::ffi::c_void;

extern "C" {
    pub fn av_asset_create_url(
        url: *const AVWString,
        is_file_url: bool,
        prefer_precise_duration_and_timing: bool,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut c_void;

    #[link_name = "avw_asset_release"]
    pub fn av_asset_release(asset: *mut c_void);

    #[link_name = "avw_asset_info_payload"]
    pub fn av_asset_info_payload(
        asset: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut AVWOwnedPayload;

    pub fn av_asset_metadata(
        asset: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut c_void;

    #[link_name = "avw_asset_status_of_value"]
    pub fn av_asset_status_of_value(
        asset: *mut c_void,
        key: *const AVWString,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    #[link_name = "avw_asset_load_values_payload"]
    pub fn av_asset_load_values_payload(
        asset: *mut c_void,
        keys_payload: *const AVWArray,
        timeout_seconds: i32,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut AVWOwnedPayload;
}
