import AVFoundation
import CoreMedia
import CoreVideo
import Foundation

public typealias AVWReadyCallback = @convention(c) (UnsafeMutableRawPointer?) -> Void

public typealias AVWPassDescriptionCallback = @convention(c) (
    UnsafePointer<CChar>?,
    UnsafeMutableRawPointer?
) -> Void

public typealias AVWSegmentCallback = @convention(c) (
    UnsafePointer<UInt8>?,
    Int,
    Int32,
    UnsafePointer<CChar>?,
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

final class InputCallbackBox {
    let userdata: UnsafeMutableRawPointer?
    let dropUserdata: AVWDropCallback?
    private var disposed = false

    init(userdata: UnsafeMutableRawPointer?, dropUserdata: AVWDropCallback?) {
        self.userdata = userdata
        self.dropUserdata = dropUserdata
    }

    func dispose() {
        guard !disposed else { return }
        disposed = true
        if let userdata, let dropUserdata {
            dropUserdata(userdata)
        }
    }
}

final class SegmentCallbackBox {
    let callback: AVWSegmentCallback?
    let userdata: UnsafeMutableRawPointer?
    let dropUserdata: AVWDropCallback?
    private var disposed = false

    init(
        callback: AVWSegmentCallback?,
        userdata: UnsafeMutableRawPointer?,
        dropUserdata: AVWDropCallback?
    ) {
        self.callback = callback
        self.userdata = userdata
        self.dropUserdata = dropUserdata
    }

    func emit(data: Data, segmentType: Int32, report: SegmentReportPayload?) {
        guard let callback else { return }
        let reportCString = report.flatMap { payload in
            (try? encodeJson(payload)).flatMap(ffiString)
        }
        data.withUnsafeBytes { bytes in
            callback(
                bytes.bindMemory(to: UInt8.self).baseAddress,
                data.count,
                segmentType,
                reportCString,
                userdata
            )
        }
        if let reportCString {
            free(reportCString)
        }
    }

    func dispose() {
        guard !disposed else { return }
        disposed = true
        if let userdata, let dropUserdata {
            dropUserdata(userdata)
        }
    }
}

struct TimePayload: Codable {
    let kind: String
    let value: Int64?
    let timescale: Int32?
}

struct TimeRangePayload: Codable {
    let start: TimePayload
    let duration: TimePayload
}

struct MetadataValuePayload: Codable {
    let kind: String
    let stringValue: String?
    let integerValue: Int64?
    let floatValue: Double?
    let booleanValue: Bool?
    let dataValue: [UInt8]?

    enum CodingKeys: String, CodingKey {
        case kind
        case value
    }

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

    init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        kind = try container.decode(String.self, forKey: .kind)
        switch kind {
        case "string":
            stringValue = try container.decode(String.self, forKey: .value)
            integerValue = nil
            floatValue = nil
            booleanValue = nil
            dataValue = nil
        case "integer":
            stringValue = nil
            integerValue = try container.decode(Int64.self, forKey: .value)
            floatValue = nil
            booleanValue = nil
            dataValue = nil
        case "float":
            stringValue = nil
            integerValue = nil
            floatValue = try container.decode(Double.self, forKey: .value)
            booleanValue = nil
            dataValue = nil
        case "boolean":
            stringValue = nil
            integerValue = nil
            floatValue = nil
            booleanValue = try container.decode(Bool.self, forKey: .value)
            dataValue = nil
        case "data":
            stringValue = nil
            integerValue = nil
            floatValue = nil
            booleanValue = nil
            dataValue = try container.decode([UInt8].self, forKey: .value)
        default:
            throw BridgeError.message("unsupported metadata value kind: \(kind)")
        }
    }

    func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(kind, forKey: .kind)
        switch kind {
        case "string":
            try container.encode(stringValue, forKey: .value)
        case "integer":
            try container.encode(integerValue, forKey: .value)
        case "float":
            try container.encode(floatValue, forKey: .value)
        case "boolean":
            try container.encode(booleanValue, forKey: .value)
        case "data":
            try container.encode(dataValue, forKey: .value)
        default:
            throw BridgeError.message("unsupported metadata value kind: \(kind)")
        }
    }
}

struct MetadataItemPayload: Codable {
    let identifier: String
    let value: MetadataValuePayload
    let dataType: String?
    let extendedLanguageTag: String?
    let localeIdentifier: String?
}

struct InputGroupPayload: Codable {
    let inputs: [Int32]
    let defaultInput: Int32?
}

struct PassDescriptionPayload: Codable {
    let sourceTimeRanges: [TimeRangePayload]
}

struct SegmentReportSamplePayload: Codable {
    let presentationTimeStamp: TimePayload
    let offset: Int64
    let length: Int64
    let isSyncSample: Bool
}

struct SegmentTrackReportPayload: Codable {
    let trackId: Int32
    let mediaType: String
    let earliestPresentationTimeStamp: TimePayload
    let duration: TimePayload
    let firstVideoSampleInformation: SegmentReportSamplePayload?
}

struct SegmentReportPayload: Codable {
    let segmentType: String
    let trackReports: [SegmentTrackReportPayload]
}

struct SizePayload: Codable {
    let width: Double
    let height: Double
}

struct TransformPayload: Codable {
    let a: Double
    let b: Double
    let c: Double
    let d: Double
    let tx: Double
    let ty: Double
}

struct WriterInfoPayload: Codable {
    let outputPath: String?
    let outputFileType: String?
    let availableMediaTypes: [String]
    let status: Int32
    let errorMessage: String?
    let metadata: [MetadataItemPayload]
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

struct InputInfoPayload: Codable {
    let mediaType: String
    let outputSettingsJson: String?
    let metadata: [MetadataItemPayload]
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
    let pixelBufferSourceAttributesJson: String?
    let taggedPixelBufferSourceAttributesJson: String?
    let hasMetadataAdaptor: Bool
    let hasCaptionAdaptor: Bool
}

struct CaptionPayload: Codable {
    let text: String
    let timeRange: TimeRangePayload
}

struct CaptionGroupPayload: Codable {
    let captions: [CaptionPayload]
    let timeRange: TimeRangePayload
}

struct TimedMetadataGroupPayload: Codable {
    let items: [MetadataItemPayload]
    let timeRange: TimeRangePayload
}

struct MetadataSpecificationPayload: Codable {
    let identifier: String
    let dataType: String
    let extendedLanguageTag: String?
}

struct OutputSettingsAssistantInfoPayload: Codable {
    let audioSettingsJson: String?
    let videoSettingsJson: String?
    let outputFileType: String?
    let sourceVideoAverageFrameDuration: TimePayload
    let sourceVideoMinFrameDuration: TimePayload
}

struct ExportSessionInfoPayload: Codable {
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
    let metadata: [MetadataItemPayload]
    let canPerformMultiplePassesOverSourceMediaData: Bool
    let directoryForTemporaryFiles: String?
    let audioTrackGroupHandling: UInt64
}

struct AudioMixInfoPayload: Codable {
    let inputParameterCount: Int
}

struct VideoCompositionInfoPayload: Codable {
    let frameDuration: TimePayload
    let renderSize: SizePayload
    let renderScale: Float
    let instructionCount: Int
    let customVideoCompositorClassName: String?
}

struct VideoCompositorInfoPayload: Codable {
    let className: String
    let sourcePixelBufferAttributesJson: String?
    let requiredPixelBufferAttributesForRenderContextJson: String?
    let supportsWideColorSourceFrames: Bool
    let supportsHDRSourceFrames: Bool
}

