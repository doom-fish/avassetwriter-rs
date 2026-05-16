#![doc = include_str!("../README.md")]
//!
//! ---
//!
//! # API Documentation
//!
//! Safe Rust bindings for Apple's
//! [AVAssetWriter](https://developer.apple.com/documentation/avfoundation/avassetwriter)
//! — mux compressed video, audio, metadata, and related writer-input surfaces
//! into `.mp4` / `.mov` / `.m4v` files on macOS.
//!
//! Designed to consume `CMSampleBuffer`s directly from
//! [`videotoolbox`](https://github.com/doom-fish/videotoolbox-rs) so the
//! recording pipeline (capture → encode → mux) stays zero-copy from
//! `IOSurface` all the way to disk.
//!
//! # Quick start
//!
//! ```rust,no_run
//! use std::path::PathBuf;
//!
//! use avassetwriter::{FileType, Writer};
//! use apple_cf::cm::CMSampleBuffer;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let sample: &CMSampleBuffer = unreachable!("doctest stub");
//! let artifacts = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/example-artifacts");
//! std::fs::create_dir_all(&artifacts)?;
//! let output = artifacts.join("out.mp4");
//! let writer = Writer::create(&output, FileType::Mp4)?;
//! let input_id = writer.add_video_input_from_sample(sample)?;
//! writer.start_session((0, 60))?;
//! writer.append_sample(input_id, sample)?;
//! writer.finish()?;
//! # Ok(())
//! # }
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]

mod callbacks;
pub mod caption;
pub mod error;
pub mod ffi;
pub mod metadata;
pub mod time;
pub mod writer;

pub use caption::{Caption, CaptionGroup};
pub use error::AVWriterError;
pub use metadata::{MetadataItem, MetadataSpecification, MetadataValue, TimedMetadataGroup};
pub use time::{Time, TimeRange};
pub use writer::{
    FileType, FileTypeProfile, InputGroupInfo, InputId, InputMediaDataLocation,
    InputPassDescription, MediaType, SegmentOutput, SegmentReport, SegmentReportSampleInfo,
    SegmentTrackReport, SegmentType, TaggedPixelBuffer, TrackAssociationType, VideoPreset, Writer,
    WriterStatus,
};

/// Common imports.
pub mod prelude {
    pub use crate::caption::{Caption, CaptionGroup};
    pub use crate::error::AVWriterError;
    pub use crate::metadata::{
        MetadataItem, MetadataSpecification, MetadataValue, TimedMetadataGroup,
    };
    pub use crate::time::{Time, TimeRange};
    pub use crate::writer::{
        FileType, FileTypeProfile, InputGroupInfo, InputId, InputMediaDataLocation,
        InputPassDescription, MediaType, TaggedPixelBuffer, TrackAssociationType, Writer,
        WriterStatus,
    };
}
