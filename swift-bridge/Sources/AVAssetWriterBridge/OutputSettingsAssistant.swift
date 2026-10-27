import AVAssetWriterObjCBridge
import AVFoundation
import CoreMedia
import CoreVideo
import Dispatch
import Foundation
import UniformTypeIdentifiers

@_cdecl("av_output_settings_assistant_available_presets_payload")
public func av_output_settings_assistant_available_presets_payload() -> UnsafeMutablePointer<AVWOwnedPayload>? {
    do {
        let presets = try AVOutputSettingsAssistant.availableOutputSettingsPresets().map { preset in
            guard let encoded = encodeOutputSettingsPreset(preset) else {
                throw BridgeError.message("unknown AVOutputSettingsPreset returned by the runtime: \(preset)")
            }
            return encoded
        }
        return makePayload(presets)
    } catch {
        return nil
    }
}

@_cdecl("av_output_settings_assistant_create")
public func av_output_settings_assistant_create(
    _ presetPtr: UnsafePointer<AVWString>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> UnsafeMutableRawPointer? {
    let presetRaw = String(abi: presetPtr)
    guard let preset = decodeOutputSettingsPreset(presetRaw) else {
        outErrorMessage?.pointee = makePayload("unknown output settings preset '\(presetRaw)'")
        return nil
    }
    guard let assistant = AVOutputSettingsAssistant(preset: preset) else {
        outErrorMessage?.pointee = makePayload("failed to create AVOutputSettingsAssistant for preset '\(presetRaw)'")
        return nil
    }
    return Unmanaged.passRetained(assistant).toOpaque()
}

@_cdecl("av_output_settings_assistant_release")
public func av_output_settings_assistant_release(_ assistantPtr: UnsafeMutableRawPointer?) {
    guard let assistantPtr else { return }
    Unmanaged<AVOutputSettingsAssistant>.fromOpaque(assistantPtr).release()
}

@_cdecl("av_output_settings_assistant_info_payload")
public func av_output_settings_assistant_info_payload(
    _ assistantPtr: UnsafeMutableRawPointer
) -> UnsafeMutablePointer<AVWOwnedPayload>? {
    let assistant = Unmanaged<AVOutputSettingsAssistant>.fromOpaque(assistantPtr).takeUnretainedValue()
    do {
        return makePayload(outputSettingsAssistantInfoPayload(from: assistant))
    } catch {
        return nil
    }
}

@_cdecl("av_output_settings_assistant_source_audio_format")
public func av_output_settings_assistant_source_audio_format(
    _ assistantPtr: UnsafeMutableRawPointer
) -> UnsafeMutableRawPointer? {
    let assistant = Unmanaged<AVOutputSettingsAssistant>.fromOpaque(assistantPtr).takeUnretainedValue()
    guard let format = assistant.sourceAudioFormat else { return nil }
    return Unmanaged.passRetained(format).toOpaque()
}

@_cdecl("av_output_settings_assistant_set_source_audio_format")
public func av_output_settings_assistant_set_source_audio_format(
    _ assistantPtr: UnsafeMutableRawPointer,
    _ formatPtr: UnsafeMutableRawPointer?,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let assistant = Unmanaged<AVOutputSettingsAssistant>.fromOpaque(assistantPtr).takeUnretainedValue()
    assistant.sourceAudioFormat = formatPtr.map {
        Unmanaged<CMFormatDescription>.fromOpaque($0).takeUnretainedValue()
    }
    _ = outErrorMessage
    return AVW_OK
}

@_cdecl("av_output_settings_assistant_source_video_format")
public func av_output_settings_assistant_source_video_format(
    _ assistantPtr: UnsafeMutableRawPointer
) -> UnsafeMutableRawPointer? {
    let assistant = Unmanaged<AVOutputSettingsAssistant>.fromOpaque(assistantPtr).takeUnretainedValue()
    guard let format = assistant.sourceVideoFormat else { return nil }
    return Unmanaged.passRetained(format).toOpaque()
}

@_cdecl("av_output_settings_assistant_set_source_video_format")
public func av_output_settings_assistant_set_source_video_format(
    _ assistantPtr: UnsafeMutableRawPointer,
    _ formatPtr: UnsafeMutableRawPointer?,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let assistant = Unmanaged<AVOutputSettingsAssistant>.fromOpaque(assistantPtr).takeUnretainedValue()
    assistant.sourceVideoFormat = formatPtr.map {
        Unmanaged<CMFormatDescription>.fromOpaque($0).takeUnretainedValue()
    }
    _ = outErrorMessage
    return AVW_OK
}

@_cdecl("av_output_settings_assistant_set_source_video_average_frame_duration")
public func av_output_settings_assistant_set_source_video_average_frame_duration(
    _ assistantPtr: UnsafeMutableRawPointer,
    _ value: Int64,
    _ timescale: Int32,
    _ kind: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let assistant = Unmanaged<AVOutputSettingsAssistant>.fromOpaque(assistantPtr).takeUnretainedValue()
    assistant.sourceVideoAverageFrameDuration = cmTime(value: value, timescale: timescale, kind: kind)
    _ = outErrorMessage
    return AVW_OK
}

@_cdecl("av_output_settings_assistant_set_source_video_min_frame_duration")
public func av_output_settings_assistant_set_source_video_min_frame_duration(
    _ assistantPtr: UnsafeMutableRawPointer,
    _ value: Int64,
    _ timescale: Int32,
    _ kind: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let assistant = Unmanaged<AVOutputSettingsAssistant>.fromOpaque(assistantPtr).takeUnretainedValue()
    assistant.sourceVideoMinFrameDuration = cmTime(value: value, timescale: timescale, kind: kind)
    _ = outErrorMessage
    return AVW_OK
}
