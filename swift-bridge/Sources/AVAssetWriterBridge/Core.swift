import AVAssetWriterObjCBridge
import AVFoundation
import CoreMedia
import CoreVideo
import Foundation

public typealias AVWReadyCallback = @convention(c) (UnsafeMutableRawPointer?) -> Void

public typealias AVWPassDescriptionCallback = @convention(c) (
    UnsafePointer<AVWOwnedPayload>?,
    UnsafeMutableRawPointer?
) -> Void

public typealias AVWSegmentCallback = @convention(c) (
    UnsafePointer<UInt8>?,
    Int,
    Int32,
    UnsafePointer<AVWOwnedPayload>?,
    UnsafeMutableRawPointer?
) -> Void

public typealias AVWDropCallback = @convention(c) (UnsafeMutableRawPointer?) -> Void

enum BridgeError: LocalizedError {
    case message(String)

    var errorDescription: String? {
        switch self {
        case .message(let message):
            return message
        }
    }
}

final class CallbackLifetimeBox {
    private let lock = NSLock()
    private var userdata: UnsafeMutableRawPointer?
    private let dropUserdata: AVWDropCallback?
    private var acceptsInvocations = true
    private var nativeOwned = true
    private var invocationCount = 0
    private var dropped = false

    init(userdata: UnsafeMutableRawPointer?, dropUserdata: AVWDropCallback?) {
        self.userdata = userdata
        self.dropUserdata = dropUserdata
    }

    func beginInvocation() -> (Bool, UnsafeMutableRawPointer?) {
        lock.lock()
        defer { lock.unlock() }
        guard acceptsInvocations, nativeOwned, invocationCount == 0 else {
            return (false, nil)
        }
        invocationCount = 1
        return (true, userdata)
    }

    func endInvocation() {
        let pendingDrop: (UnsafeMutableRawPointer, AVWDropCallback)?
        lock.lock()
        invocationCount = 0
        pendingDrop = takeDropIfReady()
        lock.unlock()
        performDrop(pendingDrop)
    }

    func deactivate() {
        lock.lock()
        acceptsInvocations = false
        lock.unlock()
    }

    func nativeRemoved() {
        let pendingDrop: (UnsafeMutableRawPointer, AVWDropCallback)?
        lock.lock()
        acceptsInvocations = false
        nativeOwned = false
        pendingDrop = takeDropIfReady()
        lock.unlock()
        performDrop(pendingDrop)
    }

    deinit {
        let pendingDrop: (UnsafeMutableRawPointer, AVWDropCallback)?
        lock.lock()
        acceptsInvocations = false
        nativeOwned = false
        pendingDrop = takeDropIfReady()
        lock.unlock()
        performDrop(pendingDrop)
    }

    private func takeDropIfReady() -> (UnsafeMutableRawPointer, AVWDropCallback)? {
        guard !nativeOwned, invocationCount == 0, !dropped else { return nil }
        dropped = true
        guard let userdata, let dropUserdata else { return nil }
        self.userdata = nil
        return (userdata, dropUserdata)
    }

    private func performDrop(_ pendingDrop: (UnsafeMutableRawPointer, AVWDropCallback)?) {
        guard let (userdata, dropUserdata) = pendingDrop else { return }
        dropUserdata(userdata)
    }
}

final class NativeCallbackOwner {
    let lifetime: CallbackLifetimeBox

    init(lifetime: CallbackLifetimeBox) {
        self.lifetime = lifetime
    }

    deinit {
        lifetime.nativeRemoved()
    }
}

final class SegmentCallbackBox {
    let callback: AVWSegmentCallback?
    let lifetime: CallbackLifetimeBox

    init(
        callback: AVWSegmentCallback?,
        userdata: UnsafeMutableRawPointer?,
        dropUserdata: AVWDropCallback?
    ) {
        self.callback = callback
        self.lifetime = CallbackLifetimeBox(userdata: userdata, dropUserdata: dropUserdata)
    }

    func emit(
        data: Data,
        segmentType: Int32,
        report: SegmentReportPayload?,
        userdata: UnsafeMutableRawPointer?
    ) {
        guard let callback else { return }
        let reportPayload = report.flatMap { makePayload($0) }
        data.withUnsafeBytes { bytes in
            callback(
                bytes.bindMemory(to: UInt8.self).baseAddress,
                data.count,
                segmentType,
                reportPayload.map { UnsafePointer($0) },
                userdata
            )
        }
        avw_payload_free(reportPayload)
    }

    func nativeRemoved() {
        lifetime.nativeRemoved()
    }

    deinit {
        lifetime.nativeRemoved()
    }
}

struct TimePayload {
    let kind: String
    let value: Int64?
    let timescale: Int32?
}

struct TimeRangePayload {
    let start: TimePayload
    let duration: TimePayload
}

struct MetadataValuePayload {
    let kind: String
    let stringValue: String?
    let integerValue: Int64?
    let floatValue: Double?
    let booleanValue: Bool?
    let dataValue: [UInt8]?

    init(
        kind: String,
        stringValue: String? = nil,
        integerValue: Int64? = nil,
        floatValue: Double? = nil,
        booleanValue: Bool? = nil,
        dataValue: [UInt8]? = nil
    ) {
        self.kind = kind
        self.stringValue = stringValue
        self.integerValue = integerValue
        self.floatValue = floatValue
        self.booleanValue = booleanValue
        self.dataValue = dataValue
    }

}

struct MetadataItemPayload {
    let identifier: String
    let value: MetadataValuePayload
    let dataType: String?
    let extendedLanguageTag: String?
    let localeIdentifier: String?
}

struct InputGroupPayload {
    let inputs: [Int32]
    let defaultInput: Int32?
}

struct PassDescriptionPayload {
    let sourceTimeRanges: [TimeRangePayload]
}

struct SegmentReportSamplePayload {
    let presentationTimeStamp: TimePayload
    let offset: Int64
    let length: Int64
    let isSyncSample: Bool
}

struct SegmentTrackReportPayload {
    let trackId: Int32
    let mediaType: String
    let earliestPresentationTimeStamp: TimePayload
    let duration: TimePayload
    let firstVideoSampleInformation: SegmentReportSamplePayload?
}

struct SegmentReportPayload {
    let segmentType: Int32
    let trackReports: [SegmentTrackReportPayload]
}

struct SizePayload {
    let width: Double
    let height: Double
}

struct TransformPayload {
    let a: Double
    let b: Double
    let c: Double
    let d: Double
    let tx: Double
    let ty: Double
}

struct WriterInfoPayload {
    let outputPath: String?
    let outputFileType: String?
    let availableMediaTypes: [String]
    let status: Int32
    let errorMessage: String?
    let shouldOptimizeForNetworkUse: Bool
    let directoryForTemporaryFiles: String?
    let inputs: [Int32]
    let inputGroups: [InputGroupPayload]
    let movieFragmentInterval: TimePayload
    let initialMovieFragmentInterval: TimePayload
    let initialMovieFragmentSequenceNumber: Int64
    let producesCombinableFragments: Bool
    let overallDurationHint: TimePayload
    let movieTimeScale: Int32
    let preferredOutputSegmentInterval: TimePayload
    let initialSegmentStartTime: TimePayload
    let outputFileTypeProfile: String?
}

struct InputInfoPayload {
    let mediaType: String
    let outputSettings: PropertyValue?
    let readyForMoreMediaData: Bool
    let expectsMediaDataInRealTime: Bool
    let languageCode: String?
    let extendedLanguageTag: String?
    let naturalSize: SizePayload
    let transform: TransformPayload
    let preferredVolume: Float
    let marksOutputTrackAsEnabled: Bool
    let mediaTimeScale: Int32
    let preferredMediaChunkDuration: TimePayload
    let preferredMediaChunkAlignment: Int64
    let sampleReferenceBaseURL: String?
    let mediaDataLocation: String?
    let performsMultiPassEncodingIfSupported: Bool
    let canPerformMultiplePasses: Bool
    let currentPassDescription: PassDescriptionPayload?
    let pixelBufferSourceAttributes: PropertyValue?
    let taggedPixelBufferSourceAttributes: PropertyValue?
    let hasMetadataAdaptor: Bool
    let hasCaptionAdaptor: Bool
}

struct CaptionPayload {
    let text: String
    let timeRange: TimeRangePayload
    let region: CaptionRegionPayload?
    let textAlignment: CaptionTextAlignmentRaw?
    let animation: CaptionAnimationRaw?
    let rubySpans: [CaptionRubySpanPayload]
}

struct CaptionGroupPayload {
    let captions: [CaptionPayload]
    let timeRange: TimeRangePayload
}

struct TimedMetadataGroupPayload {
    let items: [MetadataItemPayload]
    let timeRange: TimeRangePayload
}

struct MetadataSpecificationPayload {
    let identifier: String
    let dataType: String
    let extendedLanguageTag: String?
}

struct OutputSettingsAssistantInfoPayload {
    let audioSettings: PropertyValue?
    let videoSettings: PropertyValue?
    let outputFileType: String?
    let sourceVideoAverageFrameDuration: TimePayload
    let sourceVideoMinFrameDuration: TimePayload
}

struct ExportSessionInfoPayload {
    let presetName: String
    let assetPath: String?
    let outputFileType: String?
    let outputPath: String?
    let shouldOptimizeForNetworkUse: Bool
    let allowsParallelizedExport: Bool
    let status: Int32
    let errorMessage: String?
    let progress: Float
    let supportedFileTypes: [String]
    let timeRange: TimeRangePayload
    let fileLengthLimit: Int64
    let canPerformMultiplePassesOverSourceMediaData: Bool
    let directoryForTemporaryFiles: String?
    let audioTrackGroupHandling: UInt64
}

struct AudioMixInfoPayload {
    let inputParameterCount: Int
}

struct VideoCompositionInfoPayload {
    let frameDuration: TimePayload
    let renderSize: SizePayload
    let renderScale: Float
    let instructionCount: Int
    let customVideoCompositorClassName: String?
}

struct VideoCompositorInfoPayload {
    let className: String
    let sourcePixelBufferAttributes: PropertyValue?
    let requiredPixelBufferAttributesForRenderContext: PropertyValue?
    let supportsWideColorSourceFrames: Bool
    let supportsHDRSourceFrames: Bool
}

extension SizePayload: BridgePayload {
    typealias ABI = AVWSize

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            width: width.toABI(arena),
            height: height.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            width: try .fromABI(raw.width),
            height: try .fromABI(raw.height)
        )
    }
}

extension TransformPayload: BridgePayload {
    typealias ABI = AVWTransform

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            a: a.toABI(arena),
            b: b.toABI(arena),
            c: c.toABI(arena),
            d: d.toABI(arena),
            tx: tx.toABI(arena),
            ty: ty.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            a: try .fromABI(raw.a),
            b: try .fromABI(raw.b),
            c: try .fromABI(raw.c),
            d: try .fromABI(raw.d),
            tx: try .fromABI(raw.tx),
            ty: try .fromABI(raw.ty)
        )
    }
}

extension TimeRangePayload: BridgePayload {
    typealias ABI = AVWTimeRange

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            start: start.toABI(arena),
            duration: duration.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            start: try .fromABI(raw.start),
            duration: try .fromABI(raw.duration)
        )
    }
}

extension MetadataItemPayload: BridgePayload {
    typealias ABI = AVWMetadataItem

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            identifier: identifier.toABI(arena),
            value: value.toABI(arena),
            data_type: dataType.toABI(arena),
            extended_language_tag: extendedLanguageTag.toABI(arena),
            locale_identifier: localeIdentifier.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            identifier: try .fromABI(raw.identifier),
            value: try .fromABI(raw.value),
            dataType: try .fromABI(raw.data_type),
            extendedLanguageTag: try .fromABI(raw.extended_language_tag),
            localeIdentifier: try .fromABI(raw.locale_identifier)
        )
    }
}

extension TimedMetadataGroupPayload: BridgePayload {
    typealias ABI = AVWTimedMetadataGroup

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            items: items.toABI(arena),
            time_range: timeRange.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            items: try .fromABI(raw.items),
            timeRange: try .fromABI(raw.time_range)
        )
    }
}

extension MetadataSpecificationPayload: BridgePayload {
    typealias ABI = AVWMetadataSpecification

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            identifier: identifier.toABI(arena),
            data_type: dataType.toABI(arena),
            extended_language_tag: extendedLanguageTag.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            identifier: try .fromABI(raw.identifier),
            dataType: try .fromABI(raw.data_type),
            extendedLanguageTag: try .fromABI(raw.extended_language_tag)
        )
    }
}

extension InputGroupPayload: BridgePayload {
    typealias ABI = AVWInputGroup

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            inputs: inputs.toABI(arena),
            default_input: defaultInput.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            inputs: try .fromABI(raw.inputs),
            defaultInput: try .fromABI(raw.default_input)
        )
    }
}

extension PassDescriptionPayload: BridgePayload {
    typealias ABI = AVWPassDescription

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            source_time_ranges: sourceTimeRanges.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            sourceTimeRanges: try .fromABI(raw.source_time_ranges)
        )
    }
}

extension SegmentReportSamplePayload: BridgePayload {
    typealias ABI = AVWSegmentReportSample

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            presentation_time_stamp: presentationTimeStamp.toABI(arena),
            offset: offset.toABI(arena),
            length: length.toABI(arena),
            is_sync_sample: isSyncSample.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            presentationTimeStamp: try .fromABI(raw.presentation_time_stamp),
            offset: try .fromABI(raw.offset),
            length: try .fromABI(raw.length),
            isSyncSample: try .fromABI(raw.is_sync_sample)
        )
    }
}

extension SegmentTrackReportPayload: BridgePayload {
    typealias ABI = AVWSegmentTrackReport

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            track_id: trackId.toABI(arena),
            media_type: mediaType.toABI(arena),
            earliest_presentation_time_stamp: earliestPresentationTimeStamp.toABI(arena),
            duration: duration.toABI(arena),
            first_video_sample_information: firstVideoSampleInformation.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            trackId: try .fromABI(raw.track_id),
            mediaType: try .fromABI(raw.media_type),
            earliestPresentationTimeStamp: try .fromABI(raw.earliest_presentation_time_stamp),
            duration: try .fromABI(raw.duration),
            firstVideoSampleInformation: try .fromABI(raw.first_video_sample_information)
        )
    }
}

extension SegmentReportPayload: BridgePayload {
    typealias ABI = AVWSegmentReport

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            segment_type: segmentType.toABI(arena),
            track_reports: trackReports.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            segmentType: try .fromABI(raw.segment_type),
            trackReports: try .fromABI(raw.track_reports)
        )
    }
}

extension WriterInfoPayload: BridgePayload {
    typealias ABI = AVWWriterInfo

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            output_path: outputPath.toABI(arena),
            output_file_type: outputFileType.toABI(arena),
            available_media_types: availableMediaTypes.toABI(arena),
            status: status.toABI(arena),
            error_message: errorMessage.toABI(arena),
            should_optimize_for_network_use: shouldOptimizeForNetworkUse.toABI(arena),
            directory_for_temporary_files: directoryForTemporaryFiles.toABI(arena),
            inputs: inputs.toABI(arena),
            input_groups: inputGroups.toABI(arena),
            movie_fragment_interval: movieFragmentInterval.toABI(arena),
            initial_movie_fragment_interval: initialMovieFragmentInterval.toABI(arena),
            initial_movie_fragment_sequence_number: initialMovieFragmentSequenceNumber.toABI(arena),
            produces_combinable_fragments: producesCombinableFragments.toABI(arena),
            overall_duration_hint: overallDurationHint.toABI(arena),
            movie_time_scale: movieTimeScale.toABI(arena),
            preferred_output_segment_interval: preferredOutputSegmentInterval.toABI(arena),
            initial_segment_start_time: initialSegmentStartTime.toABI(arena),
            output_file_type_profile: outputFileTypeProfile.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            outputPath: try .fromABI(raw.output_path),
            outputFileType: try .fromABI(raw.output_file_type),
            availableMediaTypes: try .fromABI(raw.available_media_types),
            status: try .fromABI(raw.status),
            errorMessage: try .fromABI(raw.error_message),
            shouldOptimizeForNetworkUse: try .fromABI(raw.should_optimize_for_network_use),
            directoryForTemporaryFiles: try .fromABI(raw.directory_for_temporary_files),
            inputs: try .fromABI(raw.inputs),
            inputGroups: try .fromABI(raw.input_groups),
            movieFragmentInterval: try .fromABI(raw.movie_fragment_interval),
            initialMovieFragmentInterval: try .fromABI(raw.initial_movie_fragment_interval),
            initialMovieFragmentSequenceNumber: try .fromABI(raw.initial_movie_fragment_sequence_number),
            producesCombinableFragments: try .fromABI(raw.produces_combinable_fragments),
            overallDurationHint: try .fromABI(raw.overall_duration_hint),
            movieTimeScale: try .fromABI(raw.movie_time_scale),
            preferredOutputSegmentInterval: try .fromABI(raw.preferred_output_segment_interval),
            initialSegmentStartTime: try .fromABI(raw.initial_segment_start_time),
            outputFileTypeProfile: try .fromABI(raw.output_file_type_profile)
        )
    }
}

extension InputInfoPayload: BridgePayload {
    typealias ABI = AVWInputInfo

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            media_type: mediaType.toABI(arena),
            output_settings: outputSettings.toABI(arena),
            ready_for_more_media_data: readyForMoreMediaData.toABI(arena),
            expects_media_data_in_real_time: expectsMediaDataInRealTime.toABI(arena),
            language_code: languageCode.toABI(arena),
            extended_language_tag: extendedLanguageTag.toABI(arena),
            natural_size: naturalSize.toABI(arena),
            transform: transform.toABI(arena),
            preferred_volume: preferredVolume.toABI(arena),
            marks_output_track_as_enabled: marksOutputTrackAsEnabled.toABI(arena),
            media_time_scale: mediaTimeScale.toABI(arena),
            preferred_media_chunk_duration: preferredMediaChunkDuration.toABI(arena),
            preferred_media_chunk_alignment: preferredMediaChunkAlignment.toABI(arena),
            sample_reference_base_url: sampleReferenceBaseURL.toABI(arena),
            media_data_location: mediaDataLocation.toABI(arena),
            performs_multi_pass_encoding_if_supported: performsMultiPassEncodingIfSupported.toABI(arena),
            can_perform_multiple_passes: canPerformMultiplePasses.toABI(arena),
            current_pass_description: currentPassDescription.toABI(arena),
            pixel_buffer_source_attributes: pixelBufferSourceAttributes.toABI(arena),
            tagged_pixel_buffer_source_attributes: taggedPixelBufferSourceAttributes.toABI(arena),
            has_metadata_adaptor: hasMetadataAdaptor.toABI(arena),
            has_caption_adaptor: hasCaptionAdaptor.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            mediaType: try .fromABI(raw.media_type),
            outputSettings: try .fromABI(raw.output_settings),
            readyForMoreMediaData: try .fromABI(raw.ready_for_more_media_data),
            expectsMediaDataInRealTime: try .fromABI(raw.expects_media_data_in_real_time),
            languageCode: try .fromABI(raw.language_code),
            extendedLanguageTag: try .fromABI(raw.extended_language_tag),
            naturalSize: try .fromABI(raw.natural_size),
            transform: try .fromABI(raw.transform),
            preferredVolume: try .fromABI(raw.preferred_volume),
            marksOutputTrackAsEnabled: try .fromABI(raw.marks_output_track_as_enabled),
            mediaTimeScale: try .fromABI(raw.media_time_scale),
            preferredMediaChunkDuration: try .fromABI(raw.preferred_media_chunk_duration),
            preferredMediaChunkAlignment: try .fromABI(raw.preferred_media_chunk_alignment),
            sampleReferenceBaseURL: try .fromABI(raw.sample_reference_base_url),
            mediaDataLocation: try .fromABI(raw.media_data_location),
            performsMultiPassEncodingIfSupported: try .fromABI(raw.performs_multi_pass_encoding_if_supported),
            canPerformMultiplePasses: try .fromABI(raw.can_perform_multiple_passes),
            currentPassDescription: try .fromABI(raw.current_pass_description),
            pixelBufferSourceAttributes: try .fromABI(raw.pixel_buffer_source_attributes),
            taggedPixelBufferSourceAttributes: try .fromABI(raw.tagged_pixel_buffer_source_attributes),
            hasMetadataAdaptor: try .fromABI(raw.has_metadata_adaptor),
            hasCaptionAdaptor: try .fromABI(raw.has_caption_adaptor)
        )
    }
}

extension CaptionPayload: BridgePayload {
    typealias ABI = AVWCaption

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            text: text.toABI(arena),
            time_range: timeRange.toABI(arena),
            region: region.toABI(arena),
            text_alignment: textAlignment.toABI(arena),
            animation: animation.toABI(arena),
            ruby_spans: rubySpans.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            text: try .fromABI(raw.text),
            timeRange: try .fromABI(raw.time_range),
            region: try .fromABI(raw.region),
            textAlignment: try .fromABI(raw.text_alignment),
            animation: try .fromABI(raw.animation),
            rubySpans: try .fromABI(raw.ruby_spans)
        )
    }
}

extension CaptionGroupPayload: BridgePayload {
    typealias ABI = AVWCaptionGroup

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            captions: captions.toABI(arena),
            time_range: timeRange.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            captions: try .fromABI(raw.captions),
            timeRange: try .fromABI(raw.time_range)
        )
    }
}

extension OutputSettingsAssistantInfoPayload: BridgePayload {
    typealias ABI = AVWOutputSettingsAssistantInfo

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            audio_settings: audioSettings.toABI(arena),
            video_settings: videoSettings.toABI(arena),
            output_file_type: outputFileType.toABI(arena),
            source_video_average_frame_duration: sourceVideoAverageFrameDuration.toABI(arena),
            source_video_min_frame_duration: sourceVideoMinFrameDuration.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            audioSettings: try .fromABI(raw.audio_settings),
            videoSettings: try .fromABI(raw.video_settings),
            outputFileType: try .fromABI(raw.output_file_type),
            sourceVideoAverageFrameDuration: try .fromABI(raw.source_video_average_frame_duration),
            sourceVideoMinFrameDuration: try .fromABI(raw.source_video_min_frame_duration)
        )
    }
}

extension ExportSessionInfoPayload: BridgePayload {
    typealias ABI = AVWExportSessionInfo

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            preset_name: presetName.toABI(arena),
            asset_path: assetPath.toABI(arena),
            output_file_type: outputFileType.toABI(arena),
            output_path: outputPath.toABI(arena),
            should_optimize_for_network_use: shouldOptimizeForNetworkUse.toABI(arena),
            allows_parallelized_export: allowsParallelizedExport.toABI(arena),
            status: status.toABI(arena),
            error_message: errorMessage.toABI(arena),
            progress: progress.toABI(arena),
            supported_file_types: supportedFileTypes.toABI(arena),
            time_range: timeRange.toABI(arena),
            file_length_limit: fileLengthLimit.toABI(arena),
            can_perform_multiple_passes_over_source_media_data: canPerformMultiplePassesOverSourceMediaData.toABI(arena),
            directory_for_temporary_files: directoryForTemporaryFiles.toABI(arena),
            audio_track_group_handling: audioTrackGroupHandling.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            presetName: try .fromABI(raw.preset_name),
            assetPath: try .fromABI(raw.asset_path),
            outputFileType: try .fromABI(raw.output_file_type),
            outputPath: try .fromABI(raw.output_path),
            shouldOptimizeForNetworkUse: try .fromABI(raw.should_optimize_for_network_use),
            allowsParallelizedExport: try .fromABI(raw.allows_parallelized_export),
            status: try .fromABI(raw.status),
            errorMessage: try .fromABI(raw.error_message),
            progress: try .fromABI(raw.progress),
            supportedFileTypes: try .fromABI(raw.supported_file_types),
            timeRange: try .fromABI(raw.time_range),
            fileLengthLimit: try .fromABI(raw.file_length_limit),
            canPerformMultiplePassesOverSourceMediaData: try .fromABI(raw.can_perform_multiple_passes_over_source_media_data),
            directoryForTemporaryFiles: try .fromABI(raw.directory_for_temporary_files),
            audioTrackGroupHandling: try .fromABI(raw.audio_track_group_handling)
        )
    }
}

extension AudioMixInfoPayload: BridgePayload {
    typealias ABI = AVWAudioMixInfo

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            input_parameter_count: inputParameterCount.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            inputParameterCount: try .fromABI(raw.input_parameter_count)
        )
    }
}

extension VideoCompositionInfoPayload: BridgePayload {
    typealias ABI = AVWVideoCompositionInfo

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            frame_duration: frameDuration.toABI(arena),
            render_size: renderSize.toABI(arena),
            render_scale: renderScale.toABI(arena),
            instruction_count: instructionCount.toABI(arena),
            custom_video_compositor_class_name: customVideoCompositorClassName.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            frameDuration: try .fromABI(raw.frame_duration),
            renderSize: try .fromABI(raw.render_size),
            renderScale: try .fromABI(raw.render_scale),
            instructionCount: try .fromABI(raw.instruction_count),
            customVideoCompositorClassName: try .fromABI(raw.custom_video_compositor_class_name)
        )
    }
}

extension VideoCompositorInfoPayload: BridgePayload {
    typealias ABI = AVWVideoCompositorInfo

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            class_name: className.toABI(arena),
            source_pixel_buffer_attributes: sourcePixelBufferAttributes.toABI(arena),
            required_pixel_buffer_attributes_for_render_context: requiredPixelBufferAttributesForRenderContext.toABI(arena),
            supports_wide_color_source_frames: supportsWideColorSourceFrames.toABI(arena),
            supports_hdr_source_frames: supportsHDRSourceFrames.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            className: try .fromABI(raw.class_name),
            sourcePixelBufferAttributes: try .fromABI(raw.source_pixel_buffer_attributes),
            requiredPixelBufferAttributesForRenderContext: try .fromABI(raw.required_pixel_buffer_attributes_for_render_context),
            supportsWideColorSourceFrames: try .fromABI(raw.supports_wide_color_source_frames),
            supportsHDRSourceFrames: try .fromABI(raw.supports_hdr_source_frames)
        )
    }
}
