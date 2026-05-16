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

func encodeJson<T: Encodable>(_ value: T) throws -> String {
    let data = try JSONEncoder().encode(value)
    guard let string = String(data: data, encoding: .utf8) else {
        throw BridgeError.message("failed to UTF-8 encode JSON payload")
    }
    return string
}

func decodeJson<T: Decodable>(_ ptr: UnsafePointer<CChar>?, as type: T.Type) throws -> T {
    guard let ptr else {
        throw BridgeError.message("missing JSON payload")
    }
    let string = String(cString: ptr)
    guard let data = string.data(using: .utf8) else {
        throw BridgeError.message("payload was not valid UTF-8")
    }
    return try JSONDecoder().decode(T.self, from: data)
}

func jsonObjectFromCString(_ ptr: UnsafePointer<CChar>?) throws -> Any? {
    guard let ptr else { return nil }
    let string = String(cString: ptr)
    guard let data = string.data(using: .utf8) else {
        throw BridgeError.message("payload was not valid UTF-8")
    }
    return try JSONSerialization.jsonObject(with: data)
}

func jsonString(fromJSONObject object: Any?) -> String? {
    guard let object else { return nil }
    guard JSONSerialization.isValidJSONObject(object) else { return nil }
    guard let data = try? JSONSerialization.data(withJSONObject: object) else { return nil }
    return String(data: data, encoding: .utf8)
}

func decodeFileType(_ raw: String) -> AVFileType? {
    switch raw {
    case "mov": return AVFileType.mov
    case "mp4": return AVFileType.mp4
    case "m4v": return AVFileType.m4v
    case "m4a": return AVFileType.m4a
    case "3gpp": return AVFileType.mobile3GPP
    case "3gpp2": return AVFileType.mobile3GPP2
    case "caf": return AVFileType.caf
    case "wav": return AVFileType.wav
    case "aiff": return AVFileType.aiff
    case "aifc": return AVFileType.aifc
    case "amr": return AVFileType.amr
    case "mp3": return AVFileType.mp3
    case "au": return AVFileType.au
    case "ac3": return AVFileType.ac3
    case "eac3": return AVFileType.eac3
    case "jpeg": return AVFileType.jpg
    case "dng": return AVFileType.dng
    case "heic": return AVFileType.heic
    case "avci": return AVFileType.avci
    case "heif": return AVFileType.heif
    case "tiff": return AVFileType.tif
    case "itt": return AVFileType.appleiTT
    case "scc": return AVFileType.SCC
    case "ahap":
        if #available(macOS 14.0, *) {
            return AVFileType.AHAP
        }
        return nil
    case "qt_audio":
        if #available(macOS 26.0, *) {
            return AVFileType.qta
        }
        return nil
    case "dicom":
        if #available(macOS 26.0, *) {
            return AVFileType.dcm
        }
        return nil
    default:
        return nil
    }
}

func encodeFileType(_ fileType: AVFileType?) -> String? {
    guard let fileType else { return nil }
    let candidates = [
        "mov", "mp4", "m4v", "m4a", "3gpp", "3gpp2", "caf", "wav", "aiff", "aifc",
        "amr", "mp3", "au", "ac3", "eac3", "jpeg", "dng", "heic", "avci", "heif",
        "tiff", "itt", "scc", "ahap", "qt_audio", "dicom"
    ]
    for candidate in candidates {
        if let decoded = decodeFileType(candidate), decoded == fileType {
            return candidate
        }
    }
    return nil
}

func decodeFileTypeProfile(_ raw: String?) -> AVFileTypeProfile? {
    guard let raw else { return nil }
    switch raw {
    case "apple_hls": return .mpeg4AppleHLS
    case "cmaf_compliant": return .mpeg4CMAFCompliant
    default: return AVFileTypeProfile(rawValue: raw)
    }
}

func encodeFileTypeProfile(_ profile: AVFileTypeProfile?) -> String? {
    guard let profile else { return nil }
    if profile == .mpeg4AppleHLS { return "apple_hls" }
    if profile == .mpeg4CMAFCompliant { return "cmaf_compliant" }
    return profile.rawValue
}

func decodeMediaType(_ raw: String) -> AVMediaType? {
    switch raw {
    case "video": return .video
    case "audio": return .audio
    case "text": return .text
    case "closed_caption": return .closedCaption
    case "subtitle": return .subtitle
    case "timecode": return .timecode
    case "metadata": return .metadata
    case "muxed": return .muxed
    case "haptic": return .haptic
    case "depth_data": return .depthData
    case "auxiliary_picture": return .auxiliaryPicture
    default: return AVMediaType(rawValue: raw)
    }
}

func encodeMediaType(_ mediaType: AVMediaType) -> String {
    switch mediaType {
    case .video: return "video"
    case .audio: return "audio"
    case .text: return "text"
    case .closedCaption: return "closed_caption"
    case .subtitle: return "subtitle"
    case .timecode: return "timecode"
    case .metadata: return "metadata"
    case .muxed: return "muxed"
    case .haptic: return "haptic"
    case .depthData: return "depth_data"
    case .auxiliaryPicture: return "auxiliary_picture"
    default: return mediaType.rawValue
    }
}

func decodeTrackAssociationType(_ raw: String) -> String {
    switch raw {
    case "audio_fallback": return AVAssetTrack.AssociationType.audioFallback.rawValue
    case "chapter_list": return AVAssetTrack.AssociationType.chapterList.rawValue
    case "forced_subtitles_only": return AVAssetTrack.AssociationType.forcedSubtitlesOnly.rawValue
    case "selection_follower": return AVAssetTrack.AssociationType.selectionFollower.rawValue
    case "timecode": return AVAssetTrack.AssociationType.timecode.rawValue
    case "metadata_referent": return AVAssetTrack.AssociationType.metadataReferent.rawValue
    case "render_metadata_source":
        if #available(macOS 26.0, *) {
            return AVAssetTrack.AssociationType.renderMetadataSource.rawValue
        }
        return raw
    default:
        return raw
    }
}

func decodeMediaDataLocation(_ raw: String) -> AVAssetWriterInput.MediaDataLocation? {
    switch raw {
    case "interleaved":
        return .interleavedWithMainMediaData
    case "before_main_not_interleaved":
        return .beforeMainMediaDataNotInterleaved
    case "sparse_interleaved":
        if #available(macOS 26.0, *) {
            return .sparselyInterleavedWithMainMediaData
        }
        return nil
    default:
        return AVAssetWriterInput.MediaDataLocation(rawValue: raw)
    }
}

func encodeMediaDataLocation(_ location: AVAssetWriterInput.MediaDataLocation) -> String {
    if location == .interleavedWithMainMediaData { return "interleaved" }
    if location == .beforeMainMediaDataNotInterleaved { return "before_main_not_interleaved" }
    if #available(macOS 26.0, *), location == .sparselyInterleavedWithMainMediaData {
        return "sparse_interleaved"
    }
    return location.rawValue
}

func cmTime(from payload: TimePayload) -> CMTime {
    switch payload.kind {
    case "numeric":
        return CMTime(value: payload.value ?? 0, timescale: payload.timescale ?? 1)
    case "invalid":
        return .invalid
    case "indefinite":
        return .indefinite
    case "positive_infinity":
        return .positiveInfinity
    case "negative_infinity":
        return .negativeInfinity
    default:
        return .invalid
    }
}

func cmTime(value: Int64, timescale: Int32, kind: Int32) -> CMTime {
    switch kind {
    case 0:
        return CMTime(value: value, timescale: timescale)
    case 1:
        return .invalid
    case 2:
        return .indefinite
    case 3:
        return .positiveInfinity
    case 4:
        return .negativeInfinity
    default:
        return .invalid
    }
}

func encodeTime(_ time: CMTime) -> TimePayload {
    if time == .invalid {
        return TimePayload(kind: "invalid", value: nil, timescale: nil)
    }
    if time == .indefinite {
        return TimePayload(kind: "indefinite", value: nil, timescale: nil)
    }
    if time == .positiveInfinity {
        return TimePayload(kind: "positive_infinity", value: nil, timescale: nil)
    }
    if time == .negativeInfinity {
        return TimePayload(kind: "negative_infinity", value: nil, timescale: nil)
    }
    return TimePayload(kind: "numeric", value: time.value, timescale: time.timescale)
}

func cmTimeRange(from payload: TimeRangePayload) -> CMTimeRange {
    CMTimeRange(start: cmTime(from: payload.start), duration: cmTime(from: payload.duration))
}

func encodeTimeRange(_ range: CMTimeRange) -> TimeRangePayload {
    TimeRangePayload(start: encodeTime(range.start), duration: encodeTime(range.duration))
}

func metadataValueObject(from payload: MetadataValuePayload) throws -> NSCopying & NSObjectProtocol {
    switch payload.kind {
    case "string":
        return (payload.stringValue ?? "") as NSString
    case "integer":
        return NSNumber(value: payload.integerValue ?? 0)
    case "float":
        return NSNumber(value: payload.floatValue ?? 0)
    case "boolean":
        return NSNumber(value: payload.booleanValue ?? false)
    case "data":
        return Data(payload.dataValue ?? []) as NSData
    default:
        throw BridgeError.message("unsupported metadata value kind: \(payload.kind)")
    }
}

func metadataValuePayload(from value: Any?) -> MetadataValuePayload {
    switch value {
    case let string as String:
        return MetadataValuePayload(kind: "string", stringValue: string)
    case let number as NSNumber:
        let type = String(cString: number.objCType)
        if type == "c" || type == "B" {
            return MetadataValuePayload(kind: "boolean", booleanValue: number.boolValue)
        }
        if type.contains("f") || type.contains("d") {
            return MetadataValuePayload(kind: "float", floatValue: number.doubleValue)
        }
        return MetadataValuePayload(kind: "integer", integerValue: number.int64Value)
    case let data as Data:
        return MetadataValuePayload(kind: "data", dataValue: Array(data))
    case let data as NSData:
        return MetadataValuePayload(kind: "data", dataValue: Array(data as Data))
    default:
        return MetadataValuePayload(kind: "string", stringValue: String(describing: value ?? ""))
    }
}

func avMetadataItem(from payload: MetadataItemPayload) throws -> AVMetadataItem {
    let item = AVMutableMetadataItem()
    item.identifier = AVMetadataIdentifier(rawValue: payload.identifier)
    item.value = try metadataValueObject(from: payload.value)
    if let dataType = payload.dataType {
        item.dataType = dataType
    }
    item.extendedLanguageTag = payload.extendedLanguageTag
    if let localeIdentifier = payload.localeIdentifier {
        item.locale = Locale(identifier: localeIdentifier)
    }
    return item
}

func encodeMetadataItem(_ item: AVMetadataItem) -> MetadataItemPayload {
    MetadataItemPayload(
        identifier: item.identifier?.rawValue ?? "",
        value: metadataValuePayload(from: item.value),
        dataType: item.dataType,
        extendedLanguageTag: item.extendedLanguageTag,
        localeIdentifier: item.locale?.identifier
    )
}

func timedMetadataGroup(from payload: TimedMetadataGroupPayload) throws -> AVTimedMetadataGroup {
    let items = try payload.items.map(avMetadataItem)
    return AVTimedMetadataGroup(items: items, timeRange: cmTimeRange(from: payload.timeRange))
}

func caption(from payload: CaptionPayload) -> AVCaption {
    AVCaption(payload.text, timeRange: cmTimeRange(from: payload.timeRange))
}

func captionGroup(from payload: CaptionGroupPayload) -> AVCaptionGroup {
    AVCaptionGroup(captions: payload.captions.map(caption(from:)), timeRange: cmTimeRange(from: payload.timeRange))
}

func metadataFormatDescription(from specs: [MetadataSpecificationPayload]) throws -> CMFormatDescription {
    let dictionaries: [[CFString: Any]] = specs.map { spec in
        var dict: [CFString: Any] = [
            kCMMetadataFormatDescriptionMetadataSpecificationKey_Identifier: spec.identifier as CFString,
            kCMMetadataFormatDescriptionMetadataSpecificationKey_DataType: spec.dataType as CFString,
        ]
        if let extendedLanguageTag = spec.extendedLanguageTag {
            dict[kCMMetadataFormatDescriptionMetadataSpecificationKey_ExtendedLanguageTag] = extendedLanguageTag as CFString
        }
        return dict
    }
    var formatDescription: CMFormatDescription?
    let status = CMMetadataFormatDescriptionCreateWithMetadataSpecifications(
        allocator: kCFAllocatorDefault,
        metadataType: kCMMetadataFormatType_Boxed,
        metadataSpecifications: dictionaries as CFArray,
        formatDescriptionOut: &formatDescription
    )
    guard status == noErr, let formatDescription else {
        throw BridgeError.message("CMMetadataFormatDescriptionCreateWithMetadataSpecifications failed: \(status)")
    }
    return formatDescription
}

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

