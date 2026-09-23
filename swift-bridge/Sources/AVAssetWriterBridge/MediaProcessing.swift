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

private let requestSnapshotWireFormatVersion: UInt32 = 1

private struct AsynchronousVideoCompositionRequestPayload: Codable {
    let wireFormatVersion: UInt32
    let generation: UInt64
    let compositionTime: TimePayload
    let sourceTrackIDs: [Int32]
    let sourceSampleDataTrackIDs: [Int32]
    let renderSize: SizePayload
    let renderScale: Float
    let videoCompositionInstructionClassName: String?
}

private struct AsynchronousCIImageFilteringRequestPayload: Codable {
    let wireFormatVersion: UInt32
    let generation: UInt64
    let compositionTime: TimePayload
    let renderSize: SizePayload
    let sourceImageExtent: RectPayload
}

private final class RequestSnapshotStore: @unchecked Sendable {
    private struct Entry {
        let generation: UInt64
        let json: String
    }

    private let lock = NSLock()
    private var nextGeneration: UInt64? = 1
    private var latest: Entry?

    func store(_ encode: (UInt64) -> String?) {
        lock.lock()
        defer { lock.unlock() }
        guard let generation = nextGeneration else { return }
        nextGeneration = generation == UInt64.max ? nil : generation + 1
        guard let json = encode(generation) else { return }
        latest = Entry(generation: generation, json: json)
    }

    func copy() -> (generation: UInt64, json: String)? {
        lock.lock()
        defer { lock.unlock() }
        guard let latest else { return nil }
        return (latest.generation, latest.json)
    }

    func acknowledge(_ generation: UInt64) -> Bool {
        lock.lock()
        defer { lock.unlock() }
        guard latest?.generation == generation else { return false }
        latest = nil
        return true
    }

    func take() -> String? {
        lock.lock()
        defer { lock.unlock() }
        guard let latest else { return nil }
        self.latest = nil
        return latest.json
    }
}

private let videoCompositionRequestSnapshots = RequestSnapshotStore()
private let ciImageFilteringRequestSnapshots = RequestSnapshotStore()

private final class CIImageFilteringCompositionRegistry: @unchecked Sendable {
    private let lock = NSLock()
    private let compositions = NSHashTable<AVMutableVideoComposition>.weakObjects()

    func insert(_ composition: AVMutableVideoComposition) {
        lock.lock()
        compositions.add(composition)
        lock.unlock()
    }

    func contains(_ composition: AVMutableVideoComposition) -> Bool {
        lock.lock()
        defer { lock.unlock() }
        return compositions.contains(composition)
    }
}

private let ciImageFilteringCompositions = CIImageFilteringCompositionRegistry()

private func storeLatestVideoCompositionRequest(_ request: AVAsynchronousVideoCompositionRequest) {
    videoCompositionRequestSnapshots.store { generation in
        try? encodeJson(AsynchronousVideoCompositionRequestPayload(
            wireFormatVersion: requestSnapshotWireFormatVersion,
            generation: generation,
            compositionTime: encodeTime(request.compositionTime),
            sourceTrackIDs: request.sourceTrackIDs.map(\.int32Value),
            sourceSampleDataTrackIDs: request.sourceSampleDataTrackIDs,
            renderSize: SizePayload(
                width: Double(request.renderContext.size.width),
                height: Double(request.renderContext.size.height)
            ),
            renderScale: request.renderContext.renderScale,
            videoCompositionInstructionClassName: String(
                describing: type(of: request.videoCompositionInstruction)
            )
        ))
    }
}

private func storeLatestCIImageFilteringRequest(_ request: AVAsynchronousCIImageFilteringRequest) {
    let extent = request.sourceImage.extent
    ciImageFilteringRequestSnapshots.store { generation in
        try? encodeJson(AsynchronousCIImageFilteringRequestPayload(
            wireFormatVersion: requestSnapshotWireFormatVersion,
            generation: generation,
            compositionTime: encodeTime(request.compositionTime),
            renderSize: SizePayload(
                width: Double(request.renderSize.width),
                height: Double(request.renderSize.height)
            ),
            sourceImageExtent: RectPayload(
                x: Double(extent.origin.x),
                y: Double(extent.origin.y),
                width: Double(extent.size.width),
                height: Double(extent.size.height)
            )
        ))
    }
}

public typealias AVWVideoCompositionRequestCallback = @convention(c) (
    UnsafeMutableRawPointer?,
    UnsafeMutableRawPointer?
) -> Void

private final class AVWRRustVideoCompositionHandler: @unchecked Sendable {
    let callback: AVWVideoCompositionRequestCallback
    let userdata: UnsafeMutableRawPointer
    let dropUserdata: AVWDropCallback

    init(
        callback: @escaping AVWVideoCompositionRequestCallback,
        userdata: UnsafeMutableRawPointer,
        dropUserdata: @escaping AVWDropCallback
    ) {
        self.callback = callback
        self.userdata = userdata
        self.dropUserdata = dropUserdata
    }

    func emit(_ request: AVWRRustVideoCompositionRequest) {
        callback(Unmanaged.passRetained(request).toOpaque(), userdata)
    }

    deinit {
        dropUserdata(userdata)
    }
}

private final class AVWRRustVideoCompositionInstruction:
    NSObject,
    AVVideoCompositionInstructionProtocol,
    @unchecked Sendable
{
    let originalInstruction: any AVVideoCompositionInstructionProtocol
    let handler: AVWRRustVideoCompositionHandler
    let timeRange: CMTimeRange
    let enablePostProcessing: Bool
    let containsTweening: Bool
    let requiredSourceTrackIDs: [NSValue]?
    let passthroughTrackID: CMPersistentTrackID = kCMPersistentTrackID_Invalid
    let requiredSourceSampleDataTrackIDs: [NSNumber]

    init(
        originalInstruction: any AVVideoCompositionInstructionProtocol,
        sourceSampleDataTrackIDs: [NSNumber],
        handler: AVWRRustVideoCompositionHandler
    ) {
        self.originalInstruction = originalInstruction
        self.handler = handler
        self.timeRange = originalInstruction.timeRange
        self.enablePostProcessing = originalInstruction.enablePostProcessing
        self.containsTweening = originalInstruction.containsTweening
        var requiredSourceTrackIDs = originalInstruction.requiredSourceTrackIDs
        let passthroughTrackID = originalInstruction.passthroughTrackID
        if passthroughTrackID != kCMPersistentTrackID_Invalid {
            var trackIDs = requiredSourceTrackIDs ?? []
            if !trackIDs.contains(where: {
                ($0 as? NSNumber)?.int32Value == passthroughTrackID
            }) {
                trackIDs.append(NSNumber(value: passthroughTrackID))
            }
            requiredSourceTrackIDs = trackIDs
        }
        self.requiredSourceTrackIDs = requiredSourceTrackIDs
        self.requiredSourceSampleDataTrackIDs = sourceSampleDataTrackIDs
    }
}

private enum AVWRRustVideoCompositionRequestState: String {
    case pending
    case finishedWithFrame
    case finishedWithError
    case cancelled
}

private final class AVWRRustVideoCompositionRequest: @unchecked Sendable {
    let identifier: UInt64
    let request: AVAsynchronousVideoCompositionRequest
    let handler: AVWRRustVideoCompositionHandler

    private weak var owner: AVWRRustVideoCompositor?
    private let lock = NSRecursiveLock()
    private var state = AVWRRustVideoCompositionRequestState.pending

    init(
        identifier: UInt64,
        request: AVAsynchronousVideoCompositionRequest,
        handler: AVWRRustVideoCompositionHandler,
        owner: AVWRRustVideoCompositor
    ) {
        self.identifier = identifier
        self.request = request
        self.handler = handler
        self.owner = owner
    }

    func sourceFrame(trackID: Int32) throws -> CVPixelBuffer? {
        lock.lock()
        defer { lock.unlock() }
        try requirePending()
        return request.sourceFrame(byTrackID: trackID)
    }

    func sourceSampleBuffer(trackID: Int32) throws -> CMSampleBuffer? {
        lock.lock()
        defer { lock.unlock() }
        try requirePending()
        return request.sourceSampleBuffer(byTrackID: trackID)
    }

    func newOutputPixelBuffer() throws -> CVPixelBuffer {
        lock.lock()
        defer { lock.unlock() }
        try requirePending()
        guard let frame = request.renderContext.newPixelBuffer() else {
            throw BridgeError.message("video composition render context could not allocate a pixel buffer")
        }
        return frame
    }

    func finish(with frame: CVPixelBuffer) -> String? {
        transition(to: .finishedWithFrame) {
            request.finish(withComposedVideoFrame: frame)
        }
    }

    func finish(withErrorMessage message: String) -> String? {
        transition(to: .finishedWithError) {
            request.finish(with: NSError(
                domain: "avassetwriter.rust-video-compositor",
                code: -1,
                userInfo: [NSLocalizedDescriptionKey: message]
            ))
        }
    }

    func cancel() -> String? {
        transition(to: .cancelled) {
            request.finishCancelledRequest()
        }
    }

    func cancelFromSystem() {
        _ = transition(to: .cancelled) {
            request.finishCancelledRequest()
        }
    }

    private func requirePending() throws {
        guard state == .pending else {
            throw BridgeError.message("video composition request is already \(state.rawValue)")
        }
    }

    private func transition(
        to terminalState: AVWRRustVideoCompositionRequestState,
        action: () -> Void
    ) -> String? {
        lock.lock()
        guard state == .pending else {
            let currentState = state
            lock.unlock()
            return "video composition request is already \(currentState.rawValue)"
        }
        state = terminalState
        action()
        lock.unlock()
        owner?.removePendingRequest(identifier)
        return nil
    }

    deinit {
        cancelFromSystem()
    }
}

final class AVWRRustVideoCompositor: NSObject, AVVideoCompositing, @unchecked Sendable {
    private let pixelBufferAttributes: [String: any Sendable] = [
        kCVPixelBufferPixelFormatTypeKey as String: [Int(kCVPixelFormatType_32BGRA)]
    ]
    private let pendingLock = NSLock()
    private var pendingRequests: [UInt64: AVWRRustVideoCompositionRequest] = [:]
    private var nextRequestIdentifier: UInt64 = 1
    private var isCancelling = false

    var sourcePixelBufferAttributes: [String: any Sendable]? {
        pixelBufferAttributes
    }

    var requiredPixelBufferAttributesForRenderContext: [String: any Sendable] {
        pixelBufferAttributes
    }

    func renderContextChanged(_ newRenderContext: AVVideoCompositionRenderContext) {}

    func startRequest(_ request: AVAsynchronousVideoCompositionRequest) {
        storeLatestVideoCompositionRequest(request)
        guard let instruction = request.videoCompositionInstruction
            as? AVWRRustVideoCompositionInstruction
        else {
            request.finish(with: NSError(
                domain: "avassetwriter.rust-video-compositor",
                code: -1,
                userInfo: [
                    NSLocalizedDescriptionKey:
                        "Rust video compositor requires AVWRRustVideoCompositionInstruction"
                ]
            ))
            return
        }

        let (handle, shouldInvokeHandler) = register(request: request, instruction: instruction)
        if !shouldInvokeHandler {
            return
        }
        instruction.handler.emit(handle)
    }

    func cancelAllPendingVideoCompositionRequests() {
        pendingLock.lock()
        isCancelling = true
        while true {
            let requests = Array(pendingRequests.values)
            pendingRequests.removeAll()
            pendingLock.unlock()

            for request in requests {
                request.cancelFromSystem()
            }

            pendingLock.lock()
            if pendingRequests.isEmpty {
                isCancelling = false
                pendingLock.unlock()
                return
            }
        }
    }

    var supportsWideColorSourceFrames: Bool {
        false
    }

    var supportsHDRSourceFrames: Bool {
        false
    }

    @available(macOS 26.0, *)
    var supportsSourceTaggedBuffers: Bool {
        false
    }

    var canConformColorOfSourceFrames: Bool {
        false
    }

    fileprivate func removePendingRequest(_ identifier: UInt64) {
        pendingLock.lock()
        pendingRequests.removeValue(forKey: identifier)
        pendingLock.unlock()
    }

    private func register(
        request: AVAsynchronousVideoCompositionRequest,
        instruction: AVWRRustVideoCompositionInstruction
    ) -> (AVWRRustVideoCompositionRequest, Bool) {
        pendingLock.lock()
        var identifier = nextRequestIdentifier
        while pendingRequests[identifier] != nil {
            identifier = identifier == UInt64.max ? 1 : identifier + 1
        }
        nextRequestIdentifier = identifier == UInt64.max ? 1 : identifier + 1
        let handle = AVWRRustVideoCompositionRequest(
            identifier: identifier,
            request: request,
            handler: instruction.handler,
            owner: self
        )
        pendingRequests[identifier] = handle
        let shouldInvokeHandler = !isCancelling
        pendingLock.unlock()
        return (handle, shouldInvokeHandler)
    }
}

final class AVWRPassthroughVideoCompositor: NSObject, AVVideoCompositing, @unchecked Sendable {
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

private func baseVideoCompositionInstruction(
    _ instruction: any AVVideoCompositionInstructionProtocol
) -> any AVVideoCompositionInstructionProtocol {
    if let rustInstruction = instruction as? AVWRRustVideoCompositionInstruction {
        return rustInstruction.originalInstruction
    }
    return instruction
}

private func restoreRustVideoCompositionInstructions(_ composition: AVMutableVideoComposition) {
    composition.instructions = composition.instructions.map(baseVideoCompositionInstruction)
}

private func hasTaggedBufferOutput(_ composition: AVMutableVideoComposition) -> Bool {
    guard #available(macOS 26.0, *) else { return false }
    let selector = NSSelectorFromString("outputBufferDescription")
    guard composition.responds(to: selector) else { return false }
    return ((composition.value(forKey: "outputBufferDescription") as? NSArray)?.count ?? 0) > 0
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
    let time = cmTime(value: timeValue, timescale: timeScale, kind: timeKind)
    guard time.isNumeric else {
        outErrorMessage?.pointee = ffiString("volume time must be numeric")
        return AVW_INVALID_ARGUMENT
    }
    parameters.setVolume(volume, at: time)
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
    let timeRange = CMTimeRange(
        start: cmTime(value: startValue, timescale: startScale, kind: startKind),
        duration: cmTime(value: durationValue, timescale: durationScale, kind: durationKind)
    )
    guard timeRange.start.isNumeric, timeRange.duration.isNumeric else {
        outErrorMessage?.pointee = ffiString("volume ramp start and duration must be numeric")
        return AVW_INVALID_ARGUMENT
    }
    parameters.setVolumeRamp(fromStartVolume: startVolume, toEndVolume: endVolume, timeRange: timeRange)
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
    ciImageFilteringCompositions.insert(composition)
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
        if raw == "rustCallback" {
            outErrorMessage?.pointee = ffiString(
                "the Rust callback compositor must be installed with a request handler"
            )
            return AVW_INVALID_ARGUMENT
        }
        guard let compositorClass = decodeVideoCompositorClass(raw) else {
            outErrorMessage?.pointee = ffiString("unknown video compositor class '\(raw)'")
            return AVW_INVALID_ARGUMENT
        }
        restoreRustVideoCompositionInstructions(composition)
        composition.customVideoCompositorClass = compositorClass
    } else {
        restoreRustVideoCompositionInstructions(composition)
        composition.customVideoCompositorClass = nil
    }
    return AVW_OK
}

@_cdecl("av_video_composition_install_rust_compositor")
public func av_video_composition_install_rust_compositor(
    _ compositionPtr: UnsafeMutableRawPointer,
    _ callback: AVWVideoCompositionRequestCallback?,
    _ userdata: UnsafeMutableRawPointer?,
    _ dropUserdata: AVWDropCallback?,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    guard let composition = Unmanaged<AVVideoComposition>
        .fromOpaque(compositionPtr)
        .takeUnretainedValue() as? AVMutableVideoComposition
    else {
        outErrorMessage?.pointee = ffiString("video composition is not mutable")
        return AVW_INVALID_STATE
    }
    guard let callback, let userdata, let dropUserdata else {
        outErrorMessage?.pointee = ffiString("Rust video compositor handler was incomplete")
        return AVW_INVALID_ARGUMENT
    }
    if ciImageFilteringCompositions.contains(composition) {
        outErrorMessage?.pointee = ffiString(
            "CI image-filtering compositions are not supported by the Rust callback compositor"
        )
        return AVW_INVALID_STATE
    }
    guard !composition.instructions.isEmpty else {
        outErrorMessage?.pointee = ffiString("video composition has no instructions to process")
        return AVW_INVALID_STATE
    }
    if hasTaggedBufferOutput(composition) {
        outErrorMessage?.pointee = ffiString(
            "tagged-buffer video composition output is not supported by the Rust callback compositor"
        )
        return AVW_INVALID_STATE
    }
    if let currentClass = composition.customVideoCompositorClass,
       ObjectIdentifier(currentClass) != ObjectIdentifier(AVWRPassthroughVideoCompositor.self),
       ObjectIdentifier(currentClass) != ObjectIdentifier(AVWRRustVideoCompositor.self)
    {
        outErrorMessage?.pointee = ffiString(
            "video compositions using another custom compositor, including CI filtering, are unsupported"
        )
        return AVW_INVALID_STATE
    }

    let handler = AVWRRustVideoCompositionHandler(
        callback: callback,
        userdata: userdata,
        dropUserdata: dropUserdata
    )
    let sourceSampleDataTrackIDs = composition.sourceSampleDataTrackIDs.map(NSNumber.init)
    composition.instructions = composition.instructions.map { instruction in
        AVWRRustVideoCompositionInstruction(
            originalInstruction: baseVideoCompositionInstruction(instruction),
            sourceSampleDataTrackIDs: sourceSampleDataTrackIDs,
            handler: handler
        )
    }
    composition.customVideoCompositorClass = AVWRRustVideoCompositor.self
    return AVW_OK
}

@_cdecl("av_video_composition_clear_rust_compositor")
public func av_video_composition_clear_rust_compositor(
    _ compositionPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    guard let composition = Unmanaged<AVVideoComposition>
        .fromOpaque(compositionPtr)
        .takeUnretainedValue() as? AVMutableVideoComposition
    else {
        outErrorMessage?.pointee = ffiString("video composition is not mutable")
        return AVW_INVALID_STATE
    }
    restoreRustVideoCompositionInstructions(composition)
    if let currentClass = composition.customVideoCompositorClass,
       ObjectIdentifier(currentClass) == ObjectIdentifier(AVWRRustVideoCompositor.self)
    {
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

@_cdecl("av_copy_latest_video_composition_request_json")
public func av_copy_latest_video_composition_request_json(
    _ outGeneration: UnsafeMutablePointer<UInt64>?
) -> UnsafeMutablePointer<CChar>? {
    guard let snapshot = videoCompositionRequestSnapshots.copy() else { return nil }
    outGeneration?.pointee = snapshot.generation
    return ffiString(snapshot.json)
}

@_cdecl("av_ack_latest_video_composition_request")
public func av_ack_latest_video_composition_request(_ generation: UInt64) -> Bool {
    videoCompositionRequestSnapshots.acknowledge(generation)
}

@_cdecl("av_take_latest_video_composition_request_json")
public func av_take_latest_video_composition_request_json() -> UnsafeMutablePointer<CChar>? {
    guard let json = videoCompositionRequestSnapshots.take() else { return nil }
    return ffiString(json)
}

@_cdecl("av_copy_latest_ci_image_filtering_request_json")
public func av_copy_latest_ci_image_filtering_request_json(
    _ outGeneration: UnsafeMutablePointer<UInt64>?
) -> UnsafeMutablePointer<CChar>? {
    guard let snapshot = ciImageFilteringRequestSnapshots.copy() else { return nil }
    outGeneration?.pointee = snapshot.generation
    return ffiString(snapshot.json)
}

@_cdecl("av_ack_latest_ci_image_filtering_request")
public func av_ack_latest_ci_image_filtering_request(_ generation: UInt64) -> Bool {
    ciImageFilteringRequestSnapshots.acknowledge(generation)
}

@_cdecl("av_take_latest_ci_image_filtering_request_json")
public func av_take_latest_ci_image_filtering_request_json() -> UnsafeMutablePointer<CChar>? {
    guard let json = ciImageFilteringRequestSnapshots.take() else { return nil }
    return ffiString(json)
}

@_cdecl("av_video_composition_request_release")
public func av_video_composition_request_release(_ requestPtr: UnsafeMutableRawPointer?) {
    guard let requestPtr else { return }
    Unmanaged<AVWRRustVideoCompositionRequest>.fromOpaque(requestPtr).release()
}

@_cdecl("av_video_composition_request_composition_time")
public func av_video_composition_request_composition_time(
    _ requestPtr: UnsafeMutableRawPointer,
    _ outValue: UnsafeMutablePointer<Int64>?,
    _ outScale: UnsafeMutablePointer<Int32>?,
    _ outKind: UnsafeMutablePointer<Int32>?
) {
    let handle = Unmanaged<AVWRRustVideoCompositionRequest>
        .fromOpaque(requestPtr)
        .takeUnretainedValue()
    let time = encodeTime(handle.request.compositionTime)
    outValue?.pointee = time.value ?? 0
    outScale?.pointee = time.timescale ?? 0
    switch time.kind {
    case "numeric":
        outKind?.pointee = 0
    case "invalid":
        outKind?.pointee = 1
    case "indefinite":
        outKind?.pointee = 2
    case "positive_infinity":
        outKind?.pointee = 3
    case "negative_infinity":
        outKind?.pointee = 4
    default:
        outKind?.pointee = -1
    }
}

@_cdecl("av_video_composition_request_render_info")
public func av_video_composition_request_render_info(
    _ requestPtr: UnsafeMutableRawPointer,
    _ outWidth: UnsafeMutablePointer<Double>?,
    _ outHeight: UnsafeMutablePointer<Double>?,
    _ outScale: UnsafeMutablePointer<Float>?
) {
    let handle = Unmanaged<AVWRRustVideoCompositionRequest>
        .fromOpaque(requestPtr)
        .takeUnretainedValue()
    outWidth?.pointee = Double(handle.request.renderContext.size.width)
    outHeight?.pointee = Double(handle.request.renderContext.size.height)
    outScale?.pointee = handle.request.renderContext.renderScale
}

@_cdecl("av_video_composition_request_source_track_id_count")
public func av_video_composition_request_source_track_id_count(
    _ requestPtr: UnsafeMutableRawPointer
) -> Int {
    let handle = Unmanaged<AVWRRustVideoCompositionRequest>
        .fromOpaque(requestPtr)
        .takeUnretainedValue()
    return handle.request.sourceTrackIDs.count
}

@_cdecl("av_video_composition_request_source_track_id_at")
public func av_video_composition_request_source_track_id_at(
    _ requestPtr: UnsafeMutableRawPointer,
    _ index: Int,
    _ outTrackID: UnsafeMutablePointer<Int32>?
) -> Bool {
    let handle = Unmanaged<AVWRRustVideoCompositionRequest>
        .fromOpaque(requestPtr)
        .takeUnretainedValue()
    guard index >= 0, index < handle.request.sourceTrackIDs.count else { return false }
    outTrackID?.pointee = handle.request.sourceTrackIDs[index].int32Value
    return true
}

@_cdecl("av_video_composition_request_source_sample_data_track_id_count")
public func av_video_composition_request_source_sample_data_track_id_count(
    _ requestPtr: UnsafeMutableRawPointer
) -> Int {
    let handle = Unmanaged<AVWRRustVideoCompositionRequest>
        .fromOpaque(requestPtr)
        .takeUnretainedValue()
    return handle.request.sourceSampleDataTrackIDs.count
}

@_cdecl("av_video_composition_request_source_sample_data_track_id_at")
public func av_video_composition_request_source_sample_data_track_id_at(
    _ requestPtr: UnsafeMutableRawPointer,
    _ index: Int,
    _ outTrackID: UnsafeMutablePointer<Int32>?
) -> Bool {
    let handle = Unmanaged<AVWRRustVideoCompositionRequest>
        .fromOpaque(requestPtr)
        .takeUnretainedValue()
    guard index >= 0, index < handle.request.sourceSampleDataTrackIDs.count else { return false }
    outTrackID?.pointee = handle.request.sourceSampleDataTrackIDs[index]
    return true
}

@_cdecl("av_video_composition_request_copy_source_frame")
public func av_video_composition_request_copy_source_frame(
    _ requestPtr: UnsafeMutableRawPointer,
    _ trackID: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    let handle = Unmanaged<AVWRRustVideoCompositionRequest>
        .fromOpaque(requestPtr)
        .takeUnretainedValue()
    do {
        guard let frame = try handle.sourceFrame(trackID: trackID) else { return nil }
        return Unmanaged.passRetained(frame).toOpaque()
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_video_composition_request_copy_source_sample_buffer")
public func av_video_composition_request_copy_source_sample_buffer(
    _ requestPtr: UnsafeMutableRawPointer,
    _ trackID: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    let handle = Unmanaged<AVWRRustVideoCompositionRequest>
        .fromOpaque(requestPtr)
        .takeUnretainedValue()
    do {
        guard let sample = try handle.sourceSampleBuffer(trackID: trackID) else { return nil }
        return Unmanaged.passRetained(sample).toOpaque()
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_video_composition_request_new_output_pixel_buffer")
public func av_video_composition_request_new_output_pixel_buffer(
    _ requestPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    let handle = Unmanaged<AVWRRustVideoCompositionRequest>
        .fromOpaque(requestPtr)
        .takeUnretainedValue()
    do {
        return Unmanaged.passRetained(try handle.newOutputPixelBuffer()).toOpaque()
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_video_composition_request_finish_with_frame")
public func av_video_composition_request_finish_with_frame(
    _ requestPtr: UnsafeMutableRawPointer,
    _ framePtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let handle = Unmanaged<AVWRRustVideoCompositionRequest>
        .fromOpaque(requestPtr)
        .takeUnretainedValue()
    let frame = Unmanaged<CVPixelBuffer>.fromOpaque(framePtr).takeUnretainedValue()
    if let error = handle.finish(with: frame) {
        outErrorMessage?.pointee = ffiString(error)
        return AVW_INVALID_STATE
    }
    return AVW_OK
}

@_cdecl("av_video_composition_request_finish_with_error")
public func av_video_composition_request_finish_with_error(
    _ requestPtr: UnsafeMutableRawPointer,
    _ messagePtr: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let handle = Unmanaged<AVWRRustVideoCompositionRequest>
        .fromOpaque(requestPtr)
        .takeUnretainedValue()
    if let error = handle.finish(withErrorMessage: String(cString: messagePtr)) {
        outErrorMessage?.pointee = ffiString(error)
        return AVW_INVALID_STATE
    }
    return AVW_OK
}

@_cdecl("av_video_composition_request_cancel")
public func av_video_composition_request_cancel(
    _ requestPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let handle = Unmanaged<AVWRRustVideoCompositionRequest>
        .fromOpaque(requestPtr)
        .takeUnretainedValue()
    if let error = handle.cancel() {
        outErrorMessage?.pointee = ffiString(error)
        return AVW_INVALID_STATE
    }
    return AVW_OK
}

@_cdecl("av_video_compositor_release")
public func av_video_compositor_release(_ compositorPtr: UnsafeMutableRawPointer?) {
    guard let compositorPtr else { return }
    Unmanaged<NSObject>.fromOpaque(compositorPtr).release()
}
