//! [`Writer`] — safe wrapper around `AVAssetWriter`.

use core::ffi::{c_char, c_void};
use core::ptr;
use std::ffi::CString;
use std::path::Path;

use apple_cf::cm::CMSampleBuffer;

mod extended;

pub use extended::{
    FileTypeProfile, InputGroupInfo, InputMediaDataLocation, InputPassDescription, MediaType,
    SegmentOutput, SegmentReport, SegmentReportSampleInfo, SegmentTrackReport, SegmentType,
    TaggedPixelBuffer, TrackAssociationType, WriterStatus,
};

use crate::error::{from_swift, AVWriterError};
use crate::ffi;

/// Container file format for the output file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FileType {
    /// `QuickTime` `.mov`
    Mov,
    /// MPEG-4 Part 14 `.mp4`
    Mp4,
    /// iTunes `.m4v`
    M4v,
    /// iTunes `.m4a`
    M4a,
    /// 3GPP `.3gp`
    ThreeGpp,
    /// 3GPP2 `.3g2`
    ThreeGpp2,
    /// Core Audio Format `.caf`
    Caf,
    /// WAVE `.wav`
    Wav,
    /// AIFF `.aiff`
    Aiff,
    /// AIFC `.aifc`
    Aifc,
    /// AMR `.amr`
    Amr,
    /// MP3 `.mp3`
    Mp3,
    /// Sun/NeXT AU `.au`
    SunAu,
    /// AC-3 `.ac3`
    Ac3,
    /// Enhanced AC-3 `.eac3`
    Eac3,
    /// JPEG `.jpg`
    Jpeg,
    /// DNG `.dng`
    Dng,
    /// HEIC `.heic`
    Heic,
    /// AVCI `.avci`
    Avci,
    /// HEIF `.heif`
    Heif,
    /// TIFF `.tiff`
    Tiff,
    /// Apple iTT `.itt`
    AppleItt,
    /// Scenarist SCC `.scc`
    Scc,
    /// Apple Haptics `.ahap`
    Ahap,
    /// `QuickTime` audio.
    QuickTimeAudio,
    /// DICOM.
    Dicom,
}

impl FileType {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Mov => "mov",
            Self::Mp4 => "mp4",
            Self::M4v => "m4v",
            Self::M4a => "m4a",
            Self::ThreeGpp => "3gpp",
            Self::ThreeGpp2 => "3gpp2",
            Self::Caf => "caf",
            Self::Wav => "wav",
            Self::Aiff => "aiff",
            Self::Aifc => "aifc",
            Self::Amr => "amr",
            Self::Mp3 => "mp3",
            Self::SunAu => "au",
            Self::Ac3 => "ac3",
            Self::Eac3 => "eac3",
            Self::Jpeg => "jpeg",
            Self::Dng => "dng",
            Self::Heic => "heic",
            Self::Avci => "avci",
            Self::Heif => "heif",
            Self::Tiff => "tiff",
            Self::AppleItt => "itt",
            Self::Scc => "scc",
            Self::Ahap => "ahap",
            Self::QuickTimeAudio => "qt_audio",
            Self::Dicom => "dicom",
        }
    }

    pub(crate) fn from_raw(raw: &str) -> Option<Self> {
        match raw {
            "mov" => Some(Self::Mov),
            "mp4" => Some(Self::Mp4),
            "m4v" => Some(Self::M4v),
            "m4a" => Some(Self::M4a),
            "3gpp" => Some(Self::ThreeGpp),
            "3gpp2" => Some(Self::ThreeGpp2),
            "caf" => Some(Self::Caf),
            "wav" => Some(Self::Wav),
            "aiff" => Some(Self::Aiff),
            "aifc" => Some(Self::Aifc),
            "amr" => Some(Self::Amr),
            "mp3" => Some(Self::Mp3),
            "au" => Some(Self::SunAu),
            "ac3" => Some(Self::Ac3),
            "eac3" => Some(Self::Eac3),
            "jpeg" => Some(Self::Jpeg),
            "dng" => Some(Self::Dng),
            "heic" => Some(Self::Heic),
            "avci" => Some(Self::Avci),
            "heif" => Some(Self::Heif),
            "tiff" => Some(Self::Tiff),
            "itt" => Some(Self::AppleItt),
            "scc" => Some(Self::Scc),
            "ahap" => Some(Self::Ahap),
            "qt_audio" => Some(Self::QuickTimeAudio),
            "dicom" => Some(Self::Dicom),
            _ => None,
        }
    }
}

/// Identifier returned by [`Writer::add_video_input_from_sample`]. Pass this
/// back into [`Writer::append_sample`] to associate samples with the right
/// track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InputId(i32);

/// One of Apple's `AVOutputSettingsPreset*` named export presets.
/// Use with [`Writer::add_video_input_from_preset`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
#[non_exhaustive]
pub enum VideoPreset {
    /// 640x480 SD — H.264.
    Sd640x480 = 0,
    /// 960x540 — H.264.
    Hd960x540 = 1,
    /// 1280x720 HD — H.264.
    Hd1280x720 = 2,
    /// 1920x1080 Full HD — H.264.
    FullHd1920x1080 = 3,
    /// 3840x2160 4K UHD — H.264.
    Uhd3840x2160 = 4,
    /// 1920x1080 Full HD — HEVC (H.265).
    Hevc1920x1080 = 5,
    /// 1920x1080 Full HD — HEVC with alpha.
    Hevc1920x1080WithAlpha = 6,
    /// 3840x2160 4K UHD — HEVC (H.265).
    Hevc3840x2160 = 7,
    /// 3840x2160 4K UHD — HEVC with alpha.
    Hevc3840x2160WithAlpha = 8,
    /// 4320x2160 — HEVC.
    Hevc4320x2160 = 9,
    /// 7680x4320 — HEVC.
    Hevc7680x4320 = 10,
    /// 960x960 — multiview HEVC.
    MvHevc960x960 = 11,
    /// 1440x1440 — multiview HEVC.
    MvHevc1440x1440 = 12,
    /// 4320x4320 — multiview HEVC.
    MvHevc4320x4320 = 13,
    /// 7680x7680 — multiview HEVC.
    MvHevc7680x7680 = 14,
}

impl VideoPreset {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Sd640x480 => "640x480",
            Self::Hd960x540 => "960x540",
            Self::Hd1280x720 => "1280x720",
            Self::FullHd1920x1080 => "1920x1080",
            Self::Uhd3840x2160 => "3840x2160",
            Self::Hevc1920x1080 => "hevc_1920x1080",
            Self::Hevc1920x1080WithAlpha => "hevc_1920x1080_with_alpha",
            Self::Hevc3840x2160 => "hevc_3840x2160",
            Self::Hevc3840x2160WithAlpha => "hevc_3840x2160_with_alpha",
            Self::Hevc4320x2160 => "hevc_4320x2160",
            Self::Hevc7680x4320 => "hevc_7680x4320",
            Self::MvHevc960x960 => "mvhevc_960x960",
            Self::MvHevc1440x1440 => "mvhevc_1440x1440",
            Self::MvHevc4320x4320 => "mvhevc_4320x4320",
            Self::MvHevc7680x7680 => "mvhevc_7680x7680",
        }
    }

    pub(crate) fn from_raw(raw: &str) -> Option<Self> {
        Some(match raw {
            "640x480" => Self::Sd640x480,
            "960x540" => Self::Hd960x540,
            "1280x720" => Self::Hd1280x720,
            "1920x1080" => Self::FullHd1920x1080,
            "3840x2160" => Self::Uhd3840x2160,
            "hevc_1920x1080" => Self::Hevc1920x1080,
            "hevc_1920x1080_with_alpha" => Self::Hevc1920x1080WithAlpha,
            "hevc_3840x2160" => Self::Hevc3840x2160,
            "hevc_3840x2160_with_alpha" => Self::Hevc3840x2160WithAlpha,
            "hevc_4320x2160" => Self::Hevc4320x2160,
            "hevc_7680x4320" => Self::Hevc7680x4320,
            "mvhevc_960x960" => Self::MvHevc960x960,
            "mvhevc_1440x1440" => Self::MvHevc1440x1440,
            "mvhevc_4320x4320" => Self::MvHevc4320x4320,
            "mvhevc_7680x7680" => Self::MvHevc7680x7680,
            _ => return None,
        })
    }
}

/// `AVAssetWriter` wrapper.
///
/// # Lifecycle
///
/// 1. Construct with [`Writer::create`].
/// 2. Add one input per track via [`Writer::add_video_input_from_sample`].
/// 3. Call [`Writer::start_session`] with the timestamp of the first sample.
/// 4. Append samples via [`Writer::append_sample`] in monotonically
///    increasing presentation-time order.
/// 5. Call [`Writer::finish`] to flush and finalise the file. Blocks until
///    the asynchronous `finishWriting` completion handler fires.
pub struct Writer {
    ptr: *mut c_void,
}

// SAFETY: Writer wraps an opaque retained AVAssetWriter ref-counted pointer.
// All actual mutation happens on AVFoundation's internal queues — the Rust
// side merely shuttles the pointer across the FFI boundary.
unsafe impl Send for Writer {}
unsafe impl Sync for Writer {}

impl Writer {
    /// Returns the raw Swift object pointer (for use by `async_api`).
    pub(crate) const fn as_raw_ptr(&self) -> *mut c_void {
        self.ptr
    }

    /// Create a writer that will produce a file at `path` of type `file_type`.
    ///
    /// If a file already exists at `path` it will be removed first
    /// (`AVAssetWriter` refuses to overwrite).
    ///
    /// # Errors
    ///
    /// Returns [`AVWriterError::InvalidArgument`] if `path` contains an
    /// interior NUL byte, or [`AVWriterError::WriterCreateFailed`] if
    /// `AVAssetWriter` rejects the destination URL.
    ///
    /// # Panics
    ///
    /// Panics if [`FileType::as_str`] yields a string containing an
    /// interior NUL byte — this is structurally impossible for the
    /// hand-built constants in [`FileType`].
    pub fn create(path: impl AsRef<Path>, file_type: FileType) -> Result<Self, AVWriterError> {
        let path_str = path
            .as_ref()
            .to_str()
            .ok_or_else(|| AVWriterError::InvalidArgument("path is not valid UTF-8".into()))?;
        let path_c = CString::new(path_str)
            .map_err(|e| AVWriterError::InvalidArgument(format!("path NUL byte: {e}")))?;
        let type_c = CString::new(file_type.as_str()).expect("file type strings are NUL-free");

        let mut err_msg: *mut c_char = ptr::null_mut();
        let ptr = unsafe { ffi::av_writer_create(path_c.as_ptr(), type_c.as_ptr(), &mut err_msg) };
        if ptr.is_null() {
            return Err(unsafe { from_swift(ffi::status::WRITER_CREATE_FAILED, err_msg) });
        }
        Ok(Self { ptr })
    }

    /// Add a video input whose format is inferred from the supplied
    /// [`CMSampleBuffer`]. The sample buffer is used **only** to read the
    /// format description — call [`Writer::append_sample`] to actually
    /// write data.
    ///
    /// `sample_buffer` is typically obtained from
    /// `videotoolbox::EncodedFrame::cm_sample_buffer()`.
    ///
    /// # Errors
    ///
    /// Returns [`AVWriterError::InvalidArgument`] if the sample buffer has
    /// no format description, or [`AVWriterError::InvalidState`] if the
    /// writer cannot accept any more inputs.
    pub fn add_video_input_from_sample(
        &self,
        sample_buffer: &CMSampleBuffer,
    ) -> Result<InputId, AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let result = unsafe {
            ffi::av_writer_add_video_input_from_sample(
                self.ptr,
                sample_buffer.as_ptr(),
                &mut err_msg,
            )
        };
        if result < 0 {
            return Err(unsafe { from_swift(result, err_msg) });
        }
        Ok(InputId(result))
    }

    /// Add a video input pre-configured by `AVOutputSettingsAssistant`
    /// for one of Apple's named export presets. Each preset bakes in
    /// the recommended codec, bitrate, frame rate, profile, and pixel
    /// dimensions for that resolution/codec class.
    ///
    /// # Errors
    ///
    /// Returns [`AVWriterError::InvalidArgument`] for unknown presets,
    /// [`AVWriterError::InvalidState`] if the writer rejects the input.
    pub fn add_video_input_from_preset(
        &self,
        preset: VideoPreset,
    ) -> Result<InputId, AVWriterError> {
        let preset_c = CString::new(preset.as_str()).map_err(|e| {
            AVWriterError::InvalidArgument(format!("preset string contained NUL byte: {e}"))
        })?;
        let mut err_msg: *mut c_char = ptr::null_mut();
        let result = unsafe {
            ffi::av_writer_add_video_input_from_preset(self.ptr, preset_c.as_ptr(), &mut err_msg)
        };
        if result < 0 {
            return Err(unsafe { from_swift(result, err_msg) });
        }
        Ok(InputId(result))
    }

    /// Add an audio input that will mux interleaved little-endian
    /// signed-integer linear-PCM samples and transcode them to AAC
    /// (128 kbps) on its way into the output container.
    ///
    /// * `sample_rate` — source PCM sample rate, e.g. `48_000` or `44_100` Hz
    /// * `channels`    — 1 (mono) … 8
    /// * `bits_per_sample` — must be 16 or 32
    ///
    /// Use the returned [`InputId`] with [`Writer::append_audio_pcm`].
    ///
    /// # Errors
    ///
    /// Returns [`AVWriterError::InvalidArgument`] for out-of-range
    /// parameters, [`AVWriterError::InvalidState`] if the writer cannot
    /// accept additional inputs.
    pub fn add_audio_input_pcm(
        &self,
        sample_rate: f64,
        channels: u32,
        bits_per_sample: u32,
    ) -> Result<InputId, AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let result = unsafe {
            ffi::av_writer_add_audio_input_pcm(
                self.ptr,
                sample_rate,
                channels,
                bits_per_sample,
                &mut err_msg,
            )
        };
        if result < 0 {
            return Err(unsafe { from_swift(result, err_msg) });
        }
        Ok(InputId(result))
    }

    /// Begin writing. Subsequent [`Writer::append_sample`] calls will produce
    /// output starting at `source_time` (numerator, timescale) — typically
    /// the presentation time of your first sample.
    ///
    /// # Errors
    ///
    /// Returns [`AVWriterError::StartFailed`] if `AVAssetWriter` rejects the
    /// session start (usually because no inputs were added or one is
    /// misconfigured).
    pub fn start_session(&self, source_time: (i64, i32)) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_start_session(self.ptr, source_time.0, source_time.1, &mut err_msg)
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(ffi::status::WRITER_CREATE_FAILED, err_msg) });
        }
        Ok(())
    }

    /// Append a single [`CMSampleBuffer`] to the input identified by `input_id`.
    ///
    /// The sample buffer is typically obtained from
    /// `videotoolbox::EncodedFrame::cm_sample_buffer()`. Samples must be
    /// appended in monotonically increasing presentation-time order.
    ///
    /// # Errors
    ///
    /// Returns [`AVWriterError::InputNotReady`] when the input is
    /// back-pressuring (caller should retry shortly), or
    /// [`AVWriterError::AppendFailed`] for permanent failures.
    pub fn append_sample(
        &self,
        input_id: InputId,
        sample_buffer: &CMSampleBuffer,
    ) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_append_sample(self.ptr, input_id.0, sample_buffer.as_ptr(), &mut err_msg)
        };
        match status {
            ffi::status::OK => Ok(()),
            ffi::status::INPUT_NOT_READY => Err(AVWriterError::InputNotReady),
            other => Err(unsafe { from_swift(other, err_msg) }),
        }
    }

    /// Append `frame_count` PCM frames (each frame = `channels` samples) to
    /// an audio input previously created via [`Writer::add_audio_input_pcm`].
    ///
    /// `pcm_bytes` must contain `frame_count * channels * (bits_per_sample / 8)`
    /// bytes of interleaved little-endian signed-integer PCM data, where
    /// `channels` and `bits_per_sample` match the values passed to
    /// `add_audio_input_pcm`.
    ///
    /// `pts` is `(value, timescale)` of the first frame; `timescale` typically
    /// equals the configured `sample_rate` so each `value` increment moves
    /// forward by one frame.
    ///
    /// # Errors
    ///
    /// Returns [`AVWriterError::InputNotReady`] when the input is back-
    /// pressuring, [`AVWriterError::AppendFailed`] if the underlying
    /// `CMSampleBuffer` construction or append fails.
    pub fn append_audio_pcm(
        &self,
        input_id: InputId,
        pcm_bytes: &[u8],
        frame_count: usize,
        pts: (i64, i32),
    ) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_append_audio_pcm(
                self.ptr,
                input_id.0,
                pcm_bytes.as_ptr(),
                pcm_bytes.len(),
                frame_count,
                pts.0,
                pts.1,
                &mut err_msg,
            )
        };
        match status {
            ffi::status::OK => Ok(()),
            ffi::status::INPUT_NOT_READY => Err(AVWriterError::InputNotReady),
            other => Err(unsafe { from_swift(other, err_msg) }),
        }
    }

    /// Finalise the file. Marks all inputs as finished, blocks until the
    /// asynchronous `finishWriting` completion handler fires, then returns.
    ///
    /// # Errors
    ///
    /// Returns [`AVWriterError::FinishFailed`] if the writer ends in the
    /// `.failed` or `.cancelled` state.
    pub fn finish(self) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe { ffi::av_writer_finish(self.ptr, &mut err_msg) };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    /// Add a video input backed by an `AVAssetWriterInputPixelBufferAdaptor`
    /// for zero-copy `CVPixelBuffer` ingest. Use this when frames come
    /// from your own renderer (Metal, Core Image, …) rather than a
    /// pre-encoded `CMSampleBuffer`.
    ///
    /// `pixel_format_type` is a `kCVPixelFormatType_*` `FourCC` — typically
    /// `0x4247_5241` for `'BGRA'`.
    ///
    /// # Errors
    ///
    /// Returns [`AVWriterError::InvalidState`] if the writer rejects the
    /// new input.
    pub fn add_video_input_pixel_buffer(
        &self,
        width: i32,
        height: i32,
        pixel_format_type: u32,
    ) -> Result<InputId, AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let id = unsafe {
            ffi::av_writer_add_video_input_pixel_buffer(
                self.ptr,
                width,
                height,
                pixel_format_type,
                &mut err_msg,
            )
        };
        if id < 0 {
            return Err(unsafe { from_swift(id, err_msg) });
        }
        Ok(InputId(id))
    }

    /// Append a `CVPixelBuffer` to a pixel-buffer-adaptor-backed input.
    /// Returns [`AVWriterError::InputNotReady`] when the writer is
    /// momentarily back-pressured — the caller should retry after a few ms.
    ///
    /// `pts` is the presentation time as `(value, timescale)`.
    ///
    /// # Errors
    ///
    /// See [`AVWriterError`] variants.
    pub fn append_pixel_buffer(
        &self,
        input_id: InputId,
        pixel_buffer: &apple_cf::cv::CVPixelBuffer,
        pts: (i64, i32),
    ) -> Result<(), AVWriterError> {
        let mut err_msg: *mut c_char = ptr::null_mut();
        let status = unsafe {
            ffi::av_writer_append_pixel_buffer(
                self.ptr,
                input_id.0,
                pixel_buffer.as_ptr(),
                pts.0,
                pts.1,
                &mut err_msg,
            )
        };
        if status != ffi::status::OK {
            return Err(unsafe { from_swift(status, err_msg) });
        }
        Ok(())
    }

    /// Set `AVAssetWriter.shouldOptimizeForNetworkUse` — adds the
    /// moov atom at the start of the file so HTTP players can start
    /// playback immediately.
    pub fn set_optimize_for_network_use(&self, enabled: bool) {
        unsafe { ffi::av_writer_set_should_optimize_for_network_use(self.ptr, enabled) };
    }

    /// Set `AVAssetWriter.movieFragmentInterval` (seconds, or `0` to
    /// disable). Producing fragmented files yields safer recordings
    /// — if your process crashes mid-record the file is still
    /// playable up to the last fragment boundary.
    pub fn set_movie_fragment_interval_seconds(&self, seconds: f64) {
        unsafe { ffi::av_writer_set_movie_fragment_interval_seconds(self.ptr, seconds) };
    }

    /// Group inputs as mutually exclusive — e.g. multiple audio
    /// tracks where only one plays at a time. `inputs` are the
    /// [`InputId`]s from prior `add_*_input*` calls. `default_id`
    /// is the input that plays by default; pass any out-of-range
    /// value for "no default".
    ///
    /// # Errors
    ///
    /// Returns [`AVWriterError::InvalidArgument`] / [`AVWriterError::InvalidState`].
    pub fn add_input_group(
        &self,
        inputs: &[InputId],
        default_id: Option<InputId>,
    ) -> Result<(), AVWriterError> {
        let ids: Vec<i32> = inputs.iter().map(|i| i.0).collect();
        let default = default_id.map_or(-1, |i| i.0);
        let mut err_msg: *mut c_char = ptr::null_mut();
        let ok = unsafe {
            ffi::av_writer_add_input_group(self.ptr, ids.as_ptr(), ids.len(), default, &mut err_msg)
        };
        if !ok {
            return Err(unsafe { from_swift(ffi::status::INVALID_ARGUMENT, err_msg) });
        }
        Ok(())
    }
}

impl Drop for Writer {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe { ffi::av_writer_release(self.ptr) };
        }
    }
}

impl core::fmt::Debug for Writer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Writer").field("ptr", &self.ptr).finish()
    }
}
