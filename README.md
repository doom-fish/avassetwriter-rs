# avassetwriter

Safe Rust bindings for Apple's [AVAssetWriter](https://developer.apple.com/documentation/avfoundation/avassetwriter), [AVOutputSettingsAssistant](https://developer.apple.com/documentation/avfoundation/avoutputsettingsassistant), and [AVAssetExportSession](https://developer.apple.com/documentation/avfoundation/avassetexportsession) — covering writer configuration/readback, audio/video/metadata inputs, pixel-buffer/metadata/caption/tagged-buffer adaptors, output-settings recommendations, export preset discovery, compatibility checks, and file export on macOS.

> **Status:** `0.7.0` substantially covers the public writer / output-settings / export-session surface used when building real muxing and transcode pipelines.

Designed to compose with [`videotoolbox`](https://github.com/doom-fish/videotoolbox-rs): hand the `CMSampleBuffer` straight from the encoder to the muxer for video, push interleaved PCM bytes for audio, or build pixel-buffer / metadata-driven pipelines directly.

## Quick start — video + audio

```rust,no_run
use avassetwriter::prelude::*;
use videotoolbox::prelude::*;
use apple_cf::iosurface::{IOSurface, IOSurfaceLockOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let artifacts = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/example-artifacts");
    std::fs::create_dir_all(&artifacts)?;
    let output = artifacts.join("out.mp4");

    let surface = IOSurface::create(640, 480, u32::from_be_bytes(*b"BGRA"), 4)
        .ok_or("alloc failed")?;
    let encoder = CompressionSession::builder(640, 480, Codec::H264)
        .with_real_time(true)
        .with_expected_frame_rate(30.0)
        .build()?;

    let writer = Writer::create(&output, FileType::Mp4)?;

    // Video: seed format from first encoded frame, then push every frame.
    let first = encoder.encode(&surface, (0, 30))?;
    let video = writer.add_video_input_from_sample(first.cm_sample_buffer().unwrap())?;

    // Audio: 48 kHz stereo i16 PCM → AAC 128 kbps.
    let audio = writer.add_audio_input_pcm(48_000.0, 2, 16)?;

    writer.start_session((0, 30))?;
    writer.append_sample(video, first.cm_sample_buffer().unwrap())?;
    for i in 1..30 {
        let frame = encoder.encode(&surface, (i64::from(i), 30))?;
        writer.append_sample(video, frame.cm_sample_buffer().unwrap())?;
    }

    let pcm: Vec<i16> = vec![0; 48_000 * 2]; // 1 second of silence, stereo
    let pcm_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(pcm.as_ptr().cast::<u8>(), std::mem::size_of_val(&pcm[..]))
    };
    writer.append_audio_pcm(audio, pcm_bytes, 48_000, (0, 48_000))?;

    writer.finish()?;
    Ok(())
}
```

## Pipeline composition

```text
screencapturekit-rs ──► IOSurface
                              │
                              ▼
                       videotoolbox-rs ──► EncodedFrame ──► .mp4 file
                                           (CMSampleBuffer) ▲
                                                            │
                                                       avassetwriter-rs (this crate)
```

All three crates pass `CMSampleBuffer` as opaque `*mut c_void` so no shared `cm` type wrapper is required (yet).

See [`COVERAGE.md`](COVERAGE.md) for the current Apple-SDK audit, including implemented / partial / skipped rows.

## Surface highlights

- `Writer::create` + `Writer::create_segmented`
- Writer readback/configuration: status, error, output path/type, metadata, temp directory, fragment settings, duration hints, time scale, combinable fragments
- Inputs: sample-buffer video/audio, PCM audio, generic inputs, metadata inputs, caption/text inputs, pixel-buffer inputs, tagged-pixel-buffer-group inputs, multi-input groups
- Adaptors: pixel-buffer, tagged-pixel-buffer-group, metadata, caption
- `OutputSettingsAssistant`: all current output-settings presets, recommended audio/video dictionaries, recommended file type, source format hints, and source frame-duration hints
- `ExportSession`: preset discovery, compatibility checks, output file/type configuration, progress/status/error readback, compatible file types, time-range/file-length estimates, metadata, multipass/temp-dir controls, and synchronous export/cancel wrappers
- Input readback/configuration: media type, metadata, language tags, transforms, volume, source hints, media-data location, multipass state, track associations
- Segmented-output callbacks and `AVFileTypeProfile`
- Smoke examples:
  - `cargo run --example 01_write_mp4`
  - `cargo run --example 02_write_av_mp4`
  - `cargo run --example 03_smoke_surface`
  - `cargo run --example 04_output_settings_smoke`
  - `cargo run --example 05_export_session_smoke`

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your option.
