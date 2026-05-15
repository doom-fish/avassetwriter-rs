//! [`Writer`] — safe wrapper around `AVAssetWriter`.

use core::ffi::{c_char, c_void};
use core::ptr;
use std::ffi::CString;
use std::path::Path;

use apple_cf::cm::CMSampleBuffer;

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
}

impl FileType {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Mov => "mov",
            Self::Mp4 => "mp4",
            Self::M4v => "m4v",
        }
    }
}

/// Identifier returned by [`Writer::add_video_input_from_sample`]. Pass this
/// back into [`Writer::append_sample`] to associate samples with the right
/// track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct InputId(i32);

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
