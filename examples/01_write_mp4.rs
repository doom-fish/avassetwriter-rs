//! Smoke test: encode 60 frames with `videotoolbox-rs` and mux them into
//! `target/example-artifacts/avassetwriter_smoke.mp4`. Verifies the full
//!

#![allow(clippy::similar_names)]
//!   `IOSurface` → `VideoToolbox` → `AVAssetWriter` → `.mp4` file
//!
//! pipeline end-to-end.
//!
//! Run with: `cargo run --example 01_write_mp4`

use std::path::PathBuf;

use apple_cf::iosurface::{IOSurface, IOSurfaceLockOptions};
use avassetwriter::prelude::*;
use videotoolbox::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let width = 640;
    let height = 480;
    let pixel_format = u32::from_be_bytes(*b"BGRA");
    let fps = 30;
    let total_frames: i32 = 60;
    let artifacts = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/example-artifacts");
    std::fs::create_dir_all(&artifacts)?;
    let output = artifacts.join("avassetwriter_smoke.mp4");
    if output.exists() {
        std::fs::remove_file(&output)?;
    }

    let surface = IOSurface::create(
        usize::try_from(width)?,
        usize::try_from(height)?,
        pixel_format,
        4,
    )
    .ok_or("failed to allocate IOSurface")?;

    let encoder = CompressionSession::builder(width, height, Codec::H264)
        .with_real_time(true)
        .with_average_bit_rate(2_000_000)
        .with_expected_frame_rate(f64::from(fps))
        .with_max_keyframe_interval(fps)
        .build()?;

    // Pre-encode the first frame so we can use its CMSampleBuffer to seed the
    // AVAssetWriterInput format description.
    fill_surface(&surface, 0)?;
    let first_encoded = encoder.encode(&surface, (0, fps))?;

    let writer = Writer::create(&output, FileType::Mp4)?;
    let input_id = writer.add_video_input_from_sample(
        first_encoded
            .cm_sample_buffer()
            .expect("first frame must have a sample buffer"),
    )?;
    writer.start_session((0, fps))?;
    writer.append_sample(
        input_id,
        first_encoded
            .cm_sample_buffer()
            .expect("first frame must have a sample buffer"),
    )?;
    println!("wrote frame  0: {} bytes", first_encoded.data.len());

    for i in 1..total_frames {
        fill_surface(&surface, i)?;
        let encoded = encoder.encode(&surface, (i64::from(i), fps))?;
        let sb = encoded
            .cm_sample_buffer()
            .expect("encoded frame must have a sample buffer");
        // Backoff on transient AVW_INPUT_NOT_READY — at 30 fps in real-time
        // mode this should essentially never trigger.
        loop {
            match writer.append_sample(input_id, sb) {
                Ok(()) => break,
                Err(AVWriterError::InputNotReady) => {
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                Err(e) => return Err(Box::new(e)),
            }
        }
        if i % 15 == 0 {
            println!("wrote frame {i:>2}: {} bytes", encoded.data.len());
        }
    }

    writer.finish()?;

    // Sanity check: file should exist and be non-trivial in size.
    let metadata = std::fs::metadata(&output)?;
    println!(
        "\n✓ Wrote {} ({} bytes, {total_frames} frames @ {fps} fps)",
        output.display(),
        metadata.len()
    );
    assert!(metadata.len() > 1024, "output file is suspiciously small");

    Ok(())
}

fn fill_surface(surface: &IOSurface, frame_idx: i32) -> Result<(), String> {
    let mut guard = surface
        .lock(IOSurfaceLockOptions::NONE)
        .map_err(|c| format!("lock failed: {c}"))?;
    let bytes = guard
        .as_slice_mut()
        .ok_or_else(|| "non-contiguous surface".to_string())?;
    let g = u8::try_from(frame_idx * 4 % 256).unwrap_or(0);
    for px in bytes.chunks_exact_mut(4) {
        px[0] = 0x40; // B
        px[1] = g; // G
        px[2] = 0x80; // R
        px[3] = 0xFF; // A
    }
    Ok(())
}
