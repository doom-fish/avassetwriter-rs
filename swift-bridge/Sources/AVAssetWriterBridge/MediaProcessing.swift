import AVFoundation
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

private func audioMixInfoPayload(from mix: AVAudioMix) -> AudioMixInfoPayload {
    AudioMixInfoPayload(inputParameterCount: mix.inputParameters.count)
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

@_cdecl("av_audio_mix_release")
public func av_audio_mix_release(_ mixPtr: UnsafeMutableRawPointer?) {
    guard let mixPtr else { return }
    Unmanaged<AVAudioMix>.fromOpaque(mixPtr).release()
}

@_cdecl("av_video_composition_create_from_asset")
public func av_video_composition_create_from_asset(
    _ pathPtr: UnsafePointer<CChar>
) -> UnsafeMutableRawPointer? {
    Unmanaged.passRetained(AVMutableVideoComposition(propertiesOf: mediaProcessingAsset(from: pathPtr))).toOpaque()
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

@_cdecl("av_video_compositor_release")
public func av_video_compositor_release(_ compositorPtr: UnsafeMutableRawPointer?) {
    guard let compositorPtr else { return }
    Unmanaged<NSObject>.fromOpaque(compositorPtr).release()
}
