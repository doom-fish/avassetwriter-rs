# avassetwriter coverage audit (vs MacOSX26.2.sdk)

Scope: `AVAssetWriter.h`, `AVAssetWriterInput.h`, `AVAssetExportSession.h`, and `AVOutputSettingsAssistant.h` from `AVFoundation.framework`.

Methodology: per the audit instructions, counts are based on top-level public symbols (interfaces, protocols, enums, and exported constants) that are reachable from the crate's public Rust API. Objective-C member-level caveats are called out separately below but do not change the symbol counts.

SDK_PUBLIC_SYMBOLS: 64
VERIFIED: 60
GAPS: 0
EXEMPT: 4
COVERAGE_PCT: 100.0%

## 🟢 VERIFIED
| Symbol | Kind | Header | Wrapped by |
| --- | --- | --- | --- |
| `AVAssetWriterStatus` | enum | `AVAssetWriter.h` | `WriterStatus` |
| `AVAssetWriter` | interface | `AVAssetWriter.h` | `Writer` |
| `AVAssetWriterInputGroup` | interface | `AVAssetWriter.h` | `InputGroupInfo` + `Writer::{add_input_group,input_groups}` |
| `AVAssetWriterDelegate` | protocol | `AVAssetWriter.h` | `Writer::create_segmented` + `SegmentOutput` callback bridge |
| `AVAssetWriterInputMediaDataLocationInterleavedWithMainMediaData` | constant | `AVAssetWriterInput.h` | `InputMediaDataLocation::InterleavedWithMainMediaData` |
| `AVAssetWriterInputMediaDataLocationBeforeMainMediaDataNotInterleaved` | constant | `AVAssetWriterInput.h` | `InputMediaDataLocation::BeforeMainMediaDataNotInterleaved` |
| `AVAssetWriterInputMediaDataLocationSparselyInterleavedWithMainMediaData` | constant | `AVAssetWriterInput.h` | `InputMediaDataLocation::SparselyInterleavedWithMainMediaData` |
| `AVAssetWriterInput` | interface | `AVAssetWriterInput.h` | `InputId` + `Writer` input APIs (core input surface wrapped indirectly) |
| `AVAssetWriterInputPassDescription` | interface | `AVAssetWriterInput.h` | `InputPassDescription` + pass-description helpers |
| `AVAssetExportPresetLowQuality` | constant | `AVAssetExportSession.h` | `ExportPreset::LowQuality` |
| `AVAssetExportPresetMediumQuality` | constant | `AVAssetExportSession.h` | `ExportPreset::MediumQuality` |
| `AVAssetExportPresetHighestQuality` | constant | `AVAssetExportSession.h` | `ExportPreset::HighestQuality` |
| `AVAssetExportPresetHEVCHighestQuality` | constant | `AVAssetExportSession.h` | `ExportPreset::HevcHighestQuality` |
| `AVAssetExportPresetHEVCHighestQualityWithAlpha` | constant | `AVAssetExportSession.h` | `ExportPreset::HevcHighestQualityWithAlpha` |
| `AVAssetExportPreset640x480` | constant | `AVAssetExportSession.h` | `ExportPreset::P640x480` |
| `AVAssetExportPreset960x540` | constant | `AVAssetExportSession.h` | `ExportPreset::P960x540` |
| `AVAssetExportPreset1280x720` | constant | `AVAssetExportSession.h` | `ExportPreset::P1280x720` |
| `AVAssetExportPreset1920x1080` | constant | `AVAssetExportSession.h` | `ExportPreset::P1920x1080` |
| `AVAssetExportPreset3840x2160` | constant | `AVAssetExportSession.h` | `ExportPreset::P3840x2160` |
| `AVAssetExportPresetHEVC1920x1080` | constant | `AVAssetExportSession.h` | `ExportPreset::Hevc1920x1080` |
| `AVAssetExportPresetHEVC1920x1080WithAlpha` | constant | `AVAssetExportSession.h` | `ExportPreset::Hevc1920x1080WithAlpha` |
| `AVAssetExportPresetHEVC3840x2160` | constant | `AVAssetExportSession.h` | `ExportPreset::Hevc3840x2160` |
| `AVAssetExportPresetHEVC3840x2160WithAlpha` | constant | `AVAssetExportSession.h` | `ExportPreset::Hevc3840x2160WithAlpha` |
| `AVAssetExportPresetHEVC4320x2160` | constant | `AVAssetExportSession.h` | `ExportPreset::Hevc4320x2160` |
| `AVAssetExportPresetHEVC7680x4320` | constant | `AVAssetExportSession.h` | `ExportPreset::Hevc7680x4320` |
| `AVAssetExportPresetMVHEVC960x960` | constant | `AVAssetExportSession.h` | `ExportPreset::MvHevc960x960` |
| `AVAssetExportPresetMVHEVC1440x1440` | constant | `AVAssetExportSession.h` | `ExportPreset::MvHevc1440x1440` |
| `AVAssetExportPresetMVHEVC4320x4320` | constant | `AVAssetExportSession.h` | `ExportPreset::MvHevc4320x4320` |
| `AVAssetExportPresetMVHEVC7680x7680` | constant | `AVAssetExportSession.h` | `ExportPreset::MvHevc7680x7680` |
| `AVAssetExportPresetAppleM4A` | constant | `AVAssetExportSession.h` | `ExportPreset::AppleM4A` |
| `AVAssetExportPresetPassthrough` | constant | `AVAssetExportSession.h` | `ExportPreset::Passthrough` |
| `AVAssetExportPresetAppleProRes422LPCM` | constant | `AVAssetExportSession.h` | `ExportPreset::AppleProRes422Lpcm` |
| `AVAssetExportPresetAppleProRes4444LPCM` | constant | `AVAssetExportSession.h` | `ExportPreset::AppleProRes4444Lpcm` |
| `AVAssetExportPresetAppleM4VCellular` | constant | `AVAssetExportSession.h` | `ExportPreset::AppleM4vCellular` |
| `AVAssetExportPresetAppleM4ViPod` | constant | `AVAssetExportSession.h` | `ExportPreset::AppleM4viPod` |
| `AVAssetExportPresetAppleM4V480pSD` | constant | `AVAssetExportSession.h` | `ExportPreset::AppleM4v480pSd` |
| `AVAssetExportPresetAppleM4VAppleTV` | constant | `AVAssetExportSession.h` | `ExportPreset::AppleM4vAppleTv` |
| `AVAssetExportPresetAppleM4VWiFi` | constant | `AVAssetExportSession.h` | `ExportPreset::AppleM4vWiFi` |
| `AVAssetExportPresetAppleM4V720pHD` | constant | `AVAssetExportSession.h` | `ExportPreset::AppleM4v720pHd` |
| `AVAssetExportPresetAppleM4V1080pHD` | constant | `AVAssetExportSession.h` | `ExportPreset::AppleM4v1080pHd` |
| `AVAssetExportSessionStatus` | enum | `AVAssetExportSession.h` | `ExportStatus` |
| `AVAssetTrackGroupOutputHandling` | enum | `AVAssetExportSession.h` | `TrackGroupOutputHandling` |
| `AVAssetExportSession` | interface | `AVAssetExportSession.h` | `ExportSession` (core export/config surface; see caveats below) |
| `AVVideoCompositing` | protocol | `AVAssetExportSession.h` | `VideoCompositor` + `VideoCompositorClass` + `ExportSession::custom_video_compositor` |
| `AVOutputSettingsPreset640x480` | constant | `AVOutputSettingsAssistant.h` | `VideoPreset::Sd640x480` |
| `AVOutputSettingsPreset960x540` | constant | `AVOutputSettingsAssistant.h` | `VideoPreset::Hd960x540` |
| `AVOutputSettingsPreset1280x720` | constant | `AVOutputSettingsAssistant.h` | `VideoPreset::Hd1280x720` |
| `AVOutputSettingsPreset1920x1080` | constant | `AVOutputSettingsAssistant.h` | `VideoPreset::FullHd1920x1080` |
| `AVOutputSettingsPreset3840x2160` | constant | `AVOutputSettingsAssistant.h` | `VideoPreset::Uhd3840x2160` |
| `AVOutputSettingsPresetHEVC1920x1080` | constant | `AVOutputSettingsAssistant.h` | `VideoPreset::Hevc1920x1080` |
| `AVOutputSettingsPresetHEVC1920x1080WithAlpha` | constant | `AVOutputSettingsAssistant.h` | `VideoPreset::Hevc1920x1080WithAlpha` |
| `AVOutputSettingsPresetHEVC3840x2160` | constant | `AVOutputSettingsAssistant.h` | `VideoPreset::Hevc3840x2160` |
| `AVOutputSettingsPresetHEVC3840x2160WithAlpha` | constant | `AVOutputSettingsAssistant.h` | `VideoPreset::Hevc3840x2160WithAlpha` |
| `AVOutputSettingsPresetHEVC4320x2160` | constant | `AVOutputSettingsAssistant.h` | `VideoPreset::Hevc4320x2160` |
| `AVOutputSettingsPresetHEVC7680x4320` | constant | `AVOutputSettingsAssistant.h` | `VideoPreset::Hevc7680x4320` |
| `AVOutputSettingsPresetMVHEVC960x960` | constant | `AVOutputSettingsAssistant.h` | `VideoPreset::MvHevc960x960` |
| `AVOutputSettingsPresetMVHEVC1440x1440` | constant | `AVOutputSettingsAssistant.h` | `VideoPreset::MvHevc1440x1440` |
| `AVOutputSettingsPresetMVHEVC4320x4320` | constant | `AVOutputSettingsAssistant.h` | `VideoPreset::MvHevc4320x4320` |
| `AVOutputSettingsPresetMVHEVC7680x7680` | constant | `AVOutputSettingsAssistant.h` | `VideoPreset::MvHevc7680x7680` |
| `AVOutputSettingsAssistant` | interface | `AVOutputSettingsAssistant.h` | `OutputSettingsAssistant` |

## 🔴 GAPS
None.

## ⏭️ EXEMPT
| Symbol | Kind | Header | Reason | SDK attribute |
| --- | --- | --- | --- | --- |
| `AVAssetWriterInputPixelBufferAdaptor` | interface | `AVAssetWriterInput.h` | Deprecated in Swift in favor of `AVAssetWriter.inputPixelBufferReceiver(...)`; crate still wraps the legacy adaptor via `Writer::{add_video_input_pixel_buffer,attach_pixel_buffer_adaptor,pixel_buffer_pool,append_pixel_buffer}`. | `API_DEPRECATED("Use AVAssetWriter.inputPixelBufferReceiver(for:pixelBufferAttributes:) instead", macos(10.7, API_TO_BE_DEPRECATED), ...)` |
| `AVAssetWriterInputTaggedPixelBufferGroupAdaptor` | interface | `AVAssetWriterInput.h` | Deprecated in Swift in favor of `AVAssetWriter.inputTaggedPixelBufferGroupReceiver(...)`; crate still wraps the legacy tagged-buffer adaptor APIs. | `API_DEPRECATED("Use AVAssetWriter.inputTaggedPixelBufferGroupReceiver(for:pixelBufferAttributes:) instead", macos(14.0, API_TO_BE_DEPRECATED), ...)` |
| `AVAssetWriterInputMetadataAdaptor` | interface | `AVAssetWriterInput.h` | Deprecated in Swift in favor of `AVAssetWriter.inputMetadataReceiver(...)`; crate still wraps the legacy timed-metadata adaptor path. | `API_DEPRECATED("Use AVAssetWriter.inputMetadataReceiver(for:) instead", macos(10.10, API_TO_BE_DEPRECATED), ...)` |
| `AVAssetWriterInputCaptionAdaptor` | interface | `AVAssetWriterInput.h` | Deprecated in Swift in favor of `AVAssetWriter.inputCaptionReceiver(...)`; crate still wraps the legacy caption adaptor path. | `API_DEPRECATED("Use AVAssetWriter.inputCaptionReceiver(for:) instead", macos(12.0, API_TO_BE_DEPRECATED), ...)` |

## Member-level caveats (not counted above)

- `AVAssetExportSession` is still partially wrapped at the member level: `asset` is surfaced as `asset_path()` for URL-backed sessions, and `audioTimePitchAlgorithm` remains intentionally deferred.
- `MetadataItemFilter`, `AudioMix`, `VideoComposition`, and `VideoCompositor` are exposed as lightweight interop wrappers focused on round-tripping export-session state rather than full AVFoundation editing APIs.
- `OutputSettingsAssistant` exposes `audioSettings` and `videoSettings` as `serde_json::Value` dictionaries rather than typed Rust key/value wrappers.
- The crate models `AVAssetWriterInput` through `InputId` plus `Writer` methods rather than exposing a standalone Rust object for the Objective-C input instance.
- `AVAssetWriterInputPixelBufferAdaptor`, `AVAssetWriterInputTaggedPixelBufferGroupAdaptor`, `AVAssetWriterInputMetadataAdaptor`, and `AVAssetWriterInputCaptionAdaptor` are all wrapped, but excluded from coverage percentage because Apple marks those Swift-imported interfaces deprecated in favor of newer receiver APIs.

