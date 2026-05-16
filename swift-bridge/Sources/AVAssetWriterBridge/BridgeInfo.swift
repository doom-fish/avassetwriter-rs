import AVFoundation
import CoreMedia
import CoreVideo
import Dispatch
import Foundation
import UniformTypeIdentifiers

func writerInfoPayload(from wrapper: Writer) -> WriterInfoPayload {
    let outputPath = (wrapper.writer.value(forKey: "outputURL") as? URL)?.path
    return WriterInfoPayload(
        outputPath: outputPath,
        outputFileType: encodeFileType(wrapper.writer.outputFileType),
        availableMediaTypes: wrapper.writer.availableMediaTypes.map(encodeMediaType),
        status: Int32(wrapper.writer.status.rawValue),
        errorMessage: wrapper.writer.error?.localizedDescription,
        metadata: wrapper.writer.metadata.map(encodeMetadataItem),
        shouldOptimizeForNetworkUse: wrapper.writer.shouldOptimizeForNetworkUse,
        directoryForTemporaryFiles: wrapper.writer.directoryForTemporaryFiles?.path,
        inputs: wrapper.inputs.indices.map { Int32($0) },
        inputGroups: wrapper.inputGroups.map { group in
            InputGroupPayload(
                inputs: group.inputs.compactMap { input in
                    wrapper.inputs.firstIndex(where: { $0 === input }).map(Int32.init)
                },
                defaultInput: group.defaultInput.flatMap { input in
                    wrapper.inputs.firstIndex(where: { $0 === input }).map(Int32.init)
                }
            )
        },
        movieFragmentInterval: encodeTime(wrapper.writer.movieFragmentInterval),
        initialMovieFragmentInterval: {
            if #available(macOS 14.0, *) {
                return encodeTime(wrapper.writer.initialMovieFragmentInterval)
            }
            return TimePayload(kind: "invalid", value: nil, timescale: nil)
        }(),
        initialMovieFragmentSequenceNumber: Int64(wrapper.writer.initialMovieFragmentSequenceNumber),
        producesCombinableFragments: wrapper.writer.producesCombinableFragments,
        overallDurationHint: encodeTime(wrapper.writer.overallDurationHint),
        movieTimeScale: wrapper.writer.movieTimeScale,
        preferredOutputSegmentInterval: encodeTime(wrapper.writer.preferredOutputSegmentInterval),
        initialSegmentStartTime: encodeTime(wrapper.writer.initialSegmentStartTime),
        outputFileTypeProfile: encodeFileTypeProfile(wrapper.writer.outputFileTypeProfile)
    )
}

func inputInfoPayload(from wrapper: Writer, inputId: Int32) throws -> InputInfoPayload {
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        throw BridgeError.message("input id \(inputId) out of range")
    }
    let input = wrapper.inputs[Int(inputId)]
    let taggedSourceAttributesJson: String? = {
        if #available(macOS 14.0, *),
           let adaptor = wrapper.taggedPixelBufferGroupAdaptors[inputId] as? AVAssetWriterInputTaggedPixelBufferGroupAdaptor {
            return jsonString(fromJSONObject: adaptor.sourcePixelBufferAttributes)
        }
        return nil
    }()
    return InputInfoPayload(
        mediaType: encodeMediaType(input.mediaType),
        outputSettingsJson: jsonString(fromJSONObject: input.outputSettings),
        metadata: input.metadata.map(encodeMetadataItem),
        readyForMoreMediaData: input.isReadyForMoreMediaData,
        expectsMediaDataInRealTime: input.expectsMediaDataInRealTime,
        languageCode: input.languageCode,
        extendedLanguageTag: input.extendedLanguageTag,
        naturalSize: SizePayload(width: Double(input.naturalSize.width), height: Double(input.naturalSize.height)),
        transform: TransformPayload(
            a: Double(input.transform.a),
            b: Double(input.transform.b),
            c: Double(input.transform.c),
            d: Double(input.transform.d),
            tx: Double(input.transform.tx),
            ty: Double(input.transform.ty)
        ),
        preferredVolume: input.preferredVolume,
        marksOutputTrackAsEnabled: input.marksOutputTrackAsEnabled,
        mediaTimeScale: input.mediaTimeScale,
        preferredMediaChunkDuration: encodeTime(input.preferredMediaChunkDuration),
        preferredMediaChunkAlignment: Int64(input.preferredMediaChunkAlignment),
        sampleReferenceBaseURL: input.sampleReferenceBaseURL?.absoluteString,
        mediaDataLocation: encodeMediaDataLocation(input.mediaDataLocation),
        performsMultiPassEncodingIfSupported: input.performsMultiPassEncodingIfSupported,
        canPerformMultiplePasses: input.canPerformMultiplePasses,
        currentPassDescription: input.currentPassDescription.map { description in
            PassDescriptionPayload(
                sourceTimeRanges: description.sourceTimeRanges.map { value in
                    let range = value.timeRangeValue
                    return encodeTimeRange(range)
                }
            )
        },
        pixelBufferSourceAttributesJson: jsonString(fromJSONObject: wrapper.pixelBufferAdaptors[inputId]?.sourcePixelBufferAttributes),
        taggedPixelBufferSourceAttributesJson: taggedSourceAttributesJson,
        hasMetadataAdaptor: wrapper.metadataAdaptors[inputId] != nil,
        hasCaptionAdaptor: wrapper.captionAdaptors[inputId] != nil
    )
}

func outputSettingsAssistantInfoPayload(from assistant: AVOutputSettingsAssistant) -> OutputSettingsAssistantInfoPayload {
    OutputSettingsAssistantInfoPayload(
        audioSettingsJson: jsonString(fromJSONObject: assistant.audioSettings),
        videoSettingsJson: jsonString(fromJSONObject: assistant.videoSettings),
        outputFileType: encodeFileType(assistant.outputFileType),
        sourceVideoAverageFrameDuration: encodeTime(assistant.sourceVideoAverageFrameDuration),
        sourceVideoMinFrameDuration: encodeTime(assistant.sourceVideoMinFrameDuration)
    )
}

func exportSessionInfoPayload(from session: AVAssetExportSession) -> ExportSessionInfoPayload {
    ExportSessionInfoPayload(
        presetName: encodeExportPreset(session.presetName) ?? session.presetName,
        assetPath: (session.asset as? AVURLAsset)?.url.path,
        outputFileType: encodeFileType(session.outputFileType),
        outputPath: session.outputURL?.path,
        shouldOptimizeForNetworkUse: session.shouldOptimizeForNetworkUse,
        allowsParallelizedExport: {
            if #available(macOS 14.0, *) {
                return session.allowsParallelizedExport
            }
            return false
        }(),
        status: Int32(session.status.rawValue),
        errorMessage: session.error?.localizedDescription,
        progress: session.progress,
        supportedFileTypes: session.supportedFileTypes.compactMap(encodeFileType),
        timeRange: encodeTimeRange(session.timeRange),
        fileLengthLimit: session.fileLengthLimit,
        metadata: (session.metadata ?? []).map(encodeMetadataItem),
        canPerformMultiplePassesOverSourceMediaData: session.canPerformMultiplePassesOverSourceMediaData,
        directoryForTemporaryFiles: session.directoryForTemporaryFiles?.path,
        audioTrackGroupHandling: UInt64(session.audioTrackGroupHandling.rawValue)
    )
}

func encodeSegmentReport(_ report: AVAssetSegmentReport) -> SegmentReportPayload {
    SegmentReportPayload(
        segmentType: report.segmentType == .initialization ? "initialization" : "separable",
        trackReports: report.trackReports.map { trackReport in
            SegmentTrackReportPayload(
                trackId: trackReport.trackID,
                mediaType: encodeMediaType(trackReport.mediaType),
                earliestPresentationTimeStamp: encodeTime(trackReport.earliestPresentationTimeStamp),
                duration: encodeTime(trackReport.duration),
                firstVideoSampleInformation: trackReport.firstVideoSampleInformation.map { info in
                    SegmentReportSamplePayload(
                        presentationTimeStamp: encodeTime(info.presentationTimeStamp),
                        offset: Int64(info.offset),
                        length: Int64(info.length),
                        isSyncSample: info.isSyncSample
                    )
                }
            )
        }
    )
}

