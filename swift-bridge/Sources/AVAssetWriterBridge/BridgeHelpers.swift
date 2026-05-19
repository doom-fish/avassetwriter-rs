import AVFoundation
import CoreMedia
import CoreVideo
import Dispatch
import Foundation
import UniformTypeIdentifiers

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

func decodeOutputSettingsPreset(_ raw: String) -> AVOutputSettingsPreset? {
    switch raw {
    case "640x480": return .preset640x480
    case "960x540": return .preset960x540
    case "1280x720": return .preset1280x720
    case "1920x1080": return .preset1920x1080
    case "3840x2160": return .preset3840x2160
    case "hevc_1920x1080": return .hevc1920x1080
    case "hevc_1920x1080_with_alpha":
        if #available(macOS 10.15, *) {
            return .hevc1920x1080WithAlpha
        }
        return nil
    case "hevc_3840x2160": return .hevc3840x2160
    case "hevc_3840x2160_with_alpha":
        if #available(macOS 10.15, *) {
            return .hevc3840x2160WithAlpha
        }
        return nil
    case "hevc_4320x2160":
        if #available(macOS 26.0, *) {
            return .hevc4320x2160
        }
        return nil
    case "hevc_7680x4320": return .hevc7680x4320
    case "mvhevc_960x960":
        if #available(macOS 14.0, *) {
            return .mvhevc960x960
        }
        return nil
    case "mvhevc_1440x1440":
        if #available(macOS 14.0, *) {
            return .mvhevc1440x1440
        }
        return nil
    case "mvhevc_4320x4320":
        if #available(macOS 26.0, *) {
            return .mvhevc4320x4320
        }
        return nil
    case "mvhevc_7680x7680":
        if #available(macOS 26.0, *) {
            return .mvhevc7680x7680
        }
        return nil
    default:
        return nil
    }
}

func encodeOutputSettingsPreset(_ preset: AVOutputSettingsPreset) -> String? {
    let candidates = [
        "640x480", "960x540", "1280x720", "1920x1080", "3840x2160",
        "hevc_1920x1080", "hevc_1920x1080_with_alpha", "hevc_3840x2160",
        "hevc_3840x2160_with_alpha", "hevc_4320x2160", "hevc_7680x4320",
        "mvhevc_960x960", "mvhevc_1440x1440", "mvhevc_4320x4320", "mvhevc_7680x7680"
    ]
    for candidate in candidates {
        if let decoded = decodeOutputSettingsPreset(candidate), decoded == preset {
            return candidate
        }
    }
    return nil
}

func decodeExportPreset(_ raw: String) -> String? {
    switch raw {
    case "low_quality": return AVAssetExportPresetLowQuality
    case "medium_quality": return AVAssetExportPresetMediumQuality
    case "highest_quality": return AVAssetExportPresetHighestQuality
    case "hevc_highest_quality": return AVAssetExportPresetHEVCHighestQuality
    case "hevc_highest_quality_with_alpha": return AVAssetExportPresetHEVCHighestQualityWithAlpha
    case "640x480": return AVAssetExportPreset640x480
    case "960x540": return AVAssetExportPreset960x540
    case "1280x720": return AVAssetExportPreset1280x720
    case "1920x1080": return AVAssetExportPreset1920x1080
    case "3840x2160": return AVAssetExportPreset3840x2160
    case "hevc_1920x1080": return AVAssetExportPresetHEVC1920x1080
    case "hevc_1920x1080_with_alpha": return AVAssetExportPresetHEVC1920x1080WithAlpha
    case "hevc_3840x2160": return AVAssetExportPresetHEVC3840x2160
    case "hevc_3840x2160_with_alpha": return AVAssetExportPresetHEVC3840x2160WithAlpha
    case "hevc_4320x2160":
        if #available(macOS 26.0, *) {
            return AVAssetExportPresetHEVC4320x2160
        }
        return nil
    case "hevc_7680x4320": return AVAssetExportPresetHEVC7680x4320
    case "mvhevc_960x960":
        if #available(macOS 14.0, *) {
            return AVAssetExportPresetMVHEVC960x960
        }
        return nil
    case "mvhevc_1440x1440":
        if #available(macOS 14.0, *) {
            return AVAssetExportPresetMVHEVC1440x1440
        }
        return nil
    case "mvhevc_4320x4320":
        if #available(macOS 26.0, *) {
            return AVAssetExportPresetMVHEVC4320x4320
        }
        return nil
    case "mvhevc_7680x7680":
        if #available(macOS 26.0, *) {
            return AVAssetExportPresetMVHEVC7680x7680
        }
        return nil
    case "apple_m4a": return AVAssetExportPresetAppleM4A
    case "passthrough": return AVAssetExportPresetPassthrough
    case "apple_prores_422_lpcm": return AVAssetExportPresetAppleProRes422LPCM
    case "apple_prores_4444_lpcm": return AVAssetExportPresetAppleProRes4444LPCM
    case "apple_m4v_cellular": return AVAssetExportPresetAppleM4VCellular
    case "apple_m4v_ipod": return AVAssetExportPresetAppleM4ViPod
    case "apple_m4v_480p_sd": return AVAssetExportPresetAppleM4V480pSD
    case "apple_m4v_apple_tv": return AVAssetExportPresetAppleM4VAppleTV
    case "apple_m4v_wifi": return AVAssetExportPresetAppleM4VWiFi
    case "apple_m4v_720p_hd": return AVAssetExportPresetAppleM4V720pHD
    case "apple_m4v_1080p_hd": return AVAssetExportPresetAppleM4V1080pHD
    default:
        return nil
    }
}

func encodeExportPreset(_ preset: String) -> String? {
    let candidates = [
        "low_quality", "medium_quality", "highest_quality", "hevc_highest_quality",
        "hevc_highest_quality_with_alpha", "640x480", "960x540", "1280x720", "1920x1080",
        "3840x2160", "hevc_1920x1080", "hevc_1920x1080_with_alpha", "hevc_3840x2160",
        "hevc_3840x2160_with_alpha", "hevc_4320x2160", "hevc_7680x4320", "mvhevc_960x960",
        "mvhevc_1440x1440", "mvhevc_4320x4320", "mvhevc_7680x7680", "apple_m4a",
        "passthrough", "apple_prores_422_lpcm", "apple_prores_4444_lpcm", "apple_m4v_cellular",
        "apple_m4v_ipod", "apple_m4v_480p_sd", "apple_m4v_apple_tv", "apple_m4v_wifi",
        "apple_m4v_720p_hd", "apple_m4v_1080p_hd"
    ]
    for candidate in candidates {
        if let decoded = decodeExportPreset(candidate), decoded == preset {
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

func decodeMediaCharacteristic(_ raw: String) -> AVMediaCharacteristic {
    switch raw {
    case "visual": return .visual
    case "audible": return .audible
    case "legible": return .legible
    default: return AVMediaCharacteristic(rawValue: raw)
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

