use core::ffi::{c_char, c_void};

extern "C" {
    pub fn av_metadata_item_filter_for_sharing() -> *mut c_void;

    pub fn av_metadata_item_filter_kind(filter: *mut c_void) -> *mut c_char;

    pub fn av_metadata_item_filter_release(filter: *mut c_void);

    pub fn av_audio_mix_create() -> *mut c_void;

    pub fn av_audio_mix_info_json(mix: *mut c_void) -> *mut c_char;

    pub fn av_audio_mix_release(mix: *mut c_void);

    pub fn av_video_composition_create_from_asset(path: *const c_char) -> *mut c_void;

    pub fn av_video_composition_info_json(composition: *mut c_void) -> *mut c_char;

    pub fn av_video_composition_set_custom_video_compositor_class(
        composition: *mut c_void,
        class: *const c_char,
        out_error_message: *mut *mut c_char,
    ) -> i32;

    pub fn av_video_composition_release(composition: *mut c_void);

    pub fn av_video_compositor_info_json(compositor: *mut c_void) -> *mut c_char;

    pub fn av_video_compositor_release(compositor: *mut c_void);
}
