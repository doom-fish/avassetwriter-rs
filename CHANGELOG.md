# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

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
