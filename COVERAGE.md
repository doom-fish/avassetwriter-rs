# AVFoundation coverage audit

Audited against the current macOS SDK returned by `xcrun --sdk macosx --show-sdk-path`, primarily:

- `AVAssetWriter.h`
- `AVAssetWriterInput.h`
- `AVOutputSettingsAssistant.h`
- `AVAssetExportSession.h`
- `AVMediaFormat.h`
- `AVVideoSettings.h`
- `AVAudioSettings.h`
- `AVAssetTrack.h`

Legend:

- ✅ implemented
- 🟡 partial
- ⏭️ skipped

## AVAssetWriter

| Apple API row | Status | Notes |
| --- | --- | --- |
| `assetWriterWithURL:fileType:error:` | ⏭️ skipped | Deprecated factory; safe Rust surface uses designated init wrapper `Writer::create`. |
| `initWithURL:fileType:error:` | ✅ implemented | `Writer::create(path, file_type)`. |
| `initWithContentType:` | ✅ implemented | `Writer::create_segmented(...)` uses the content-type initializer for segmented output. |
| `canApplyOutputSettings:forMediaType:` | ✅ implemented | `Writer::can_apply_output_settings`. |
| `canAddInput:` / `addInput:` | ✅ implemented | Covered by sample-buffer, PCM, generic JSON, metadata, caption, and pixel-buffer input constructors. |
| `canAddInputGroup:` / `addInputGroup:` | ✅ implemented | `Writer::add_input_group`. |
| `startWriting` / `startSessionAtSourceTime:` | ✅ implemented | `Writer::start_session`. |
| `endSessionAtSourceTime:` | ✅ implemented | `Writer::end_session`. |
| `finishWritingWithCompletionHandler:` | ✅ implemented | `Writer::finish` blocks until the completion handler fires. |
| `cancelWriting` | ✅ implemented | `Writer::cancel`. |
| `outputURL` / `outputFileType` / `availableMediaTypes` | ✅ implemented | Read back through `Writer::{output_path,output_file_type,available_media_types}`. |
| `status` / `error` | ✅ implemented | `WriterStatus` + error string readback. |
| `metadata` | ✅ implemented | `Writer::{metadata,set_metadata}`. |
| `shouldOptimizeForNetworkUse` | ✅ implemented | `Writer::{set_optimize_for_network_use,should_optimize_for_network_use}`. |
| `directoryForTemporaryFiles` | ✅ implemented | `Writer::{set_directory_for_temporary_files,directory_for_temporary_files}`. |
| `movieFragmentInterval` / `initialMovieFragmentInterval` | ✅ implemented | Writer fragment interval readback + setters. |
| `initialMovieFragmentSequenceNumber` | ✅ implemented | Readback + setter. |
| `producesCombinableFragments` | ✅ implemented | Readback + setter. |
| `overallDurationHint` / `movieTimeScale` | ✅ implemented | Readback + setters. |
| `preferredOutputSegmentInterval` / `initialSegmentStartTime` | ✅ implemented | Readback + setters for segmented output. |
| `outputFileTypeProfile` | ✅ implemented | `FileTypeProfile` readback + setter. |
| Segmented-output delegate callbacks | ✅ implemented | Segmented writer bridge + `SegmentOutput` callback surface. |

## AVAssetWriterInput

| Apple API row | Status | Notes |
| --- | --- | --- |
| `initWithMediaType:outputSettings:` | ✅ implemented | `Writer::add_input`, `add_video_input_from_preset`, `add_audio_input_pcm`, caption/metadata helpers. |
| `initWithMediaType:outputSettings:sourceFormatHint:` | ✅ implemented | Sample-buffer video/audio and generic input helpers. |
| `appendSampleBuffer:` | ✅ implemented | `Writer::append_sample`. |
| `markAsFinished` | ✅ implemented | `Writer::mark_input_as_finished` and `Writer::finish`. |
| `requestMediaDataWhenReadyOnQueue:usingBlock:` | ✅ implemented | `Writer::request_input_media_data_when_ready`. |
| `mediaType` / `outputSettings` / `sourceFormatHint` | ✅ implemented | `Writer::{input_media_type,input_output_settings,input_source_format_hint}`. |
| `metadata` | ✅ implemented | `Writer::{input_metadata,set_input_metadata}`. |
| `isReadyForMoreMediaData` / `expectsMediaDataInRealTime` | ✅ implemented | Input-state readback + setter. |
| `languageCode` / `extendedLanguageTag` | ✅ implemented | Readback + setters. |
| `naturalSize` / `transform` / `preferredVolume` | ✅ implemented | Readback + setters. |
| `marksOutputTrackAsEnabled` | ✅ implemented | Readback + setter. |
| `mediaTimeScale` | ✅ implemented | Readback + setter. |
| `preferredMediaChunkDuration` / `preferredMediaChunkAlignment` | ✅ implemented | Readback + setters. |
| `sampleReferenceBaseURL` | ✅ implemented | Readback + setter. |
| `mediaDataLocation` | ✅ implemented | All three macOS-visible constants are mapped. |
| `canAddTrackAssociationWithTrackOfInput:type:` / `addTrackAssociationWithTrackOfInput:type:` | ✅ implemented | `Writer::{can_add_track_association,add_track_association}`. |
| `performsMultiPassEncodingIfSupported` / `canPerformMultiplePasses` | ✅ implemented | Readback + setter. |
| `currentPassDescription` | ✅ implemented | Readback via `InputPassDescription`. |
| `respondToEachPassDescriptionOnQueue:usingBlock:` | ✅ implemented | `Writer::respond_to_each_pass_description`. |
| `markCurrentPassAsFinished` | ✅ implemented | `Writer::mark_current_pass_as_finished`. |

## Related writer adaptors and groups

| Apple API row | Status | Notes |
| --- | --- | --- |
| `AVAssetWriterInputPixelBufferAdaptor` | ✅ implemented | Attach/read pool/append pixel buffers. |
| `AVAssetWriterInputTaggedPixelBufferGroupAdaptor` | ✅ implemented | Attach/read pool/append tagged groups via Swift-native `CMTaggedBuffer`. |
| `AVAssetWriterInputMetadataAdaptor` | ✅ implemented | Attach + append timed metadata groups. |
| `AVAssetWriterInputCaptionAdaptor` | ✅ implemented | Attach + append captions / caption groups. |
| `AVAssetWriterInputGroup` | ✅ implemented | Safe grouping via `InputGroupInfo` + `Writer::add_input_group`. |

## AVOutputSettingsAssistant

| Apple API row | Status | Notes |
| --- | --- | --- |
| `availableOutputSettingsPresets` | ✅ implemented | `OutputSettingsAssistant::available_presets`. |
| `outputSettingsAssistantWithPreset:` | ✅ implemented | `OutputSettingsAssistant::new` and `Writer::add_video_input_from_preset`. |
| `audioSettings` / `videoSettings` | ✅ implemented | Surfaced as `serde_json::Value` dictionaries. |
| `outputFileType` | ✅ implemented | `OutputSettingsAssistant::output_file_type`. |
| `sourceAudioFormat` | ✅ implemented | Get/set `CMFormatDescription` hint. |
| `sourceVideoFormat` | ✅ implemented | Get/set `CMFormatDescription` hint. |
| `sourceVideoAverageFrameDuration` | ✅ implemented | Get/set `Time`. |
| `sourceVideoMinFrameDuration` | ✅ implemented | Get/set `Time`. |

## AVAssetExportSession

| Apple API row | Status | Notes |
| --- | --- | --- |
| `allExportPresets` | ✅ implemented | `ExportSession::available_presets`. |
| `exportPresetsCompatibleWithAsset:` | ✅ implemented | `ExportSession::compatible_presets` (via the Swift-imported compatibility helper). |
| `determineCompatibilityOfExportPreset:withAsset:outputFileType:completionHandler:` | ✅ implemented | `ExportSession::determine_compatibility`. |
| `exportSessionWithAsset:presetName:` / `initWithAsset:presetName:` | ✅ implemented | `ExportSession::new` creates an internal `AVURLAsset`-backed session. |
| `presetName` | ✅ implemented | `ExportSession::preset_name`. |
| `asset` | ✅ implemented | `Asset` now exposes URL-backed `AVAsset` loading, duration/metadata readback, and per-key status inspection. |
| `outputFileType` / `outputURL` | ✅ implemented | `ExportSession::{output_file_type,set_output_file_type,output_path,set_output_path}`. |
| `shouldOptimizeForNetworkUse` | ✅ implemented | Getter + setter. |
| `allowsParallelizedExport` | ✅ implemented | Getter + setter (setter reports unsupported runtimes on macOS < 14). |
| `status` / `error` / `progress` | ✅ implemented | `ExportStatus`, error string, and progress readback. |
| `exportAsynchronouslyWithCompletionHandler:` | ✅ implemented | `ExportSession::export` blocks until completion. |
| `cancelExport` | ✅ implemented | `ExportSession::cancel`. |
| `supportedFileTypes` | ✅ implemented | `ExportSession::supported_file_types`. |
| `determineCompatibleFileTypesWithCompletionHandler:` | ✅ implemented | `ExportSession::compatible_file_types`. |
| `timeRange` | ✅ implemented | Getter + setter. |
| `maxDuration` | ⏭️ skipped | Deprecated in Apple headers; safe wrapper uses `estimateMaximumDurationWithCompletionHandler:` instead. |
| `estimatedOutputFileLength` | ⏭️ skipped | Deprecated in Apple headers; safe wrapper uses `estimateOutputFileLengthWithCompletionHandler:` instead. |
| `fileLengthLimit` | ✅ implemented | Getter + setter. |
| `estimateMaximumDurationWithCompletionHandler:` | ✅ implemented | `ExportSession::estimated_maximum_duration`. |
| `estimateOutputFileLengthWithCompletionHandler:` | ✅ implemented | `ExportSession::estimated_output_file_length`. |
| `metadata` | ✅ implemented | Getter + setter using `MetadataItem`. |
| `metadataItemFilter` | ✅ implemented | `MetadataItemFilter` + `ExportSession::{metadata_item_filter,set_metadata_item_filter}`. |
| `audioTimePitchAlgorithm` | ⏭️ skipped | Requires a dedicated safe wrapper for AVFAudio processing constants/validation. |
| `audioMix` | ✅ implemented | `AudioMix` + `ExportSession::{audio_mix,set_audio_mix}`. |
| `videoComposition` | ✅ implemented | `VideoComposition` + `ExportSession::{video_composition,set_video_composition}`. |
| `customVideoCompositor` | ✅ implemented | `VideoCompositor`, `VideoCompositorClass`, and `ExportSession::custom_video_compositor` cover the protocol/object readback path. |
| `audioTrackGroupHandling` | ✅ implemented | `TrackGroupOutputHandling` getter + setter. |
| `canPerformMultiplePassesOverSourceMediaData` | ✅ implemented | Getter + setter. |
| `directoryForTemporaryFiles` | ✅ implemented | Getter + setter. |

## Constants and keyed settings

| Surface | Status | Notes |
| --- | --- | --- |
| `AVFileType*` constants | ✅ implemented | All 26 macOS-visible `AVFileType` constants are mapped in the Swift bridge and `FileType` enum. |
| `AVOutputSettingsPreset*` constants | ✅ implemented | All 15 macOS-visible output-settings presets are mapped through `VideoPreset` + `OutputSettingsAssistant`. |
| `AVAssetExportPreset*` constants | ✅ implemented | All macOS-visible export presets from `AVAssetExportSession.h` are mapped through `ExportPreset`. |
| `AVAssetTrack.AssociationType*` constants | ✅ implemented | Covered by `TrackAssociationType`. |
| `AVAssetWriterInputMediaDataLocation*` constants | ✅ implemented | Covered by `InputMediaDataLocation`, including sparse interleaving on macOS 26+. |
| `AVAudioSettings.h` keys used directly by this crate | ✅ implemented | `AVFormatIDKey`, `AVSampleRateKey`, `AVNumberOfChannelsKey`, and `AVEncoderBitRateKey`. |
| `AVVideoSettings.h` dictionaries | 🟡 partial | Surfaced as raw JSON dictionaries via `OutputSettingsAssistant::video_settings` and `Writer::input_output_settings`; there is not yet a hand-typed key enum/wrapper layer. |
| `AVMediaTypeMetadataObject` | ⏭️ skipped | Not available on macOS; documented as iOS/tvOS/visionOS-only. |

## Deferred / intentionally skipped summary

1. `AVAssetExportSession.audioTimePitchAlgorithm` remains deferred until this crate grows a safe constant layer for AVFAudio processing algorithms instead of exposing stringly typed values.
2. `MetadataItemFilter`, `AudioMix`, `VideoComposition`, and `VideoCompositor` are intentionally lightweight wrappers focused on export-session interop rather than full AVFoundation instruction/input-parameter editing APIs.
3. Deprecated `AVAssetExportSession.maxDuration` / `estimatedOutputFileLength` remain skipped in favor of the modern async estimate APIs.
4. `AVVideoSettings` constants are currently exposed as JSON dictionaries rather than as a dedicated typed Rust wrapper.
