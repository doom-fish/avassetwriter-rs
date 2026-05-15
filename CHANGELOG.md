# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-05-15

### Changed (BREAKING)

- `Writer::add_video_input_from_sample(*mut c_void)` →
  `add_video_input_from_sample(&apple_cf::cm::CMSampleBuffer)`. The opaque
  pointer overload is gone; pass the safe wrapper directly.
- `Writer::append_sample(InputId, *mut c_void)` →
  `append_sample(InputId, &apple_cf::cm::CMSampleBuffer)`.

### Added

- `apple-cf` as a regular dependency (with `cm` feature).

## [Unreleased]

### Added

- **Audio support** — `Writer::add_audio_input_pcm(sample_rate, channels, bits_per_sample)`
  configures an audio track and `Writer::append_audio_pcm` muxes interleaved
  little-endian signed-integer PCM bytes. AVAssetWriter transcodes to AAC at
  128 kbps internally for `.mp4` / `.m4v` containers.
- Smoke test `02_write_av_mp4` verifies the full pipeline:
  IOSurface → H.264 video + 48 kHz stereo PCM sine → AAC-muxed `.mp4`.
  ffprobe confirms two streams: H.264 High @ 640×480 / 2.0s and AAC LC stereo / 2.0s.

### Changed

- README quick-start example now demonstrates video + audio together.

## Initial release

### Added

- Initial scaffold targeting `AVAssetWriter` with a single video track.
- `Writer::create` / `add_video_input_from_sample` / `start_session` /
  `append_sample` / `finish`.
- `FileType` enum (`Mp4`, `Mov`, `M4v`).
- `AVWriterError` covers create / start / append / finish / input-not-ready /
  invalid-argument / invalid-state failure modes.
- Swift bridge wraps `AVAssetWriter` + `AVAssetWriterInput`, hiding the
  Obj-C / KVO / async-completion-handler surface behind plain `@_cdecl`
  exports.
- Smoke test `01_write_mp4` muxes 60 H.264 frames produced by `videotoolbox`
  into a verified `.mp4` file end-to-end.
