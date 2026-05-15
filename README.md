# avassetwriter

Safe Rust bindings for Apple's [AVAssetWriter](https://developer.apple.com/documentation/avfoundation/avassetwriter) — mux compressed video **and PCM audio** into `.mp4` / `.mov` / `.m4v` files on macOS.

> **Status:** experimental. Video + AAC-transcoded audio in v0.1; timed-metadata tracks land in v0.2.

Designed to compose with [`videotoolbox`](https://github.com/doom-fish/videotoolbox-rs): hand the `CMSampleBuffer` straight from the encoder to the muxer for video, and push interleaved PCM bytes for audio (the writer transcodes to AAC internally).

## Quick start — video + audio

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

## Roadmap

- [x] Single video track from H.264/HEVC `CMSampleBuffer`
- [x] Audio track via `add_audio_input_pcm` + `append_audio_pcm` (PCM → AAC)
- [ ] Audio track from external `CMSampleBuffer` (zero-copy from `AVCaptureSession`)
- [ ] Timed-metadata track
- [ ] Per-track output settings (codec selection at writer level instead of inheriting from sample)
- [ ] Direct ingest from raw bitstream bytes (for callers that don't have a `CMSampleBuffer` handy)
- [ ] Async `finish()` that doesn't block the calling thread

## License

Licensed under either of [Apache-2.0](LICENSE-APACHE) or [MIT](LICENSE-MIT) at your option.
