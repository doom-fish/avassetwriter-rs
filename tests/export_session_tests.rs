#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

use std::path::{Path, PathBuf};

use apple_cf::iosurface::{IOSurface, IOSurfaceLockOptions};
use avassetwriter::{
    AudioMix, ExportPreset, ExportSession, ExportStatus, FileType, MetadataItem,
    MetadataItemFilter, MetadataItemFilterKind, Time, TimeRange, TrackGroupOutputHandling,
    VideoComposition, VideoCompositorClass, Writer,
};
use videotoolbox::prelude::*;

const WIDTH: usize = 160;
const HEIGHT: usize = 120;
const FPS: i32 = 30;
const TOTAL_FRAMES: i32 = 4;

fn artifacts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/test-artifacts")
}

#[test]
#[allow(clippy::too_many_lines)]
fn export_session_smoke() -> Result<(), Box<dyn std::error::Error>> {
    let artifacts = artifacts_dir();
    std::fs::create_dir_all(&artifacts)?;

    let source = artifacts.join("export-source.mp4");
    write_test_asset(&source)?;

    let available = ExportSession::available_presets()?;
    assert!(!available.is_empty());

    let compatible = ExportSession::compatible_presets(&source)?;
    assert!(!compatible.is_empty());
    let preset = compatible
        .iter()
        .copied()
        .find(|value| *value == ExportPreset::Passthrough)
        .unwrap_or(compatible[0]);

    let session = ExportSession::new(&source, preset)?;
    assert_eq!(session.preset_name()?, preset);
    assert_eq!(
        session.asset_path()?,
        Some(source.to_string_lossy().into_owned())
    );

    let supported_types = session.supported_file_types()?;
    assert!(!supported_types.is_empty());

    let compatible_types = session.compatible_file_types()?;
    assert!(!compatible_types.is_empty());
    let output_type = compatible_types
        .iter()
        .copied()
        .find(|value| *value == FileType::Mp4)
        .unwrap_or(compatible_types[0]);

    assert!(ExportSession::determine_compatibility(
        &source,
        preset,
        Some(output_type)
    )?);

    let temp_dir = artifacts.join("export-temp");
    std::fs::create_dir_all(&temp_dir)?;
    let output = artifacts.join(format!(
        "exported-smoke.{}",
        extension_for_file_type(output_type)
    ));
    if output.exists() {
        std::fs::remove_file(&output)?;
    }

    session.set_output_file_type(Some(output_type))?;
    session.set_output_path(Some(output.as_path()))?;
    session.set_should_optimize_for_network_use(true)?;
    session.set_time_range(TimeRange::new(
        Time::new(0, FPS),
        Time::new(i64::from(TOTAL_FRAMES), FPS),
    ))?;
    session.set_metadata(&[MetadataItem::string(
        "mdta/com.apple.quicktime.title",
        "export session smoke",
    )])?;
    let sharing_filter = MetadataItemFilter::for_sharing()?;
    session.set_metadata_item_filter(Some(&sharing_filter))?;
    let attached_filter = session
        .metadata_item_filter()?
        .expect("metadata filter should round-trip");
    assert!(matches!(
        attached_filter.kind()?,
        MetadataItemFilterKind::Sharing
    ));

    let audio_mix = AudioMix::new()?;
    session.set_audio_mix(Some(&audio_mix))?;
    let attached_audio_mix = session.audio_mix()?.expect("audio mix should round-trip");
    assert_eq!(attached_audio_mix.input_parameter_count()?, 0);
    session.set_audio_mix(None)?;
    assert!(session.audio_mix()?.is_none());

    let video_composition = VideoComposition::from_asset(&source)?;
    assert!(video_composition.instruction_count()? > 0);
    let (render_width, render_height) = video_composition.render_size()?;
    assert!(render_width > 0.0);
    assert!(render_height > 0.0);
    video_composition.set_custom_video_compositor_class(Some(VideoCompositorClass::Passthrough))?;
    session.set_video_composition(Some(&video_composition))?;
    let attached_composition = session
        .video_composition()?
        .expect("video composition should round-trip");
    assert_eq!(
        attached_composition.custom_video_compositor_class()?,
        Some(VideoCompositorClass::Passthrough)
    );
    let compositor = session
        .custom_video_compositor()?
        .expect("custom video compositor should be created");
    assert_eq!(compositor.kind()?, Some(VideoCompositorClass::Passthrough));
    assert!(compositor.source_pixel_buffer_attributes()?.is_some());
    assert!(compositor
        .required_pixel_buffer_attributes_for_render_context()?
        .is_some());
    session.set_video_composition(None)?;
    assert!(session.video_composition()?.is_none());
    assert!(session.custom_video_compositor()?.is_none());

    let session = ExportSession::new(&source, preset)?;
    session.set_output_file_type(Some(output_type))?;
    session.set_output_path(Some(output.as_path()))?;
    session.set_should_optimize_for_network_use(true)?;
    session.set_time_range(TimeRange::new(
        Time::new(0, FPS),
        Time::new(i64::from(TOTAL_FRAMES), FPS),
    ))?;
    session.set_metadata(&[MetadataItem::string(
        "mdta/com.apple.quicktime.title",
        "export session smoke",
    )])?;
    session.set_can_perform_multiple_passes_over_source_media_data(false)?;
    session.set_directory_for_temporary_files(Some(temp_dir.as_path()))?;
    session.set_audio_track_group_handling(TrackGroupOutputHandling::None)?;

    match session.set_allows_parallelized_export(false) {
        Ok(()) => assert!(!session.allows_parallelized_export()?),
        Err(avassetwriter::AVWriterError::InvalidState(_)) => {}
        Err(error) => return Err(Box::new(error)),
    }

    assert_eq!(
        session.directory_for_temporary_files()?,
        Some(temp_dir.to_string_lossy().into_owned())
    );
    assert_eq!(
        session.audio_track_group_handling()?,
        TrackGroupOutputHandling::None
    );
    assert_eq!(session.metadata()?.len(), 1);
    let _ = session.estimated_maximum_duration()?;
    let _ = session.estimated_output_file_length()?;

    session.export()?;
    assert_eq!(session.status()?, ExportStatus::Completed);
    assert_eq!(session.error_message()?, None);

    let metadata = std::fs::metadata(&output)?;
    assert!(metadata.len() > 0);
    Ok(())
}

fn write_test_asset(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if path.exists() {
        std::fs::remove_file(path)?;
    }

    let surface = IOSurface::create(WIDTH, HEIGHT, u32::from_be_bytes(*b"BGRA"), 4)
        .ok_or("failed to create IOSurface")?;
    let encoder = CompressionSession::builder(WIDTH as i32, HEIGHT as i32, Codec::H264)
        .with_real_time(true)
        .with_average_bit_rate(200_000)
        .with_expected_frame_rate(f64::from(FPS))
        .with_max_keyframe_interval(FPS)
        .build()?;

    fill_surface(&surface, 0)?;
    let first_frame = encoder.encode(&surface, (0, FPS))?;

    let writer = Writer::create(path, FileType::Mp4)?;
    let input = writer.add_video_input_from_sample(
        first_frame
            .cm_sample_buffer()
            .expect("first encoded frame should have a CMSampleBuffer"),
    )?;
    writer.start_session((0, FPS))?;
    writer.append_sample(
        input,
        first_frame
            .cm_sample_buffer()
            .expect("first encoded frame should have a CMSampleBuffer"),
    )?;

    for frame_index in 1..TOTAL_FRAMES {
        fill_surface(&surface, frame_index)?;
        let frame = encoder.encode(&surface, (i64::from(frame_index), FPS))?;
        writer.append_sample(
            input,
            frame
                .cm_sample_buffer()
                .expect("encoded frame should have a CMSampleBuffer"),
        )?;
    }

    writer.finish()?;
    Ok(())
}

fn fill_surface(surface: &IOSurface, frame_idx: i32) -> Result<(), String> {
    let mut guard = surface
        .lock(IOSurfaceLockOptions::NONE)
        .map_err(|code| format!("lock failed: {code}"))?;
    let bytes = guard
        .as_slice_mut()
        .ok_or_else(|| "surface storage was not contiguous".to_string())?;
    let green = (frame_idx * 17).clamp(0, 255) as u8;
    for pixel in bytes.chunks_exact_mut(4) {
        pixel[0] = 0x20;
        pixel[1] = green;
        pixel[2] = 0x90;
        pixel[3] = 0xFF;
    }
    Ok(())
}

const fn extension_for_file_type(file_type: FileType) -> &'static str {
    match file_type {
        FileType::Mp4 => "mp4",
        FileType::M4v => "m4v",
        FileType::M4a => "m4a",
        _ => "mov",
    }
}
