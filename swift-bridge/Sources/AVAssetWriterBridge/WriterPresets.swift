import AVAssetWriterObjCBridge
import AVFoundation
import CoreMedia
import CoreVideo
import Dispatch
import Foundation
import UniformTypeIdentifiers

// MARK: - Output settings presets (v0.4)

/// Create a video input from one of Apple's named output-settings presets.
@_cdecl("av_writer_add_video_input_from_preset")
public func av_writer_add_video_input_from_preset(
    _ writerPtr: UnsafeMutableRawPointer,
    _ presetPtr: UnsafePointer<AVWString>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let presetRaw = String(abi: presetPtr)
    guard let preset = decodeOutputSettingsPreset(presetRaw) else {
        outErrorMessage?.pointee = makePayload("unknown output settings preset '\(presetRaw)'")
        return AVW_INVALID_ARGUMENT
    }
    guard let assistant = AVOutputSettingsAssistant(preset: preset),
          let videoSettings = assistant.videoSettings else {
        outErrorMessage?.pointee = makePayload("AVOutputSettingsAssistant returned nil for preset")
        return AVW_INVALID_ARGUMENT
    }
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    return wrapper.addInput(
        mediaType: .video,
        outputSettings: videoSettings,
        sourceFormatHint: nil,
        expectsMediaDataInRealTime: true,
        outErrorMessage
    )
}

// MARK: - Input groups + writer options (v0.5)

@_cdecl("av_writer_set_should_optimize_for_network_use")
public func av_writer_set_should_optimize_for_network_use(
    _ writerPtr: UnsafeMutableRawPointer,
    _ shouldOptimize: Bool,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.configurationStatus("shouldOptimizeForNetworkUse", outErrorMessage) {
        return status
    }
    wrapper.writer.shouldOptimizeForNetworkUse = shouldOptimize
    return AVW_OK
}

@_cdecl("av_writer_set_movie_fragment_interval_seconds")
public func av_writer_set_movie_fragment_interval_seconds(
    _ writerPtr: UnsafeMutableRawPointer,
    _ seconds: Double,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.configurationStatus("movieFragmentInterval", outErrorMessage) {
        return status
    }
    guard !seconds.isNaN else {
        outErrorMessage?.pointee = makePayload("movie fragment interval must not be NaN")
        return AVW_INVALID_ARGUMENT
    }
    guard seconds > 0 else {
        wrapper.writer.movieFragmentInterval = .invalid
        return AVW_OK
    }
    let interval = CMTime(seconds: seconds, preferredTimescale: 600)
    guard interval.isNumeric else {
        outErrorMessage?.pointee = makePayload("movie fragment interval \(seconds)s is out of range")
        return AVW_INVALID_ARGUMENT
    }
    wrapper.writer.movieFragmentInterval = interval
    return AVW_OK
}

/// Group input ids that should be mutually exclusive (e.g. multiple
/// audio tracks where only one plays at a time). Pass an int32[]
/// array of input ids + count. Returns true on success.
@_cdecl("av_writer_add_input_group")
public func av_writer_add_input_group(
    _ writerPtr: UnsafeMutableRawPointer,
    _ ids: UnsafePointer<Int32>,
    _ count: Int,
    _ defaultId: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Bool {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if wrapper.configurationStatus("input groups", outErrorMessage) != nil {
        return false
    }
    guard count > 0 else {
        outErrorMessage?.pointee = makePayload("an input group needs at least one input")
        return false
    }
    var inputs: [AVAssetWriterInput] = []
    for i in 0..<count {
        let id = ids[i]
        guard Int(id) >= 0, Int(id) < wrapper.inputs.count else {
            outErrorMessage?.pointee = makePayload("input id \(id) out of range")
            return false
        }
        inputs.append(wrapper.inputs[Int(id)])
    }
    guard Set(inputs.map(ObjectIdentifier.init)).count == inputs.count else {
        outErrorMessage?.pointee = makePayload("an input group cannot list an input twice")
        return false
    }
    let defaultInput: AVAssetWriterInput? =
        defaultId >= 0 && Int(defaultId) < wrapper.inputs.count
        ? wrapper.inputs[Int(defaultId)]
        : nil
    if let defaultInput, !inputs.contains(where: { $0 === defaultInput }) {
        outErrorMessage?.pointee = makePayload("default input \(defaultId) is not in the group")
        return false
    }
    let group = AVAssetWriterInputGroup(inputs: inputs, defaultInput: defaultInput)
    var message: NSString?
    guard AVWXAddInputGroup(wrapper.writer, group, &message) else {
        outErrorMessage?.pointee = makePayload((message as String?) ?? "writer cannot add input group")
        return false
    }
    wrapper.inputGroups.append(group)
    return true
}
