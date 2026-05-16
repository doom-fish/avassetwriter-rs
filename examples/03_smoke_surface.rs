//! Smoke test for the broader `AVAssetWriter` surface without requiring external
//! media assets.
//!
//! Verifies multi-track setup, metadata/caption inputs, readback/configuration,
//! input grouping, and segmented-writer construction.
//!
//! Run with: `cargo run --example 03_smoke_surface`

#![allow(clippy::too_many_lines)]

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use avassetwriter::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let artifacts = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/example-artifacts");
    std::fs::create_dir_all(&artifacts)?;

    let output = artifacts.join("avassetwriter_surface.mov");
    if output.exists() {
        std::fs::remove_file(&output)?;
    }

    let temp_dir = artifacts.join("avassetwriter-temp");
    std::fs::create_dir_all(&temp_dir)?;

    println!("create writer");
    let writer = Writer::create(&output, FileType::Mov)?;
    println!("configure writer");
    writer.set_optimize_for_network_use(true);
    writer.set_directory_for_temporary_files(&temp_dir)?;
    writer.set_movie_fragment_interval((1, 1))?;
    writer.set_initial_movie_fragment_interval((1, 2))?;
    writer.set_initial_movie_fragment_sequence_number(7)?;
    writer.set_produces_combinable_fragments(true)?;
    writer.set_overall_duration_hint((120, 60))?;
    writer.set_movie_time_scale(600)?;
    writer.set_metadata(&[MetadataItem::string(
        "mdta/com.apple.quicktime.title",
        "avassetwriter surface smoke",
    )])?;

    println!("add video inputs");
    let primary_video =
        writer.add_video_input_pixel_buffer(320, 240, u32::from_be_bytes(*b"BGRA"))?;
    let secondary_video =
        writer.add_video_input_pixel_buffer(320, 240, u32::from_be_bytes(*b"BGRA"))?;
    writer.add_input_group(&[primary_video, secondary_video], Some(primary_video))?;

    println!("add metadata input");
    let metadata_input = writer.add_metadata_input(
        &[MetadataSpecification {
            identifier: "mdta/com.apple.quicktime.title".into(),
            data_type: "com.apple.metadata.datatype.UTF-8".into(),
            extended_language_tag: Some("en-US".into()),
        }],
        false,
    )?;
    println!("add caption input");
    let caption_input = writer.add_caption_input(&MediaType::Text, false)?;

    println!("query writer state");
    assert_eq!(writer.status()?, WriterStatus::Unknown);
    assert_eq!(
        writer.output_path()?,
        Some(output.to_string_lossy().into_owned())
    );
    assert_eq!(writer.output_file_type()?, Some(FileType::Mov));
    assert!(writer.should_optimize_for_network_use()?);
    assert_eq!(
        writer.directory_for_temporary_files()?,
        Some(temp_dir.to_string_lossy().into_owned())
    );
    assert_eq!(writer.movie_fragment_interval()?, Time::new(1, 1));
    assert_eq!(writer.initial_movie_fragment_interval()?, Time::new(1, 2));
    assert_eq!(writer.initial_movie_fragment_sequence_number()?, 7);
    assert!(writer.produces_combinable_fragments()?);
    assert_eq!(writer.overall_duration_hint()?, Time::new(120, 60));
    assert_eq!(writer.movie_time_scale()?, 600);
    assert!(writer.available_media_types()?.contains(&MediaType::Video));
    assert_eq!(writer.inputs()?.len(), 4);
    assert_eq!(writer.input_groups()?.len(), 1);
    assert_eq!(writer.input_media_type(primary_video)?, MediaType::Video);
    assert_eq!(
        writer.input_media_type(metadata_input)?,
        MediaType::Metadata
    );
    assert_eq!(writer.input_media_type(caption_input)?, MediaType::Text);
    let _ = writer.pixel_buffer_pool(primary_video)?;
    assert!(writer.input_output_settings(primary_video)?.is_some());
    assert!(writer.input_has_metadata_adaptor(metadata_input)?);
    assert!(writer.input_has_caption_adaptor(caption_input)?);
    assert_eq!(writer.metadata()?.len(), 1);
    if writer.can_add_track_association(
        primary_video,
        secondary_video,
        &TrackAssociationType::Timecode,
    )? {
        writer.add_track_association(
            primary_video,
            secondary_video,
            &TrackAssociationType::Timecode,
        )?;
    }

    println!("create segmented writer");
    let segment_count = Arc::new(Mutex::new(0usize));
    let on_segment_count = Arc::clone(&segment_count);
    let segmented = Writer::create_segmented(FileType::Mp4, None, move |_| {
        *on_segment_count.lock().expect("segment mutex poisoned") += 1;
    })?;
    segmented.set_preferred_output_segment_interval((2, 1))?;
    segmented.set_initial_segment_start_time((0, 1))?;
    assert_eq!(segmented.status()?, WriterStatus::Unknown);
    assert_eq!(segmented.output_file_type()?, Some(FileType::Mp4));
    assert_eq!(segmented.output_file_type_profile()?, None);
    assert_eq!(
        segmented.preferred_output_segment_interval()?,
        Time::new(2, 1)
    );
    assert_eq!(segmented.initial_segment_start_time()?, Time::new(0, 1));
    assert_eq!(*segment_count.lock().expect("segment mutex poisoned"), 0);

    println!(
        "surface smoke ok: writer={} inputs={} segmented_profile={:?}",
        output.display(),
        writer.inputs()?.len(),
        segmented.output_file_type_profile()?
    );

    Ok(())
}
