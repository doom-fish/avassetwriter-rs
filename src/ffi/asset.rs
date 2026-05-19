use core::ffi::{c_char, c_void};

extern "C" {
    pub fn av_asset_create_url(
        url: *const c_char,
        is_file_url: bool,
        prefer_precise_duration_and_timing: bool,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_void;

    pub fn av_asset_release(asset: *mut c_void);

    pub fn av_asset_info_json(
        asset: *mut c_void,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_char;

    pub fn av_asset_status_of_value(
        asset: *mut c_void,
        key: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_asset_load_values_json(
        asset: *mut c_void,
        keys_json: *const c_char,
        timeout_seconds: i32,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_char;
}
