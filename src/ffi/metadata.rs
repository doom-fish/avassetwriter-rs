use core::ffi::{c_char, c_void};

pub use doom_fish_utils::ffi_callbacks::DropCallback;

pub type MetadataItemValueRequestCallback =
    unsafe extern "C" fn(request: *mut c_void, userdata: *mut c_void);

extern "C" {
    pub fn av_metadata_item_create_json(
        payload_json: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_void;

    pub fn av_metadata_item_info_json(
        item: *mut c_void,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_char;

    pub fn av_metadata_item_release(item: *mut c_void);

    pub fn av_metadata_item_status_of_value(
        item: *mut c_void,
        key: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_metadata_item_load_values_json(
        item: *mut c_void,
        keys_json: *const c_char,
        timeout_seconds: i32,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_char;

    pub fn av_metadata_item_filter_preferred_languages_json(
        items_json: *const c_char,
        languages_json: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_char;

    pub fn av_metadata_item_filter_identifier_json(
        items_json: *const c_char,
        identifier: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_char;

    pub fn av_metadata_item_filter_metadata_item_filter_json(
        items_json: *const c_char,
        filter: *mut c_void,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_char;

    pub fn av_metadata_item_create_lazy_json(
        base_item_json: *const c_char,
        callback: Option<MetadataItemValueRequestCallback>,
        userdata: *mut c_void,
        drop_userdata: Option<DropCallback>,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_void;

    pub fn av_metadata_item_value_request_retain(request: *mut c_void) -> *mut c_void;

    pub fn av_metadata_item_value_request_release(request: *mut c_void);

    pub fn av_metadata_item_value_request_metadata_item_json(
        request: *mut c_void,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_char;

    pub fn av_metadata_item_value_request_respond_with_value_json(
        request: *mut c_void,
        value_json: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_metadata_item_value_request_respond_with_error(
        request: *mut c_void,
        error_message: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_timed_metadata_group_create_json(
        payload_json: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_void;

    pub fn av_timed_metadata_group_info_json(
        group: *mut c_void,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_char;

    pub fn av_timed_metadata_group_release(group: *mut c_void);

    pub fn av_date_range_metadata_group_create_json(
        payload_json: *const c_char,
        mutable_group: bool,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_void;

    pub fn av_date_range_metadata_group_info_json(
        group: *mut c_void,
        out_error_message: *mut *mut c_char,
    ) -> *mut c_char;

    pub fn av_date_range_metadata_group_release(group: *mut c_void);
}
