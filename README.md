# avassetwriter

Safe Rust bindings for Apple's [AVAssetWriter](https://developer.apple.com/documentation/avfoundation/avassetwriter), [AVOutputSettingsAssistant](https://developer.apple.com/documentation/avfoundation/avoutputsettingsassistant), and [AVAssetExportSession](https://developer.apple.com/documentation/avfoundation/avassetexportsession) — covering writer configuration/readback, audio/video/metadata inputs, pixel-buffer/metadata/caption/tagged-buffer adaptors, output-settings recommendations, export preset discovery, compatibility checks, export-session media-processing objects, and file export on macOS.

> **Status:** The current release substantially covers the public writer, output-settings, export-session, metadata, and custom video-composition surface used by real muxing and transcode pipelines. It does not wrap `AVAssetReader` or its outputs: the crate writes and exports media, but cannot read samples back out of an asset. Requires macOS 13 or later.

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
    let first = encoder.encode(&surface, CMTime::new(0, 30))?;
    let video = writer.add_video_input_from_sample(first.cm_sample_buffer().unwrap())?;

    // Audio: 48 kHz stereo i16 PCM → AAC 128 kbps.
    let audio = writer.add_audio_input_pcm(48_000.0, 2, 16)?;

    writer.start_session(CMTime::new(0, 30))?;
    writer.append_sample(video, first.cm_sample_buffer().unwrap())?;
    for i in 1..30 {
        let frame = encoder.encode(&surface, CMTime::new(i64::from(i), 30))?;
        writer.append_sample(video, frame.cm_sample_buffer().unwrap())?;
    }

    let pcm: Vec<i16> = vec![0; 48_000 * 2]; // 1 second of silence, stereo
    let pcm_bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(pcm.as_ptr().cast::<u8>(), std::mem::size_of_val(&pcm[..]))
    };
    writer.append_audio_pcm(audio, pcm_bytes, 48_000, CMTime::new(0, 48_000))?;

    writer.finish()?;
    Ok(())
}
```

## Output paths

`Writer::create` never touches an existing path: it returns
`AVWriterError::OutputExists` instead. To replace an earlier output, use
`Writer::create_with_policy(path, file_type, OutputPolicy::ReplaceFile)`.
`ReplaceFile` removes only a regular file; directories and symlinks are always
refused. `ExportSession::set_output_policy` applies the same rules to exports,
and an export never replaces its own source asset.

## Errors instead of exceptions

`AVFoundation` raises Objective-C exceptions for many kinds of misuse, and an
uncaught exception aborts the process. The bridge checks every documented
precondition first. Where only `AVFoundation` can judge a value (output settings,
language codes, caption groups and regions, adaptor attachment), the call runs
inside an Objective-C `@try` and the exception reason comes back as an error.
Misuse such as calling `start_session` twice, configuring the writer after
writing starts, or appending outside the session returns
`AVWriterError::InvalidState`. Invalid values, such as a sample whose media
type doesn't match its input or a non-numeric presentation time, return
`AVWriterError::InvalidArgument`.

All times and ranges are `apple_cf::cm::CMTime` / `CMTimeRange`, re-exported
from the prelude.

## Async API

Enable with the `async` Cargo feature:

```toml
[dependencies]
avassetwriter = { version = "0.12", features = ["async"] }
```

The `async_api` module wraps three completion-handler APIs as
executor-agnostic [`Future`](std::future::Future) types and exposes
`AVAssetWriterInput.requestMediaDataWhenReady(on:using:)` as a bounded async
stream:

| Rust type | Apple API |
|-----------|-----------|
| `AsyncWriter::finish(writer)` | `AVAssetWriter.finishWritingWithCompletionHandler:` |
| `AsyncWriterInput::request_media_data_when_ready(&writer, input, capacity)` | `AVAssetWriterInput.requestMediaDataWhenReady(on:using:)` |
| `AsyncExportSession::export(session)` | `AVAssetExportSession.exportAsynchronouslyWithCompletionHandler:` |
| `AsyncExportSession::compatible_file_types(session)` | `AVAssetExportSession.determineCompatibleFileTypesWithCompletionHandler:` |

```rust,no_run
use apple_cf::cm::CMTime;
use avassetwriter::{FileType, Writer};
use avassetwriter::async_api::{AsyncWriter, AsyncWriterInput};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
pollster::block_on(async {
    let writer = Writer::create("out.m4a", FileType::M4a)?;
    let input = writer.add_audio_input_pcm(48_000.0, 1, 16)?;
    writer.start_session(CMTime::new(0, 48_000))?;
    let ready = AsyncWriterInput::request_media_data_when_ready(&writer, input, 4)?;
    let _ = ready.next().await;
    let silence = vec![0_u8; 48_000 * 2];
    writer.append_audio_pcm(input, &silence, 48_000, CMTime::new(0, 48_000))?;
    drop(ready);
    AsyncWriter::finish(writer).await?;
    Ok::<_, Box<dyn std::error::Error>>(())
})
# }
```

See `examples/06_async_api.rs` for a complete walkthrough.

## Threading and callback teardown

`Writer` is `Send + Sync`. Every writer, input, adaptor, format, ID-allocation,
readiness, multipass, segment, and metadata operation is serialized by the
Swift bridge as one complete operation. The gate is recursive, so a writer
callback can call nonblocking writer methods on the same thread. Blocking
`Writer::finish`, `Writer::end_session`, and `Writer::flush_segment` return
`AVWriterError::InvalidState` when called from inside a writer callback.

Readiness, multipass, segment, and async-stream registrations retain their Rust
userdata while an invocation is active. Native registration is disabled before
the final Rust drop callback runs; teardown from inside the callback defers that
drop until the callback returns instead of waiting on itself. Explicit
`Writer::remove_input_callbacks` and `Writer::remove_segment_callback` methods
are available when a registration should end before writer teardown.
Readiness and multipass registration require `Writer::start_session` to have
succeeded.

## Lazy metadata identity

Cloning a native-backed `MetadataItem` retains the same native item, including
its lazy value loader and loading state. Metadata filters, groups, writer/export
setters, getters, and value requests transport retained native items rather than
rebuilding them from JSON.

The `with_data_type`, `with_extended_language_tag`, and
`with_locale_identifier` builders use native copy-on-write. If the item is lazy,
the native mutable-copy operation may synchronously invoke its loader; the
resulting item keeps the authoritative loaded value and all earlier property
changes.

## Rust video composition

`VideoComposition::install_rust_video_compositor` installs a real retained
request handler. Each `VideoCompositionRequest` can copy source pixel/sample
buffers directly, allocate an output pixel buffer from the render context, and
finish with a frame, error, or cancellation exactly once. Requests are `Send`,
so the handler may complete them synchronously or move them to a worker thread.
Dropping an unfinished request cancels it.

```rust,no_run
use avassetwriter::{
    AVWriterError, ExportPreset, ExportSession, FileType, VideoComposition,
    VideoCompositionRequest,
};

fn finish_with_first_source_frame(
    request: VideoCompositionRequest,
) -> Result<(), AVWriterError> {
    let track_id = request
        .source_track_ids()?
        .into_iter()
        .next()
        .ok_or_else(|| AVWriterError::InvalidState("request had no source track".into()))?;
    let frame = request
        .source_frame(track_id)?
        .ok_or_else(|| AVWriterError::InvalidState("request had no source frame".into()))?;
    request.finish_with_frame(&frame)
}

# fn main() -> Result<(), Box<dyn std::error::Error>> {
let source = "input.mov";
let output = "output.mp4";
let composition = VideoComposition::from_asset(source)?;
composition.install_rust_video_compositor(|request| {
    let _ = std::thread::spawn(move || {
        if let Err(error) = finish_with_first_source_frame(request) {
            eprintln!("video composition request failed: {error}");
        }
    });
})?;

let session = ExportSession::new(source, ExportPreset::HighestQuality)?;
session.set_output_path(Some(std::path::Path::new(output)))?;
session.set_output_file_type(Some(FileType::Mp4))?;
session.set_video_composition(Some(&composition))?;
session.export()?;
# Ok(())
# }
```

The bundled compositor currently supports 32-bit BGRA SDR source/output pixel
buffers and source sample-buffer access. Tagged buffers, wide-color/HDR source
frames, and direct Rust `CIImage` processing are rejected explicitly. The
existing process-global request snapshots remain available for diagnostics and
use a versioned, single-consumer copy/decode/ack transaction. Failed transfer
or decoding leaves the generation available for a later retry.

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

The encoder and writer share `apple_cf::cm::CMSampleBuffer` and
`apple_cf::cv::CVPixelBuffer` wrappers, preserving retained native ownership
without copying frame or sample data.

See [`COVERAGE.md`](COVERAGE.md) for the current Apple-SDK audit, including implemented / partial / skipped rows.

## Surface highlights

- `Writer::create` / `Writer::create_with_policy` + `Writer::create_segmented`
- Writer readback/configuration: status, error, output path/type, metadata, temp directory, fragment settings, duration hints, time scale, combinable fragments
- Inputs: sample-buffer video/audio, PCM audio, generic inputs, metadata inputs, caption/text inputs, pixel-buffer inputs, tagged-pixel-buffer-group inputs, multi-input groups
- Adaptors: pixel-buffer, tagged-pixel-buffer-group, metadata, caption
- `OutputSettingsAssistant`: all current output-settings presets, recommended audio/video dictionaries, recommended file type, source format hints, and source frame-duration hints
- `ExportSession`: preset discovery, compatibility checks, output file/type configuration, progress/status/error readback, compatible file types, time-range/file-length estimates, metadata/metadata-filter control, audio-mix/video-composition interop, retained Rust custom-compositor requests, multipass/temp-dir controls, and synchronous export/cancel wrappers
- Native metadata identity and lazy-loader preservation across clone, filtering, grouping, writer/export round trips, and property copy-on-write
- Thread-safe `Writer` operations with reentrant callbacks and in-flight-safe callback teardown
- Input readback/configuration: media type, metadata, language tags, transforms, volume, source hints, media-data location, multipass state, track associations
- Segmented-output callbacks and `AVFileTypeProfile`
- Smoke examples:
  - `cargo run --example 01_write_mp4`
  - `cargo run --example 02_write_av_mp4`
  - `cargo run --example 03_smoke_surface`
  - `cargo run --example 04_output_settings_smoke`
  - `cargo run --example 05_export_session_smoke` (includes asynchronous Rust frame processing)

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your option.
