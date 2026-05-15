# avassetwriter

Safe Rust bindings for Apple's [AVAssetWriter](https://developer.apple.com/documentation/avfoundation/avassetwriter) — mux compressed video into `.mp4` / `.mov` / `.m4v` files on macOS.

> **Status:** experimental. Video-only at v0.1; audio inputs and timed-metadata tracks land in v0.2.

Designed to compose with [`videotoolbox`](https://github.com/doom-fish/videotoolbox-rs): hand the `CMSampleBuffer` straight from the encoder to the muxer, no byte-copying or format-description reconstruction needed.

## Quick start

```rust,no_run
use avassetwriter::prelude::*;
use videotoolbox::prelude::*;
use apple_cf::iosurface::{IOSurface, IOSurfaceLockOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let surface = IOSurface::create(640, 480, u32::from_be_bytes(*b"BGRA"), 4)
        .ok_or("alloc failed")?;
    let encoder = CompressionSession::builder(640, 480, Codec::H264)
        .with_real_time(true)
        .with_expected_frame_rate(30.0)
        .build()?;

    let writer = Writer::create("/tmp/out.mp4", FileType::Mp4)?;

    // Seed the writer input with the first encoded frame's format.
    let first = encoder.encode(&surface, (0, 30))?;
    let input = writer.add_video_input_from_sample(first.cm_sample_buffer_ptr())?;
    writer.start_session((0, 30))?;
    writer.append_sample(input, first.cm_sample_buffer_ptr())?;

    for i in 1..30 {
        let frame = encoder.encode(&surface, (i64::from(i), 30))?;
        writer.append_sample(input, frame.cm_sample_buffer_ptr())?;
    }
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

## Roadmap

- [x] Single video track from H.264/HEVC `CMSampleBuffer`
- [ ] Audio track (PCM and compressed)
- [ ] Timed-metadata track
- [ ] Per-track output settings (codec selection at writer level instead of inheriting from sample)
- [ ] Direct ingest from raw bitstream bytes (for callers that don't have a `CMSampleBuffer` handy)
- [ ] Async `finish()` that doesn't block the calling thread

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your option.
