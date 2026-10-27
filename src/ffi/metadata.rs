use super::abi::{
    AVWArray, AVWDateRangeMetadataGroup, AVWMetadataItem, AVWMetadataValue, AVWOwnedPayload,
    AVWString, AVWTimedMetadataGroup,
};
use core::ffi::c_void;

pub use doom_fish_utils::ffi_callbacks::DropCallback;

pub type MetadataItemValueRequestCallback =
    unsafe extern "C" fn(request: *mut c_void, userdata: *mut c_void);

extern "C" {
    pub fn av_metadata_item_create_payload(
        payload_payload: *const AVWMetadataItem,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut c_void;

    pub fn av_metadata_item_retain(item: *mut c_void) -> *mut c_void;

    pub fn av_metadata_item_properties_payload(
        item: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut AVWOwnedPayload;

    pub fn av_metadata_item_value_payload(
        item: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut AVWOwnedPayload;

    pub fn av_metadata_item_copy_with_data_type(
        item: *mut c_void,
        bytes: *const u8,
        count: usize,
    ) -> *mut c_void;

    pub fn av_metadata_item_copy_with_extended_language_tag(
        item: *mut c_void,
        bytes: *const u8,
        count: usize,
    ) -> *mut c_void;

    pub fn av_metadata_item_copy_with_locale_identifier(
        item: *mut c_void,
        bytes: *const u8,
        count: usize,
    ) -> *mut c_void;

    pub fn av_metadata_item_info_payload(
        item: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut AVWOwnedPayload;

    pub fn av_metadata_item_release(item: *mut c_void);

    pub fn av_metadata_item_array_count(array: *mut c_void) -> usize;

    pub fn av_metadata_item_array_item_retain(array: *mut c_void, index: usize) -> *mut c_void;

    pub fn av_metadata_item_array_release(array: *mut c_void);

    pub fn av_metadata_item_status_of_value(
        item: *mut c_void,
        key: *const AVWString,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_metadata_item_load_values_payload(
        item: *mut c_void,
        keys_payload: *const AVWArray,
        timeout_seconds: i32,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut AVWOwnedPayload;

    pub fn av_metadata_item_filter_preferred_languages_payload(
        items_payload: *const AVWArray,
        languages_payload: *const AVWArray,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut AVWOwnedPayload;

    pub fn av_metadata_item_filter_preferred_languages(
        items: *const *mut c_void,
        item_count: usize,
        languages_payload: *const AVWArray,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut c_void;

    pub fn av_metadata_item_filter_identifier_payload(
        items_payload: *const AVWArray,
        identifier: *const AVWString,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut AVWOwnedPayload;

    pub fn av_metadata_item_filter_identifier(
        items: *const *mut c_void,
        item_count: usize,
        identifier: *const AVWString,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut c_void;

    pub fn av_metadata_item_filter_metadata_item_filter_payload(
        items_payload: *const AVWArray,
        filter: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut AVWOwnedPayload;

    pub fn av_metadata_item_filter_metadata_item_filter(
        items: *const *mut c_void,
        item_count: usize,
        filter: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut c_void;

    pub fn av_metadata_item_create_lazy_payload(
        base_item_payload: *const AVWMetadataItem,
        callback: Option<MetadataItemValueRequestCallback>,
        userdata: *mut c_void,
        drop_userdata: Option<DropCallback>,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut c_void;

    pub fn av_metadata_item_create_lazy(
        base_item: *mut c_void,
        callback: Option<MetadataItemValueRequestCallback>,
        userdata: *mut c_void,
        drop_userdata: Option<DropCallback>,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut c_void;

    pub fn av_metadata_item_value_request_retain(request: *mut c_void) -> *mut c_void;

    pub fn av_metadata_item_value_request_release(request: *mut c_void);

    pub fn av_metadata_item_value_request_metadata_item_payload(
        request: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut AVWOwnedPayload;

    pub fn av_metadata_item_value_request_metadata_item(request: *mut c_void) -> *mut c_void;

    pub fn av_metadata_item_value_request_respond_with_value_payload(
        request: *mut c_void,
        value_payload: *const AVWMetadataValue,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_metadata_item_value_request_respond_with_error(
        request: *mut c_void,
        error_message: *const AVWString,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> i32;

    pub fn av_timed_metadata_group_create_payload(
        payload_payload: *const AVWTimedMetadataGroup,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut c_void;

    pub fn av_timed_metadata_group_create_items(
        items: *const *mut c_void,
        item_count: usize,
        start_value: i64,
        start_scale: i32,
        start_kind: i32,
        duration_value: i64,
        duration_scale: i32,
        duration_kind: i32,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut c_void;

    pub fn av_timed_metadata_group_retain(group: *mut c_void) -> *mut c_void;

    pub fn av_metadata_group_items(
        group: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut c_void;

    pub fn av_timed_metadata_group_properties_payload(
        group: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut AVWOwnedPayload;

    #[link_name = "avw_timed_metadata_group_info_payload"]
    pub fn av_timed_metadata_group_info_payload(
        group: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut AVWOwnedPayload;

    pub fn av_timed_metadata_group_release(group: *mut c_void);

    pub fn av_date_range_metadata_group_create_payload(
        payload_payload: *const AVWDateRangeMetadataGroup,
        mutable_group: bool,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut c_void;

    pub fn av_date_range_metadata_group_create_items(
        items: *const *mut c_void,
        item_count: usize,
        start_date: *const AVWString,
        end_date: *const AVWString,
        mutable_group: bool,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut c_void;

    pub fn av_date_range_metadata_group_retain(group: *mut c_void) -> *mut c_void;

    pub fn av_date_range_metadata_group_mutable_copy(group: *mut c_void) -> *mut c_void;

    pub fn av_date_range_metadata_group_immutable_copy(group: *mut c_void) -> *mut c_void;

    pub fn av_date_range_metadata_group_properties_payload(
        group: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut AVWOwnedPayload;

    #[link_name = "avw_date_range_metadata_group_info_payload"]
    pub fn av_date_range_metadata_group_info_payload(
        group: *mut c_void,
        out_error_message: *mut *mut AVWOwnedPayload,
    ) -> *mut AVWOwnedPayload;

    pub fn av_date_range_metadata_group_release(group: *mut c_void);
}
