#![doc = include_str!("../README.md")]
//!
//! ---
//!
//! # API Documentation
//!
//! Safe Rust bindings for Apple's
//! [AVAssetWriter](https://developer.apple.com/documentation/avfoundation/avassetwriter)
//! — mux compressed video (and eventually audio) into `.mp4` / `.mov` / `.m4v`
//! files on macOS.
//!
//! Designed to consume `CMSampleBuffer`s directly from
//! [`videotoolbox`](https://github.com/doom-fish/videotoolbox-rs) so the
//! recording pipeline (capture → encode → mux) stays zero-copy from
//! IOSurface all the way to disk.
//!
//! # Quick start
//!
//! ```rust,no_run
//! use avassetwriter::{FileType, Writer};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let cm_sample_buffer_ptr: *mut std::ffi::c_void = std::ptr::null_mut();
//! let writer = Writer::create("/tmp/out.mp4", FileType::Mp4)?;
//! let input_id = writer.add_video_input_from_sample(cm_sample_buffer_ptr)?;
//! writer.start_session((0, 60))?;
//! writer.append_sample(input_id, cm_sample_buffer_ptr)?;
//! writer.finish()?;
//! # Ok(())
//! # }
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod error;
pub mod ffi;
pub mod writer;

pub use error::AVWriterError;
pub use writer::{FileType, InputId, Writer};

/// Common imports.
pub mod prelude {
    pub use crate::error::AVWriterError;
    pub use crate::writer::{FileType, InputId, Writer};
}
