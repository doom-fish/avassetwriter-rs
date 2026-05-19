import AVFoundation
import CoreMedia
import Foundation

private struct CompositionInfoPayload: Codable {
    let duration: TimePayload
    let naturalSize: SizePayload
    let isPlayable: Bool
    let isExportable: Bool
    let isReadable: Bool
    let isComposable: Bool
    let urlAssetInitializationOptionsJson: String?
    let trackIds: [Int32]
    let unusedTrackId: Int32
}

private struct CompositionTimeMappingPayload: Codable {
    let source: TimeRangePayload
    let target: TimeRangePayload
}

private struct AssetTrackSegmentInfoPayload: Codable {
    let timeMapping: CompositionTimeMappingPayload
    let isEmpty: Bool
}

private struct CompositionTrackSegmentInfoPayload: Codable {
    let timeMapping: CompositionTimeMappingPayload
    let isEmpty: Bool
    let sourceUrl: String?
    let sourceTrackId: Int32?
}

private struct FormatDescriptionSummaryPayload: Codable {
    let mediaTypeRaw: UInt32
    let mediaType: String
    let mediaSubtypeRaw: UInt32
    let mediaSubtype: String
}

private struct CompositionTrackFormatDescriptionReplacementInfoPayload: Codable {
    let originalFormatDescription: FormatDescriptionSummaryPayload
    let replacementFormatDescription: FormatDescriptionSummaryPayload
}

private struct CompositionTrackInfoPayload: Codable {
    let trackId: Int32
    let mediaType: String
    let isPlayable: Bool
    let isDecodable: Bool
    let isEnabled: Bool
    let isSelfContained: Bool
    let totalSampleDataLength: Int64
    let timeRange: TimeRangePayload
    let naturalTimeScale: Int32
    let estimatedDataRate: Float
    let languageCode: String?
    let extendedLanguageTag: String?
    let naturalSize: SizePayload
    let preferredTransform: TransformPayload
    let preferredVolume: Float
    let hasAudioSampleDependencies: Bool
    let nominalFrameRate: Float
    let minFrameDuration: TimePayload
    let requiresFrameReordering: Bool
    let segmentCount: Int
    let formatDescriptionCount: Int
    let formatDescriptionReplacementCount: Int
}

private func fourCharCodeString(_ code: FourCharCode) -> String {
    let bytes = [
        UInt8((code >> 24) & 0xFF),
        UInt8((code >> 16) & 0xFF),
        UInt8((code >> 8) & 0xFF),
        UInt8(code & 0xFF)
    ]
    if bytes.allSatisfy({ $0 == 32 || (33...126).contains($0) }),
       let string = String(bytes: bytes, encoding: .ascii) {
        return string
    }
    return String(format: "0x%08X", code)
}

private func compositionTimeMappingPayload(from mapping: CMTimeMapping) -> CompositionTimeMappingPayload {
    CompositionTimeMappingPayload(
        source: encodeTimeRange(mapping.source),
        target: encodeTimeRange(mapping.target)
    )
}

private func assetTrackSegmentInfoPayload(from segment: AVAssetTrackSegment) -> AssetTrackSegmentInfoPayload {
    AssetTrackSegmentInfoPayload(
        timeMapping: compositionTimeMappingPayload(from: segment.timeMapping),
        isEmpty: segment.isEmpty
    )
}

private func compositionTrackSegmentInfoPayload(
    from segment: AVCompositionTrackSegment
) -> CompositionTrackSegmentInfoPayload {
    CompositionTrackSegmentInfoPayload(
        timeMapping: compositionTimeMappingPayload(from: segment.timeMapping),
        isEmpty: segment.isEmpty,
        sourceUrl: segment.sourceURL?.absoluteString,
        sourceTrackId: segment.isEmpty ? nil : segment.sourceTrackID
    )
}

private func formatDescriptionSummaryPayload(
    from description: CMFormatDescription
) -> FormatDescriptionSummaryPayload {
    let mediaTypeRaw = CMFormatDescriptionGetMediaType(description)
    let mediaSubtypeRaw = CMFormatDescriptionGetMediaSubType(description)
    return FormatDescriptionSummaryPayload(
        mediaTypeRaw: mediaTypeRaw,
        mediaType: fourCharCodeString(mediaTypeRaw),
        mediaSubtypeRaw: mediaSubtypeRaw,
        mediaSubtype: fourCharCodeString(mediaSubtypeRaw)
    )
}

private func compositionTrackFormatDescriptionReplacementInfoPayload(
    from replacement: AVCompositionTrackFormatDescriptionReplacement
) -> CompositionTrackFormatDescriptionReplacementInfoPayload {
    CompositionTrackFormatDescriptionReplacementInfoPayload(
        originalFormatDescription: formatDescriptionSummaryPayload(
            from: replacement.originalFormatDescription
        ),
        replacementFormatDescription: formatDescriptionSummaryPayload(
            from: replacement.replacementFormatDescription
        )
    )
}

private func compositionTrackInfoPayload(from track: AVCompositionTrack) -> CompositionTrackInfoPayload {
    CompositionTrackInfoPayload(
        trackId: track.trackID,
        mediaType: encodeMediaType(track.mediaType),
        isPlayable: track.isPlayable,
        isDecodable: track.isDecodable,
        isEnabled: track.isEnabled,
        isSelfContained: track.isSelfContained,
        totalSampleDataLength: Int64(track.totalSampleDataLength),
        timeRange: encodeTimeRange(track.timeRange),
        naturalTimeScale: track.naturalTimeScale,
        estimatedDataRate: track.estimatedDataRate,
        languageCode: track.languageCode,
        extendedLanguageTag: track.extendedLanguageTag,
        naturalSize: SizePayload(
            width: Double(track.naturalSize.width),
            height: Double(track.naturalSize.height)
        ),
        preferredTransform: TransformPayload(
            a: Double(track.preferredTransform.a),
            b: Double(track.preferredTransform.b),
            c: Double(track.preferredTransform.c),
            d: Double(track.preferredTransform.d),
            tx: Double(track.preferredTransform.tx),
            ty: Double(track.preferredTransform.ty)
        ),
        preferredVolume: track.preferredVolume,
        hasAudioSampleDependencies: track.hasAudioSampleDependencies,
        nominalFrameRate: track.nominalFrameRate,
        minFrameDuration: encodeTime(track.minFrameDuration),
        requiresFrameReordering: track.requiresFrameReordering,
        segmentCount: track.segments.count,
        formatDescriptionCount: track.formatDescriptions.count,
        formatDescriptionReplacementCount: track.formatDescriptionReplacements.count
    )
}

private func compositionInfoPayload(from composition: AVComposition) -> CompositionInfoPayload {
    CompositionInfoPayload(
        duration: encodeTime(composition.duration),
        naturalSize: SizePayload(
            width: Double(composition.naturalSize.width),
            height: Double(composition.naturalSize.height)
        ),
        isPlayable: composition.isPlayable,
        isExportable: composition.isExportable,
        isReadable: composition.isReadable,
        isComposable: composition.isComposable,
        urlAssetInitializationOptionsJson: jsonString(
            fromJSONObject: composition.urlAssetInitializationOptions
        ),
        trackIds: composition.tracks.map(\.trackID),
        unusedTrackId: composition.unusedTrackID()
    )
}

@_cdecl("av_composition_create_empty")
public func av_composition_create_empty() -> UnsafeMutableRawPointer? {
    let snapshot = AVMutableComposition().copy() as! AVComposition
    return Unmanaged.passRetained(snapshot).toOpaque()
}

@_cdecl("av_composition_create_from_asset")
public func av_composition_create_from_asset(
    _ assetPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    let asset = Unmanaged<AVAsset>.fromOpaque(assetPtr).takeUnretainedValue()
    let composition = AVMutableComposition()
    do {
        try composition.insertTimeRange(
            CMTimeRange(start: .zero, duration: asset.duration),
            of: asset,
            at: .zero
        )
        let snapshot = composition.copy() as! AVComposition
        return Unmanaged.passRetained(snapshot).toOpaque()
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_composition_release")
public func av_composition_release(_ compositionPtr: UnsafeMutableRawPointer?) {
    guard let compositionPtr else { return }
    Unmanaged<AVComposition>.fromOpaque(compositionPtr).release()
}

@_cdecl("av_composition_info_json")
public func av_composition_info_json(
    _ compositionPtr: UnsafeMutableRawPointer
) -> UnsafeMutablePointer<CChar>? {
    let composition = Unmanaged<AVComposition>.fromOpaque(compositionPtr).takeUnretainedValue()
    do {
        return ffiString(try encodeJson(compositionInfoPayload(from: composition)))
    } catch {
        return nil
    }
}

@_cdecl("av_composition_track_count")
public func av_composition_track_count(_ compositionPtr: UnsafeMutableRawPointer) -> Int {
    let composition = Unmanaged<AVComposition>.fromOpaque(compositionPtr).takeUnretainedValue()
    return composition.tracks.count
}

@_cdecl("av_composition_copy_track_at_index")
public func av_composition_copy_track_at_index(
    _ compositionPtr: UnsafeMutableRawPointer,
    _ index: Int
) -> UnsafeMutableRawPointer? {
    let composition = Unmanaged<AVComposition>.fromOpaque(compositionPtr).takeUnretainedValue()
    guard index >= 0, index < composition.tracks.count else { return nil }
    return Unmanaged.passRetained(composition.tracks[index]).toOpaque()
}

@_cdecl("av_composition_track_release")
public func av_composition_track_release(_ trackPtr: UnsafeMutableRawPointer?) {
    guard let trackPtr else { return }
    Unmanaged<AVCompositionTrack>.fromOpaque(trackPtr).release()
}

@_cdecl("av_composition_track_info_json")
public func av_composition_track_info_json(
    _ trackPtr: UnsafeMutableRawPointer
) -> UnsafeMutablePointer<CChar>? {
    let track = Unmanaged<AVCompositionTrack>.fromOpaque(trackPtr).takeUnretainedValue()
    do {
        return ffiString(try encodeJson(compositionTrackInfoPayload(from: track)))
    } catch {
        return nil
    }
}

@_cdecl("av_composition_track_segment_count")
public func av_composition_track_segment_count(_ trackPtr: UnsafeMutableRawPointer) -> Int {
    let track = Unmanaged<AVCompositionTrack>.fromOpaque(trackPtr).takeUnretainedValue()
    return track.segments.count
}

@_cdecl("av_composition_track_copy_segment_at_index")
public func av_composition_track_copy_segment_at_index(
    _ trackPtr: UnsafeMutableRawPointer,
    _ index: Int
) -> UnsafeMutableRawPointer? {
    let track = Unmanaged<AVCompositionTrack>.fromOpaque(trackPtr).takeUnretainedValue()
    guard index >= 0, index < track.segments.count else { return nil }
    return Unmanaged.passRetained(track.segments[index]).toOpaque()
}

@_cdecl("av_composition_track_segment_for_track_time")
public func av_composition_track_segment_for_track_time(
    _ trackPtr: UnsafeMutableRawPointer,
    _ timeValue: Int64,
    _ timeScale: Int32,
    _ timeKind: Int32
) -> UnsafeMutableRawPointer? {
    let track = Unmanaged<AVCompositionTrack>.fromOpaque(trackPtr).takeUnretainedValue()
    guard let segment = track.segment(forTrackTime: cmTime(value: timeValue, timescale: timeScale, kind: timeKind)) else {
        return nil
    }
    return Unmanaged.passRetained(segment).toOpaque()
}

@_cdecl("av_composition_track_has_media_characteristic")
public func av_composition_track_has_media_characteristic(
    _ trackPtr: UnsafeMutableRawPointer,
    _ mediaCharacteristicPtr: UnsafePointer<CChar>
) -> Bool {
    let track = Unmanaged<AVCompositionTrack>.fromOpaque(trackPtr).takeUnretainedValue()
    let characteristic = decodeMediaCharacteristic(String(cString: mediaCharacteristicPtr))
    return track.hasMediaCharacteristic(characteristic)
}

@_cdecl("av_composition_track_sample_presentation_time_json")
public func av_composition_track_sample_presentation_time_json(
    _ trackPtr: UnsafeMutableRawPointer,
    _ timeValue: Int64,
    _ timeScale: Int32,
    _ timeKind: Int32
) -> UnsafeMutablePointer<CChar>? {
    let track = Unmanaged<AVCompositionTrack>.fromOpaque(trackPtr).takeUnretainedValue()
    do {
        return ffiString(try encodeJson(encodeTime(
            track.samplePresentationTime(
                forTrackTime: cmTime(value: timeValue, timescale: timeScale, kind: timeKind)
            )
        )))
    } catch {
        return nil
    }
}

@_cdecl("av_composition_track_format_description_count")
public func av_composition_track_format_description_count(
    _ trackPtr: UnsafeMutableRawPointer
) -> Int {
    let track = Unmanaged<AVCompositionTrack>.fromOpaque(trackPtr).takeUnretainedValue()
    return track.formatDescriptions.count
}

@_cdecl("av_composition_track_copy_format_description_at_index")
public func av_composition_track_copy_format_description_at_index(
    _ trackPtr: UnsafeMutableRawPointer,
    _ index: Int
) -> UnsafeMutableRawPointer? {
    let track = Unmanaged<AVCompositionTrack>.fromOpaque(trackPtr).takeUnretainedValue()
    guard index >= 0, index < track.formatDescriptions.count else {
        return nil
    }
    let description = track.formatDescriptions[index] as! CMFormatDescription
    return Unmanaged.passRetained(description).toOpaque()
}

@_cdecl("av_composition_track_format_description_replacement_count")
public func av_composition_track_format_description_replacement_count(
    _ trackPtr: UnsafeMutableRawPointer
) -> Int {
    let track = Unmanaged<AVCompositionTrack>.fromOpaque(trackPtr).takeUnretainedValue()
    return track.formatDescriptionReplacements.count
}

@_cdecl("av_composition_track_copy_format_description_replacement_at_index")
public func av_composition_track_copy_format_description_replacement_at_index(
    _ trackPtr: UnsafeMutableRawPointer,
    _ index: Int
) -> UnsafeMutableRawPointer? {
    let track = Unmanaged<AVCompositionTrack>.fromOpaque(trackPtr).takeUnretainedValue()
    guard index >= 0, index < track.formatDescriptionReplacements.count else { return nil }
    return Unmanaged.passRetained(track.formatDescriptionReplacements[index]).toOpaque()
}

@_cdecl("av_composition_track_segment_create_url")
public func av_composition_track_segment_create_url(
    _ urlPtr: UnsafePointer<CChar>,
    _ isFileURL: Bool,
    _ trackId: Int32,
    _ sourceStartValue: Int64,
    _ sourceStartScale: Int32,
    _ sourceStartKind: Int32,
    _ sourceDurationValue: Int64,
    _ sourceDurationScale: Int32,
    _ sourceDurationKind: Int32,
    _ targetStartValue: Int64,
    _ targetStartScale: Int32,
    _ targetStartKind: Int32,
    _ targetDurationValue: Int64,
    _ targetDurationScale: Int32,
    _ targetDurationKind: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    let urlString = String(cString: urlPtr)
    let url = isFileURL ? URL(fileURLWithPath: urlString) : URL(string: urlString)
    guard let url else {
        outErrorMessage?.pointee = ffiString("invalid URL: \(urlString)")
        return nil
    }
    let sourceRange = CMTimeRange(
        start: cmTime(value: sourceStartValue, timescale: sourceStartScale, kind: sourceStartKind),
        duration: cmTime(
            value: sourceDurationValue,
            timescale: sourceDurationScale,
            kind: sourceDurationKind
        )
    )
    let targetRange = CMTimeRange(
        start: cmTime(value: targetStartValue, timescale: targetStartScale, kind: targetStartKind),
        duration: cmTime(
            value: targetDurationValue,
            timescale: targetDurationScale,
            kind: targetDurationKind
        )
    )
    let segment = AVCompositionTrackSegment(
        url: url,
        trackID: trackId,
        sourceTimeRange: sourceRange,
        targetTimeRange: targetRange
    )
    return Unmanaged.passRetained(segment).toOpaque()
}

@_cdecl("av_composition_track_segment_create_empty")
public func av_composition_track_segment_create_empty(
    _ startValue: Int64,
    _ startScale: Int32,
    _ startKind: Int32,
    _ durationValue: Int64,
    _ durationScale: Int32,
    _ durationKind: Int32
) -> UnsafeMutableRawPointer? {
    let timeRange = CMTimeRange(
        start: cmTime(value: startValue, timescale: startScale, kind: startKind),
        duration: cmTime(value: durationValue, timescale: durationScale, kind: durationKind)
    )
    return Unmanaged.passRetained(AVCompositionTrackSegment(timeRange: timeRange)).toOpaque()
}

@_cdecl("av_composition_track_segment_release")
public func av_composition_track_segment_release(_ segmentPtr: UnsafeMutableRawPointer?) {
    guard let segmentPtr else { return }
    Unmanaged<AVCompositionTrackSegment>.fromOpaque(segmentPtr).release()
}

@_cdecl("av_composition_track_segment_info_json")
public func av_composition_track_segment_info_json(
    _ segmentPtr: UnsafeMutableRawPointer
) -> UnsafeMutablePointer<CChar>? {
    let segment = Unmanaged<AVCompositionTrackSegment>.fromOpaque(segmentPtr).takeUnretainedValue()
    do {
        return ffiString(try encodeJson(compositionTrackSegmentInfoPayload(from: segment)))
    } catch {
        return nil
    }
}

@_cdecl("av_composition_track_segment_asset_track_segment")
public func av_composition_track_segment_asset_track_segment(
    _ segmentPtr: UnsafeMutableRawPointer
) -> UnsafeMutableRawPointer? {
    let segment = Unmanaged<AVCompositionTrackSegment>.fromOpaque(segmentPtr).takeUnretainedValue()
    return Unmanaged.passRetained(segment as AVAssetTrackSegment).toOpaque()
}

@_cdecl("av_asset_track_segment_release")
public func av_asset_track_segment_release(_ segmentPtr: UnsafeMutableRawPointer?) {
    guard let segmentPtr else { return }
    Unmanaged<AVAssetTrackSegment>.fromOpaque(segmentPtr).release()
}

@_cdecl("av_asset_track_segment_info_json")
public func av_asset_track_segment_info_json(
    _ segmentPtr: UnsafeMutableRawPointer
) -> UnsafeMutablePointer<CChar>? {
    let segment = Unmanaged<AVAssetTrackSegment>.fromOpaque(segmentPtr).takeUnretainedValue()
    do {
        return ffiString(try encodeJson(assetTrackSegmentInfoPayload(from: segment)))
    } catch {
        return nil
    }
}

@_cdecl("av_composition_track_format_description_replacement_release")
public func av_composition_track_format_description_replacement_release(
    _ replacementPtr: UnsafeMutableRawPointer?
) {
    guard let replacementPtr else { return }
    Unmanaged<AVCompositionTrackFormatDescriptionReplacement>.fromOpaque(replacementPtr).release()
}

@_cdecl("av_composition_track_format_description_replacement_info_json")
public func av_composition_track_format_description_replacement_info_json(
    _ replacementPtr: UnsafeMutableRawPointer
) -> UnsafeMutablePointer<CChar>? {
    let replacement = Unmanaged<AVCompositionTrackFormatDescriptionReplacement>
        .fromOpaque(replacementPtr)
        .takeUnretainedValue()
    do {
        return ffiString(
            try encodeJson(
                compositionTrackFormatDescriptionReplacementInfoPayload(from: replacement)
            )
        )
    } catch {
        return nil
    }
}

@_cdecl("av_composition_track_format_description_replacement_original_format_description")
public func av_composition_track_format_description_replacement_original_format_description(
    _ replacementPtr: UnsafeMutableRawPointer
) -> UnsafeMutableRawPointer? {
    let replacement = Unmanaged<AVCompositionTrackFormatDescriptionReplacement>
        .fromOpaque(replacementPtr)
        .takeUnretainedValue()
    return Unmanaged.passRetained(replacement.originalFormatDescription).toOpaque()
}

@_cdecl("av_composition_track_format_description_replacement_replacement_format_description")
public func av_composition_track_format_description_replacement_replacement_format_description(
    _ replacementPtr: UnsafeMutableRawPointer
) -> UnsafeMutableRawPointer? {
    let replacement = Unmanaged<AVCompositionTrackFormatDescriptionReplacement>
        .fromOpaque(replacementPtr)
        .takeUnretainedValue()
    return Unmanaged.passRetained(replacement.replacementFormatDescription).toOpaque()
}
