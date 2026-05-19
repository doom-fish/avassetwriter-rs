# Changelog

## [0.9.0] - 2026-05-19

### Added

- Added a real `Asset` wrapper for URL-backed and remote `AVAsset` loading/status/metadata inspection.
- Expanded the caption surface with `CaptionRegion`, `MutableCaptionRegion`, `MutableCaption`, `CaptionRuby`, `CaptionGrouper`, `CaptionFormatConformer`, `CaptionRenderer`, `CaptionConversionValidator`, and related warning/settings/value types.
- Expanded media-processing support with `MutableAudioMix`, `AudioMixInputParameters`, volume-ramp inspection, and request snapshot helpers for `AVAsynchronousVideoCompositionRequest` / `AVAsynchronousCIImageFilteringRequest`.
- Reworked metadata wrappers around `MetadataItem`, timed/date-range groups, and `MetadataItemValueRequest` so export and writer APIs use the richer object-backed surface.

## [0.8.6] - 2026-05-18

- Widen videotoolbox version bound to `<0.17` so 0.16.x resolves.

## [0.8.5] - 2026-05-18

- Widen apple-cf version bound to `<0.10` so 0.9.x resolves.

## [0.8.4] - 2026-05-18

- Widen videotoolbox version bound to `<0.16` so 0.15.x resolves.

## [0.8.3] - 2026-05-18

### Changed

- Re-exported `DropCallback` from `doom-fish-utils::ffi_callbacks` and removed the duplicate local FFI typedef.
- Widened the local `videotoolbox` dev-dependency range to `<0.15` so validation resolves against the current sibling crate version.

## [0.8.2] - 2026-05-18

- Widen videotoolbox version bound so the 0.x bump dep resolves. No source changes.

All notable changes to this project will be documented in this file.

## [0.8.1] - 2026-05-21

### Fixed

- **Panic safety across FFI boundary**: The three `extern "C"` callback
  trampolines in `callbacks.rs` (`ready_callback_trampoline`,
  `pass_description_callback_trampoline`, `segment_callback_trampoline`) now
  wrap user-closure invocations in `std::panic::catch_unwind`. Previously a
  panic in any user-supplied callback was undefined behaviour.
- **Panic safety in async completion callbacks**: The three `extern "C"`
  completion callbacks in `async_api.rs` (`writer_finish_cb`, `export_cb`,
  `compatible_file_types_cb`) are now wrapped with
  `doom_fish_utils::panic_safe::catch_user_panic`.
- **Missing `unsafe impl Send`**: Added explicit `unsafe impl Send` (with
  SAFETY rationale) for `ExportSession`, `OutputSettingsAssistant`,
  `MetadataItemFilter`, `AudioMix`, `VideoComposition`, and `VideoCompositor`.
  All wrap ARC-retained Obj-C pointers whose retain/release operations are
  atomic, making cross-thread moves safe; none implement `Sync` since
  AVFoundation types are not documented as thread-safe for concurrent access.
- **SAFETY comments**: Added `// SAFETY:` annotations to all previously
  un-annotated unsafe blocks in `callbacks.rs` and `async_api.rs`.
- **Broken intra-doc link**: Removed private `FileType::as_str` link in
  `Writer::create` doc comment (`cargo doc` was emitting a warning).
- **Cargo.toml version ranges**: Updated `apple-cf` constraint from
  `>=0.1.1, <0.8` to `>=0.7, <0.9` and `doom-fish-utils` from `"0.1"` to
  `">=0.1, <0.3"` per the `>=X.Y, <X.(Y+2)` policy.

## [0.8.0] - 2026-05-20

### Added

- **`async_api` module** (behind the `async` Cargo feature) — Tier-1 async
  wrappers for three `AVFoundation` completion-handler APIs using
  `doom_fish_utils::completion::AsyncCompletion`:
  - `AsyncWriter::finish(writer: Writer) → WriterFinishFuture` —
    async version of `AVAssetWriter.finishWritingWithCompletionHandler:`.
  - `AsyncExportSession::export(session: &ExportSession) → ExportFuture` —
    async version of `AVAssetExportSession.exportAsynchronouslyWithCompletionHandler:`
    (also covers the macOS 26.0+ `export(to:as:isolation:)` Swift concurrency
    projection of the same API).
  - `AsyncExportSession::compatible_file_types(session: &ExportSession) → CompatibleFileTypesFuture` —
    async version of `AVAssetExportSession.determineCompatibleFileTypesWithCompletionHandler:`.
- New Swift bridge file `swift-bridge/Sources/AVAssetWriterBridge/Async.swift`
  with three `@_cdecl` callback thunks.
- `examples/06_async_api.rs` demonstrating all three async futures with
  `pollster::block_on`.
- `tests/async_api_tests.rs` with four integration tests (happy path +
  error paths) for each future type.
- `doom-fish-utils` optional dependency (enabled by the `async` feature).
- `pollster = "0.3"` dev-dependency for running async examples/tests
  synchronously.

### Fixed

- `apple-cf` version constraint updated from `<0.7` to `<0.8` to unblock
  builds against `apple-cf-rs` 0.7.x.

### Notes

- `AVAssetWriterInput.requestMediaDataWhenReady(on:using:)` is a multi-fire
  handler (fires once per ready window) and is deferred to Tier 2 (Stream
  pattern).
- `AVOutputSettingsAssistant.compatibilityTest(forSourceFormat:completionHandler:)`
  does **not** exist in the AVFoundation SDK; `AVOutputSettingsAssistant` is a
  synchronous class with no completion-handler surface.

## [0.7.1] - 2026-05-16

### Added

- Lightweight `MetadataItemFilter`, `AudioMix`, `VideoComposition`, and `VideoCompositor` wrappers for `AVAssetExportSession` media-processing interop.
- `ExportSession::{metadata_item_filter,set_metadata_item_filter,audio_mix,set_audio_mix,video_composition,set_video_composition,custom_video_compositor}` plus a built-in passthrough `VideoCompositorClass` for custom-compositor round-trips.
- Extended export-session smoke coverage and example flow to exercise metadata filters, audio mixes, video compositions, and custom video compositor inspection.

### Changed

- `COVERAGE.md` / `COVERAGE_AUDIT.md` now report full top-level `AVAssetExportSession.h` coverage on macOS, with only deprecated or intentionally deferred rows remaining.

## [0.7.0] - 2026-05-16

### Added

- Safe `OutputSettingsAssistant` wrapper covering preset discovery, recommended audio/video settings dictionaries, recommended output file type, source format hints, and source frame-duration hints.
- Safe `ExportSession` wrapper covering export preset discovery, compatibility checks, output path/type configuration, progress/status/error readback, compatible file types, time-range/file-length estimates, metadata, temp-directory control, and synchronous export/cancel helpers.
- Tagged-pixel-buffer-group append support backed by Swift-native `CMTaggedBuffer` construction.
- New smoke examples `04_output_settings_smoke` and `05_export_session_smoke` plus new `output_settings_tests` / `export_session_tests` coverage.
- `COVERAGE.md` audit documenting the writer / output-settings / export-session surface against the current macOS SDK.

### Changed

- Expanded `VideoPreset` to cover all current `AVOutputSettingsPreset` variants visible on macOS.
- Split the Swift bridge into logical multi-file areas with every `.swift` file kept under ~500 lines.
- Refreshed README/docs to describe the new writer/export/output-settings surface.

## [0.6.0] - 2026-05-16

### Added

- Broad `AVAssetWriter` / `AVAssetWriterInput` surface expansion covering writer readback, writer configuration, generic input creation, metadata-track creation, caption/text inputs, input-state readback, track associations, multipass callbacks, and segmented-output configuration.
- Safe Rust models for `CMTime` / `CMTimeRange`, metadata payloads, caption payloads, segmented-output reports, and callback trampolines.
- Public readback types including `WriterStatus`, `MediaType`, `FileTypeProfile`, `InputGroupInfo`, `InputPassDescription`, `SegmentReport`, `SegmentTrackReport`, and `SegmentReportSampleInfo`.
- New smoke example `03_smoke_surface` covering multi-track, metadata, caption, and segmented-writer setup flows without requiring external media assets.
- Much stricter API-coverage tests that scan the split Swift bridge and require 100% coverable `AVAssetWriter` / `AVAssetWriterInput` header coverage.

### Changed

- Expanded `FileType` coverage to include the broader set of `AVFileType` constants exposed by current Apple SDKs.
- Quick-start docs and examples now write into `target/example-artifacts` instead of `/tmp`.
- Development baseline now uses the local `videotoolbox` `0.10.x` path dependency range.

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
