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
    _ presetPtr: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let presetRaw = String(cString: presetPtr)
    guard let preset = decodeOutputSettingsPreset(presetRaw) else {
        outErrorMessage?.pointee = ffiString("unknown output settings preset '\(presetRaw)'")
        return AVW_INVALID_ARGUMENT
    }
    guard let assistant = AVOutputSettingsAssistant(preset: preset),
          let videoSettings = assistant.videoSettings else {
        outErrorMessage?.pointee = ffiString("AVOutputSettingsAssistant returned nil for preset")
        return AVW_INVALID_ARGUMENT
    }
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    let input = AVAssetWriterInput(mediaType: .video, outputSettings: videoSettings)
    input.expectsMediaDataInRealTime = true
    if !wrapper.writer.canAdd(input) {
        outErrorMessage?.pointee = ffiString("writer cannot add video input (status=\(wrapper.writer.status.rawValue))")
        return AVW_INVALID_STATE
    }
    wrapper.writer.add(input)
    let id = Int32(wrapper.inputs.count)
    wrapper.inputs.append(input)
    return id
}

// MARK: - Input groups + writer options (v0.5)

@_cdecl("av_writer_set_should_optimize_for_network_use")
public func av_writer_set_should_optimize_for_network_use(
    _ writerPtr: UnsafeMutableRawPointer,
    _ shouldOptimize: Bool
) {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    wrapper.writer.shouldOptimizeForNetworkUse = shouldOptimize
}

@_cdecl("av_writer_set_movie_fragment_interval_seconds")
public func av_writer_set_movie_fragment_interval_seconds(
    _ writerPtr: UnsafeMutableRawPointer,
    _ seconds: Double
) {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    if seconds > 0 {
        wrapper.writer.movieFragmentInterval = CMTime(seconds: seconds, preferredTimescale: 600)
    } else {
        wrapper.writer.movieFragmentInterval = .invalid
    }
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
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Bool {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    var inputs: [AVAssetWriterInput] = []
    for i in 0..<count {
        let id = ids[i]
        guard Int(id) >= 0, Int(id) < wrapper.inputs.count else {
            outErrorMessage?.pointee = ffiString("input id \(id) out of range")
            return false
        }
        inputs.append(wrapper.inputs[Int(id)])
    }
    let defaultInput: AVAssetWriterInput? =
        defaultId >= 0 && Int(defaultId) < wrapper.inputs.count
        ? wrapper.inputs[Int(defaultId)]
        : nil
    let group = AVAssetWriterInputGroup(inputs: inputs, defaultInput: defaultInput)
    if !wrapper.writer.canAdd(group) {
        outErrorMessage?.pointee = ffiString("writer cannot add input group")
        return false
    }
    wrapper.writer.add(group)
    wrapper.inputGroups.append(group)
    return true
}
