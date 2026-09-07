#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss
)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use apple_cf::cv::CVPixelBuffer;
use apple_cf::iosurface::{IOSurface, IOSurfaceLockOptions};
use avassetwriter::{
    AudioMix, ExportPreset, ExportSession, FileType, MetadataItem, MetadataItemFilter, Time,
    TimeRange, TrackGroupOutputHandling, VideoComposition, VideoCompositionRequest,
    VideoCompositor, VideoCompositorClass, Writer,
};
use videotoolbox::prelude::*;

const WIDTH: usize = 160;
const HEIGHT: usize = 120;
const FPS: i32 = 30;
const TOTAL_FRAMES: i32 = 4;

#[allow(clippy::too_many_lines)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let artifacts = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/example-artifacts");
    std::fs::create_dir_all(&artifacts)?;

    let source = artifacts.join("export-session-source.mp4");
    write_test_asset(&source)?;

    let presets = ExportSession::compatible_presets(&source)?;
    let preset = presets
        .iter()
        .copied()
        .find(|value| {
            matches!(
                value,
                ExportPreset::HighestQuality
                    | ExportPreset::MediumQuality
                    | ExportPreset::LowQuality
            )
        })
        .ok_or("no compatible re-encoding export preset")?;
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
    let sharing_filter = MetadataItemFilter::for_sharing()?;
    session.set_metadata_item_filter(Some(&sharing_filter))?;

    let audio_mix = AudioMix::new()?;
    session.set_audio_mix(Some(&audio_mix))?;
    println!(
        "audio mix parameters: {}",
        session
            .audio_mix()?
            .expect("audio mix should round-trip")
            .input_parameter_count()?
    );
    session.set_audio_mix(None)?;

    let video_composition = VideoComposition::from_asset(&source)?;
    let processed_requests = Arc::new(AtomicUsize::new(0));
    video_composition.install_rust_video_compositor({
        let processed_requests = Arc::clone(&processed_requests);
        move |request| {
            processed_requests.fetch_add(1, Ordering::Relaxed);
            std::thread::spawn(move || process_video_request(request));
        }
    })?;
    session.set_video_composition(Some(&video_composition))?;
    if let Some(compositor) = session.custom_video_compositor()? {
        assert_eq!(compositor.kind()?, Some(VideoCompositorClass::RustCallback));
        println!("custom compositor: {}", compositor.class_name()?);
    }

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
    session.set_video_composition(Some(&video_composition))?;
    session.set_can_perform_multiple_passes_over_source_media_data(false)?;
    session.set_directory_for_temporary_files(Some(temp_dir.as_path()))?;
    session.set_audio_track_group_handling(TrackGroupOutputHandling::None)?;
    let _ = session.estimated_maximum_duration()?;
    let _ = session.estimated_output_file_length()?;
    session.export()?;
    assert!(processed_requests.load(Ordering::Relaxed) > 0);
    if let Some(snapshot) = VideoCompositor::take_latest_video_composition_request()? {
        println!(
            "processed request at {:?} from tracks {:?}",
            snapshot.composition_time, snapshot.source_track_ids
        );
    }

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

fn process_video_request(request: VideoCompositionRequest) {
    let track_id = match request.source_track_ids() {
        Ok(track_ids) => match track_ids.into_iter().next() {
            Some(track_id) => track_id,
            None => {
                let _ = request.finish_with_error("video composition request had no source track");
                return;
            }
        },
        Err(error) => {
            let _ = request.finish_with_error(&error.to_string());
            return;
        }
    };
    let source = match request.source_frame(track_id) {
        Ok(Some(source)) => source,
        Ok(None) => {
            let _ = request.finish_with_error("video composition request had no source frame");
            return;
        }
        Err(error) => {
            let _ = request.finish_with_error(&error.to_string());
            return;
        }
    };
    let output = match request.new_output_pixel_buffer() {
        Ok(output) => output,
        Err(error) => {
            let _ = request.finish_with_error(&error.to_string());
            return;
        }
    };
    if let Err(error) = invert_bgra(&source, &output) {
        let _ = request.finish_with_error(&error);
        return;
    }
    if let Err(error) = request.finish_with_frame(&output) {
        eprintln!("failed to finish video composition request: {error}");
    }
}

fn invert_bgra(source: &CVPixelBuffer, output: &CVPixelBuffer) -> Result<(), String> {
    if source.width() != output.width() || source.height() != output.height() {
        return Err("source and output pixel-buffer sizes differ".into());
    }
    let width = source.width();
    let height = source.height();
    let source_guard = source
        .lock_read_only()
        .map_err(|code| format!("source pixel-buffer lock failed: {code}"))?;
    let mut output_guard = output
        .lock_read_write()
        .map_err(|code| format!("output pixel-buffer lock failed: {code}"))?;
    let source_stride = source_guard.bytes_per_row();
    let output_stride = output_guard.bytes_per_row();
    // SAFETY: the request exposes the source frame read-only for this processing scope.
    let source_bytes = unsafe { source_guard.as_slice() }
        .ok_or_else(|| "source pixel buffer is not contiguous BGRA".to_string())?;
    // SAFETY: the new output buffer has not been submitted and has no other mutable byte view.
    let output_bytes = unsafe { output_guard.as_slice_mut() }
        .ok_or_else(|| "output pixel buffer is not contiguous BGRA".to_string())?;
    let row_bytes = width
        .checked_mul(4)
        .ok_or_else(|| "pixel row size overflowed".to_string())?;
    for row in 0..height {
        let source_start = row
            .checked_mul(source_stride)
            .ok_or_else(|| "source row offset overflowed".to_string())?;
        let output_start = row
            .checked_mul(output_stride)
            .ok_or_else(|| "output row offset overflowed".to_string())?;
        let source_end = source_start
            .checked_add(row_bytes)
            .ok_or_else(|| "source row end overflowed".to_string())?;
        let output_end = output_start
            .checked_add(row_bytes)
            .ok_or_else(|| "output row end overflowed".to_string())?;
        let source_row = source_bytes
            .get(source_start..source_end)
            .ok_or_else(|| "source row exceeded its pixel buffer".to_string())?;
        let output_row = output_bytes
            .get_mut(output_start..output_end)
            .ok_or_else(|| "output row exceeded its pixel buffer".to_string())?;
        for (source_pixel, output_pixel) in source_row
            .chunks_exact(4)
            .zip(output_row.chunks_exact_mut(4))
        {
            output_pixel[0] = u8::MAX - source_pixel[0];
            output_pixel[1] = u8::MAX - source_pixel[1];
            output_pixel[2] = u8::MAX - source_pixel[2];
            output_pixel[3] = source_pixel[3];
        }
    }
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
    // SAFETY: this exclusive lock scope does not hand the surface to the encoder.
    let bytes = unsafe { guard.as_slice_mut() }
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
