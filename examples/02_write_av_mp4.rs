//! Smoke test: write a 60-frame H.264 video + a matching 2-second 48 kHz
//! stereo PCM sine-wave track into /tmp/avassetwriter_av_smoke.mp4.
//!
//! Verifies:
//!   IOSurface --> VideoToolbox --> AVAssetWriter (video)
//!   PCM bytes -->  AVAssetWriter (audio, transcoded to AAC internally)
//! ...both interleaved into a single .mp4 with two tracks.
//!
//! Run with: `cargo run --example 02_write_av_mp4`

use apple_cf::iosurface::{IOSurface, IOSurfaceLockOptions};
use avassetwriter::prelude::*;
use videotoolbox::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let width: i32 = 640;
    let height: i32 = 480;
    let pixel_format = u32::from_be_bytes(*b"BGRA");
    let video_fps: i32 = 30;
    let total_frames: i32 = 60;
    let sample_rate: f64 = 48_000.0;
    let channels: u32 = 2;
    let bits_per_sample: u32 = 16;
    let output = "/tmp/avassetwriter_av_smoke.mp4";

    let surface = IOSurface::create(
        usize::try_from(width)?,
        usize::try_from(height)?,
        pixel_format,
        4,
    )
    .ok_or("IOSurface alloc")?;

    let encoder = CompressionSession::builder(width, height, Codec::H264)
        .with_real_time(true)
        .with_average_bit_rate(2_000_000)
        .with_expected_frame_rate(f64::from(video_fps))
        .with_max_keyframe_interval(video_fps)
        .build()?;

    // Pre-encode the first frame so the writer's video input can use its
    // CMSampleBuffer to seed the format description.
    fill_surface(&surface, 0)?;
    let first_frame = encoder.encode(&surface, (0, video_fps))?;

    let writer = Writer::create(output, FileType::Mp4)?;
    let video_input = writer.add_video_input_from_sample(first_frame.cm_sample_buffer_ptr())?;
    let audio_input = writer.add_audio_input_pcm(sample_rate, channels, bits_per_sample)?;
    writer.start_session((0, video_fps))?;

    // 1. Write the video track
    writer.append_sample(video_input, first_frame.cm_sample_buffer_ptr())?;
    for i in 1..total_frames {
        fill_surface(&surface, i)?;
        let frame = encoder.encode(&surface, (i64::from(i), video_fps))?;
        writer.append_sample(video_input, frame.cm_sample_buffer_ptr())?;
    }
    println!("wrote {total_frames} video frames");

    // 2. Generate a 2-second 440 Hz sine wave (interleaved stereo i16) and
    //    push it as one big chunk. AVAssetWriter handles the AAC encoding.
    let total_audio_frames = (sample_rate * 2.0) as usize;
    let mut pcm = Vec::<i16>::with_capacity(total_audio_frames * channels as usize);
    let frequency = 440.0_f64;
    for n in 0..total_audio_frames {
        let t = n as f64 / sample_rate;
        let amplitude = (2.0 * std::f64::consts::PI * frequency * t).sin() * 0.5;
        let sample_i16 = (amplitude * f64::from(i16::MAX)) as i16;
        for _ in 0..channels {
            pcm.push(sample_i16);
        }
    }
    let pcm_bytes: &[u8] = bytemuck_cast(&pcm);
    writer.append_audio_pcm(
        audio_input,
        pcm_bytes,
        total_audio_frames,
        (0, sample_rate as i32),
    )?;
    println!(
        "wrote {total_audio_frames} audio frames ({:.1}s @ {sample_rate} Hz stereo)",
        total_audio_frames as f64 / sample_rate
    );

    writer.finish()?;

    let metadata = std::fs::metadata(output)?;
    println!("\nOK Wrote {output} ({} bytes)", metadata.len());
    assert!(metadata.len() > 4096, "output file is suspiciously small");
    Ok(())
}

fn fill_surface(surface: &IOSurface, frame_idx: i32) -> Result<(), String> {
    let mut g = surface
        .lock(IOSurfaceLockOptions::NONE)
        .map_err(|c| format!("lock failed: {c}"))?;
    if let Some(bytes) = g.as_slice_mut() {
        let v = u8::try_from(frame_idx * 4 % 256).unwrap_or(0);
        for px in bytes.chunks_exact_mut(4) {
            px[0] = 0x40;
            px[1] = v;
            px[2] = 0x80;
            px[3] = 0xFF;
        }
    }
    Ok(())
}

/// Tiny no-dep equivalent of `bytemuck::cast_slice::<i16, u8>`.
fn bytemuck_cast(slice: &[i16]) -> &[u8] {
    let len = std::mem::size_of_val(slice);
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast::<u8>(), len) }
}
