import AVFoundation
import CoreImage
import CoreMedia
import CoreVideo
import Foundation

private func mediaProcessingAsset(from pathPtr: UnsafePointer<CChar>) -> AVURLAsset {
    AVURLAsset(url: URL(fileURLWithPath: String(cString: pathPtr)))
}

private func eraseSendableDictionary(_ dictionary: [String: any Sendable]?) -> [String: Any]? {
    dictionary.map { value in
        Dictionary(uniqueKeysWithValues: value.map { ($0.key, $0.value) })
    }
}

private func metadataItemFilterKind(_ filter: AVMetadataItemFilter) -> String {
    if filter.isEqual(AVMetadataItemFilter.forSharing()) {
        return "sharing"
    }
    return NSStringFromClass(type(of: filter))
}

private func decodeVideoCompositorClass(_ raw: String) -> AVVideoCompositing.Type? {
    switch raw {
    case "passthrough":
        return AVWRPassthroughVideoCompositor.self
    default:
        return nil
    }
}

private func encodeVideoCompositorClassName(_ compositorClass: AVVideoCompositing.Type?) -> String? {
    compositorClass.map(NSStringFromClass)
}

private struct AudioMixInputParametersInfoPayload: Codable {
    let trackId: Int32
    let audioTimePitchAlgorithm: String?
}

private struct AudioVolumeRampPayload: Codable {
    let startVolume: Float
    let endVolume: Float
    let timeRange: TimeRangePayload
}

private func audioMixInfoPayload(from mix: AVAudioMix) -> AudioMixInfoPayload {
    AudioMixInfoPayload(inputParameterCount: mix.inputParameters.count)
}

private func audioMixInputParametersInfoPayload(
    from parameters: AVAudioMixInputParameters
) -> AudioMixInputParametersInfoPayload {
    AudioMixInputParametersInfoPayload(
        trackId: parameters.trackID,
        audioTimePitchAlgorithm: parameters.audioTimePitchAlgorithm?.rawValue
    )
}

private func videoCompositionInfoPayload(from composition: AVVideoComposition) -> VideoCompositionInfoPayload {
    VideoCompositionInfoPayload(
        frameDuration: encodeTime(composition.frameDuration),
        renderSize: SizePayload(
            width: Double(composition.renderSize.width),
            height: Double(composition.renderSize.height)
        ),
        renderScale: {
            if #available(macOS 10.14, *) {
                return composition.renderScale
            }
            return 1.0
        }(),
        instructionCount: composition.instructions.count,
        customVideoCompositorClassName: {
            if #available(macOS 10.9, *) {
                return encodeVideoCompositorClassName(composition.customVideoCompositorClass)
            }
            return nil
        }()
    )
}

private func videoCompositorInfoPayload(from compositor: any AVVideoCompositing) -> VideoCompositorInfoPayload {
    VideoCompositorInfoPayload(
        className: NSStringFromClass(type(of: compositor as AnyObject)),
        sourcePixelBufferAttributesJson: jsonString(fromJSONObject: eraseSendableDictionary(compositor.sourcePixelBufferAttributes)),
        requiredPixelBufferAttributesForRenderContextJson: jsonString(
            fromJSONObject: eraseSendableDictionary(compositor.requiredPixelBufferAttributesForRenderContext)
        ),
        supportsWideColorSourceFrames: {
            if #available(macOS 10.12, *) {
                return compositor.supportsWideColorSourceFrames ?? false
            }
            return false
        }(),
        supportsHDRSourceFrames: {
            if #available(macOS 11.0, *) {
                return compositor.supportsHDRSourceFrames ?? false
            }
            return false
        }()
    )
}

private struct RectPayload: Codable {
    let x: Double
    let y: Double
    let width: Double
    let height: Double
}

private struct AsynchronousVideoCompositionRequestPayload: Codable {
    let compositionTime: TimePayload
    let sourceTrackIDs: [Int32]
    let sourceSampleDataTrackIDs: [Int32]
    let renderSize: SizePayload
    let renderScale: Float
    let videoCompositionInstructionClassName: String?
}

private struct AsynchronousCIImageFilteringRequestPayload: Codable {
    let compositionTime: TimePayload
    let renderSize: SizePayload
    let sourceImageExtent: RectPayload
}

private var latestVideoCompositionRequestJson: String?
private var latestCIImageFilteringRequestJson: String?
private let requestSnapshotLock = NSLock()

private func storeLatestVideoCompositionRequest(_ request: AVAsynchronousVideoCompositionRequest) {
    let payload = AsynchronousVideoCompositionRequestPayload(
        compositionTime: encodeTime(request.compositionTime),
        sourceTrackIDs: request.sourceTrackIDs.map(\.int32Value),
        sourceSampleDataTrackIDs: request.sourceSampleDataTrackIDs,
        renderSize: SizePayload(
            width: Double(request.renderContext.size.width),
            height: Double(request.renderContext.size.height)
        ),
        renderScale: request.renderContext.renderScale,
        videoCompositionInstructionClassName: String(describing: type(of: request.videoCompositionInstruction))
    )
    guard let json = try? encodeJson(payload) else { return }
    requestSnapshotLock.lock()
    latestVideoCompositionRequestJson = json
    requestSnapshotLock.unlock()
}

private func storeLatestCIImageFilteringRequest(_ request: AVAsynchronousCIImageFilteringRequest) {
    let extent = request.sourceImage.extent
    let payload = AsynchronousCIImageFilteringRequestPayload(
        compositionTime: encodeTime(request.compositionTime),
        renderSize: SizePayload(width: Double(request.renderSize.width), height: Double(request.renderSize.height)),
        sourceImageExtent: RectPayload(
            x: Double(extent.origin.x),
            y: Double(extent.origin.y),
            width: Double(extent.size.width),
            height: Double(extent.size.height)
        )
    )
    guard let json = try? encodeJson(payload) else { return }
    requestSnapshotLock.lock()
    latestCIImageFilteringRequestJson = json
    requestSnapshotLock.unlock()
}

private func takeLatestVideoCompositionRequestJson() -> String? {
    requestSnapshotLock.lock()
    defer { requestSnapshotLock.unlock() }
    let json = latestVideoCompositionRequestJson
    latestVideoCompositionRequestJson = nil
    return json
}

private func takeLatestCIImageFilteringRequestJson() -> String? {
    requestSnapshotLock.lock()
    defer { requestSnapshotLock.unlock() }
    let json = latestCIImageFilteringRequestJson
    latestCIImageFilteringRequestJson = nil
    return json
}

final class AVWRPassthroughVideoCompositor: NSObject, AVVideoCompositing {
    private let pixelBufferAttributes: [String: any Sendable] = [
        kCVPixelBufferPixelFormatTypeKey as String: [Int(kCVPixelFormatType_32BGRA)]
    ]

    var sourcePixelBufferAttributes: [String: any Sendable]? {
        pixelBufferAttributes
    }

    var requiredPixelBufferAttributesForRenderContext: [String: any Sendable] {
        pixelBufferAttributes
    }

    func renderContextChanged(_ newRenderContext: AVVideoCompositionRenderContext) {}

    func startRequest(_ request: AVAsynchronousVideoCompositionRequest) {
        storeLatestVideoCompositionRequest(request)
        guard
            let trackID = request.sourceTrackIDs.first?.int32Value,
            let frame = request.sourceFrame(byTrackID: trackID)
        else {
            request.finish(with: NSError(
                domain: "avassetwriter",
                code: -1,
                userInfo: [NSLocalizedDescriptionKey: "passthrough compositor had no source frame"]
            ))
            return
        }
        request.finish(withComposedVideoFrame: frame)
    }

    func cancelAllPendingVideoCompositionRequests() {}

    var supportsWideColorSourceFrames: Bool {
        false
    }

    var supportsHDRSourceFrames: Bool {
        false
    }
}

@_cdecl("av_metadata_item_filter_for_sharing")
public func av_metadata_item_filter_for_sharing() -> UnsafeMutableRawPointer? {
    guard #available(macOS 10.9, *) else {
        return nil
    }
    return Unmanaged.passRetained(AVMetadataItemFilter.forSharing()).toOpaque()
}

@_cdecl("av_metadata_item_filter_kind")
public func av_metadata_item_filter_kind(
    _ filterPtr: UnsafeMutableRawPointer
) -> UnsafeMutablePointer<CChar>? {
    let filter = Unmanaged<AVMetadataItemFilter>.fromOpaque(filterPtr).takeUnretainedValue()
    return ffiString(metadataItemFilterKind(filter))
}

@_cdecl("av_metadata_item_filter_release")
public func av_metadata_item_filter_release(_ filterPtr: UnsafeMutableRawPointer?) {
    guard let filterPtr else { return }
    Unmanaged<AVMetadataItemFilter>.fromOpaque(filterPtr).release()
}

@_cdecl("av_audio_mix_create")
public func av_audio_mix_create() -> UnsafeMutableRawPointer? {
    Unmanaged.passRetained(AVMutableAudioMix()).toOpaque()
}

@_cdecl("av_audio_mix_info_json")
public func av_audio_mix_info_json(
    _ mixPtr: UnsafeMutableRawPointer
) -> UnsafeMutablePointer<CChar>? {
    let mix = Unmanaged<AVAudioMix>.fromOpaque(mixPtr).takeUnretainedValue()
    do {
        return ffiString(try encodeJson(audioMixInfoPayload(from: mix)))
    } catch {
        return nil
    }
}

@_cdecl("av_audio_mix_input_parameter_count")
public func av_audio_mix_input_parameter_count(_ mixPtr: UnsafeMutableRawPointer) -> Int {
    let mix = Unmanaged<AVAudioMix>.fromOpaque(mixPtr).takeUnretainedValue()
    return mix.inputParameters.count
}

@_cdecl("av_audio_mix_copy_input_parameter_at_index")
public func av_audio_mix_copy_input_parameter_at_index(
    _ mixPtr: UnsafeMutableRawPointer,
    _ index: Int
) -> UnsafeMutableRawPointer? {
    let mix = Unmanaged<AVAudioMix>.fromOpaque(mixPtr).takeUnretainedValue()
    guard index >= 0, index < mix.inputParameters.count else { return nil }
    return Unmanaged.passRetained(mix.inputParameters[index]).toOpaque()
}

@_cdecl("av_audio_mix_set_input_parameters")
public func av_audio_mix_set_input_parameters(
    _ mixPtr: UnsafeMutableRawPointer,
    _ parametersPtr: UnsafePointer<UnsafeMutableRawPointer?>,
    _ count: Int,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    guard let mix = Unmanaged<AVAudioMix>.fromOpaque(mixPtr).takeUnretainedValue() as? AVMutableAudioMix else {
        outErrorMessage?.pointee = ffiString("audio mix is not mutable")
        return AVW_INVALID_STATE
    }
    var parameters: [AVAudioMixInputParameters] = []
    parameters.reserveCapacity(count)
    for index in 0..<count {
        guard let parameterPtr = parametersPtr[index] else {
            outErrorMessage?.pointee = ffiString("audio mix input parameters array contained a null entry")
            return AVW_INVALID_ARGUMENT
        }
        parameters.append(Unmanaged<AVAudioMixInputParameters>.fromOpaque(parameterPtr).takeUnretainedValue())
    }
    mix.inputParameters = parameters
    return AVW_OK
}

@_cdecl("av_audio_mix_release")
public func av_audio_mix_release(_ mixPtr: UnsafeMutableRawPointer?) {
    guard let mixPtr else { return }
    Unmanaged<AVAudioMix>.fromOpaque(mixPtr).release()
}

@_cdecl("av_audio_mix_input_parameters_create")
public func av_audio_mix_input_parameters_create() -> UnsafeMutableRawPointer? {
    Unmanaged.passRetained(AVMutableAudioMixInputParameters()).toOpaque()
}

@_cdecl("av_audio_mix_input_parameters_info_json")
public func av_audio_mix_input_parameters_info_json(
    _ parametersPtr: UnsafeMutableRawPointer
) -> UnsafeMutablePointer<CChar>? {
    let parameters = Unmanaged<AVAudioMixInputParameters>.fromOpaque(parametersPtr).takeUnretainedValue()
    do {
        return ffiString(try encodeJson(audioMixInputParametersInfoPayload(from: parameters)))
    } catch {
        return nil
    }
}

@_cdecl("av_audio_mix_input_parameters_volume_ramp_json")
public func av_audio_mix_input_parameters_volume_ramp_json(
    _ parametersPtr: UnsafeMutableRawPointer,
    _ timeValue: Int64,
    _ timeScale: Int32,
    _ timeKind: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutablePointer<CChar>? {
    let parameters = Unmanaged<AVAudioMixInputParameters>.fromOpaque(parametersPtr).takeUnretainedValue()
    var startVolume: Float = 0
    var endVolume: Float = 0
    var timeRange = CMTimeRange.invalid
    guard parameters.getVolumeRamp(
        for: cmTime(value: timeValue, timescale: timeScale, kind: timeKind),
        startVolume: &startVolume,
        endVolume: &endVolume,
        timeRange: &timeRange
    ) else {
        _ = outErrorMessage
        return nil
    }
    do {
        return ffiString(try encodeJson(AudioVolumeRampPayload(
            startVolume: startVolume,
            endVolume: endVolume,
            timeRange: encodeTimeRange(timeRange)
        )))
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_audio_mix_input_parameters_set_track_id")
public func av_audio_mix_input_parameters_set_track_id(
    _ parametersPtr: UnsafeMutableRawPointer,
    _ trackID: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    guard let parameters = Unmanaged<AVAudioMixInputParameters>.fromOpaque(parametersPtr).takeUnretainedValue() as? AVMutableAudioMixInputParameters else {
        outErrorMessage?.pointee = ffiString("audio mix input parameters are not mutable")
        return AVW_INVALID_STATE
    }
    parameters.trackID = trackID
    return AVW_OK
}

@_cdecl("av_audio_mix_input_parameters_set_audio_time_pitch_algorithm")
public func av_audio_mix_input_parameters_set_audio_time_pitch_algorithm(
    _ parametersPtr: UnsafeMutableRawPointer,
    _ algorithmPtr: UnsafePointer<CChar>?,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    guard let parameters = Unmanaged<AVAudioMixInputParameters>.fromOpaque(parametersPtr).takeUnretainedValue() as? AVMutableAudioMixInputParameters else {
        outErrorMessage?.pointee = ffiString("audio mix input parameters are not mutable")
        return AVW_INVALID_STATE
    }
    parameters.audioTimePitchAlgorithm = algorithmPtr.map {
        AVAudioTimePitchAlgorithm(rawValue: String(cString: $0))
    }
    return AVW_OK
}

@_cdecl("av_audio_mix_input_parameters_set_volume")
public func av_audio_mix_input_parameters_set_volume(
    _ parametersPtr: UnsafeMutableRawPointer,
    _ volume: Float,
    _ timeValue: Int64,
    _ timeScale: Int32,
    _ timeKind: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    guard let parameters = Unmanaged<AVAudioMixInputParameters>.fromOpaque(parametersPtr).takeUnretainedValue() as? AVMutableAudioMixInputParameters else {
        outErrorMessage?.pointee = ffiString("audio mix input parameters are not mutable")
        return AVW_INVALID_STATE
    }
    parameters.setVolume(volume, at: cmTime(value: timeValue, timescale: timeScale, kind: timeKind))
    return AVW_OK
}

@_cdecl("av_audio_mix_input_parameters_set_volume_ramp")
public func av_audio_mix_input_parameters_set_volume_ramp(
    _ parametersPtr: UnsafeMutableRawPointer,
    _ startVolume: Float,
    _ endVolume: Float,
    _ startValue: Int64,
    _ startScale: Int32,
    _ startKind: Int32,
    _ durationValue: Int64,
    _ durationScale: Int32,
    _ durationKind: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    guard let parameters = Unmanaged<AVAudioMixInputParameters>.fromOpaque(parametersPtr).takeUnretainedValue() as? AVMutableAudioMixInputParameters else {
        outErrorMessage?.pointee = ffiString("audio mix input parameters are not mutable")
        return AVW_INVALID_STATE
    }
    parameters.setVolumeRamp(
        fromStartVolume: startVolume,
        toEndVolume: endVolume,
        timeRange: CMTimeRange(
            start: cmTime(value: startValue, timescale: startScale, kind: startKind),
            duration: cmTime(value: durationValue, timescale: durationScale, kind: durationKind)
        )
    )
    return AVW_OK
}

@_cdecl("av_audio_mix_input_parameters_release")
public func av_audio_mix_input_parameters_release(_ parametersPtr: UnsafeMutableRawPointer?) {
    guard let parametersPtr else { return }
    Unmanaged<AVAudioMixInputParameters>.fromOpaque(parametersPtr).release()
}

@_cdecl("av_video_composition_create_from_asset")
public func av_video_composition_create_from_asset(
    _ pathPtr: UnsafePointer<CChar>
) -> UnsafeMutableRawPointer? {
    Unmanaged.passRetained(AVMutableVideoComposition(propertiesOf: mediaProcessingAsset(from: pathPtr))).toOpaque()
}

@_cdecl("av_video_composition_create_from_asset_ci_filter_recorder")
public func av_video_composition_create_from_asset_ci_filter_recorder(
    _ pathPtr: UnsafePointer<CChar>
) -> UnsafeMutableRawPointer? {
    let asset = mediaProcessingAsset(from: pathPtr)
    let composition = AVMutableVideoComposition(asset: asset) { request in
        storeLatestCIImageFilteringRequest(request)
        request.finish(with: request.sourceImage, context: nil)
    }
    return Unmanaged.passRetained(composition).toOpaque()
}

@_cdecl("av_video_composition_info_json")
public func av_video_composition_info_json(
    _ compositionPtr: UnsafeMutableRawPointer
) -> UnsafeMutablePointer<CChar>? {
    let composition = Unmanaged<AVVideoComposition>.fromOpaque(compositionPtr).takeUnretainedValue()
    do {
        return ffiString(try encodeJson(videoCompositionInfoPayload(from: composition)))
    } catch {
        return nil
    }
}

@_cdecl("av_video_composition_set_custom_video_compositor_class")
public func av_video_composition_set_custom_video_compositor_class(
    _ compositionPtr: UnsafeMutableRawPointer,
    _ classPtr: UnsafePointer<CChar>?,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    guard #available(macOS 10.9, *) else {
        outErrorMessage?.pointee = ffiString("custom video compositors require macOS 10.9+")
        return AVW_INVALID_STATE
    }
    guard let composition = Unmanaged<AVVideoComposition>.fromOpaque(compositionPtr).takeUnretainedValue() as? AVMutableVideoComposition else {
        outErrorMessage?.pointee = ffiString("video composition is not mutable")
        return AVW_INVALID_STATE
    }
    if let classPtr {
        let raw = String(cString: classPtr)
        guard let compositorClass = decodeVideoCompositorClass(raw) else {
            outErrorMessage?.pointee = ffiString("unknown video compositor class '\(raw)'")
            return AVW_INVALID_ARGUMENT
        }
        composition.customVideoCompositorClass = compositorClass
    } else {
        composition.customVideoCompositorClass = nil
    }
    return AVW_OK
}

@_cdecl("av_video_composition_release")
public func av_video_composition_release(_ compositionPtr: UnsafeMutableRawPointer?) {
    guard let compositionPtr else { return }
    Unmanaged<AVVideoComposition>.fromOpaque(compositionPtr).release()
}

@_cdecl("av_video_compositor_info_json")
public func av_video_compositor_info_json(
    _ compositorPtr: UnsafeMutableRawPointer
) -> UnsafeMutablePointer<CChar>? {
    let object = Unmanaged<NSObject>.fromOpaque(compositorPtr).takeUnretainedValue()
    guard let compositor = object as? AVVideoCompositing else {
        return nil
    }
    do {
        return ffiString(try encodeJson(videoCompositorInfoPayload(from: compositor)))
    } catch {
        return nil
    }
}

@_cdecl("av_take_latest_video_composition_request_json")
public func av_take_latest_video_composition_request_json() -> UnsafeMutablePointer<CChar>? {
    guard let json = takeLatestVideoCompositionRequestJson() else { return nil }
    return ffiString(json)
}

@_cdecl("av_take_latest_ci_image_filtering_request_json")
public func av_take_latest_ci_image_filtering_request_json() -> UnsafeMutablePointer<CChar>? {
    guard let json = takeLatestCIImageFilteringRequestJson() else { return nil }
    return ffiString(json)
}

@_cdecl("av_video_compositor_release")
public func av_video_compositor_release(_ compositorPtr: UnsafeMutableRawPointer?) {
    guard let compositorPtr else { return }
    Unmanaged<NSObject>.fromOpaque(compositorPtr).release()
}
