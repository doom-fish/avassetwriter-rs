#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

use std::path::{Path, PathBuf};

use apple_cf::iosurface::{IOSurface, IOSurfaceLockOptions};
use avassetwriter::{
    ExportPreset, ExportSession, FileType, MetadataItem, Time, TimeRange, TrackGroupOutputHandling,
    Writer,
};
use videotoolbox::prelude::*;

const WIDTH: usize = 160;
const HEIGHT: usize = 120;
const FPS: i32 = 30;
const TOTAL_FRAMES: i32 = 4;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let artifacts = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/example-artifacts");
    std::fs::create_dir_all(&artifacts)?;

    let source = artifacts.join("export-session-source.mp4");
    write_test_asset(&source)?;

    let presets = ExportSession::compatible_presets(&source)?;
    let preset = presets
        .iter()
        .copied()
        .find(|value| *value == ExportPreset::Passthrough)
        .unwrap_or(presets[0]);
    let session = ExportSession::new(&source, preset)?;
    let output_type = session
        .compatible_file_types()?
        .into_iter()
        .find(|value| *value == FileType::Mp4)
        .unwrap_or(FileType::Mov);

    let output = artifacts.join(format!(
        "export-session-smoke.{}",
        extension_for_file_type(output_type)
    ));
    if output.exists() {
        std::fs::remove_file(&output)?;
    }
    let temp_dir = artifacts.join("export-session-temp");
    std::fs::create_dir_all(&temp_dir)?;

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
    let _ = session.estimated_maximum_duration()?;
    let _ = session.estimated_output_file_length()?;
    session.export()?;

    let metadata = std::fs::metadata(&output)?;
    println!(
        "exported {} -> {} using {:?} / {:?} ({} bytes)",
        source.display(),
        output.display(),
        preset,
        output_type,
        metadata.len()
    );
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
