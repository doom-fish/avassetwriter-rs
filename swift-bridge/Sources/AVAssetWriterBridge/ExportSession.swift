import AVFoundation
import CoreMedia
import CoreVideo
import Dispatch
import Foundation
import UniformTypeIdentifiers

private func exportAsset(from pathPtr: UnsafePointer<CChar>) -> AVURLAsset {
    AVURLAsset(url: URL(fileURLWithPath: String(cString: pathPtr)))
}

@_cdecl("av_export_session_all_presets_json")
public func av_export_session_all_presets_json() -> UnsafeMutablePointer<CChar>? {
    do {
        let presets = try AVAssetExportSession.allExportPresets().map { preset in
            guard let encoded = encodeExportPreset(preset) else {
                throw BridgeError.message("unknown AVAssetExportSession preset returned by the runtime: \(preset)")
            }
            return encoded
        }
        return ffiString(try encodeJson(presets))
    } catch {
        return nil
    }
}

@_cdecl("av_export_session_compatible_presets_json")
public func av_export_session_compatible_presets_json(
    _ pathPtr: UnsafePointer<CChar>
) -> UnsafeMutablePointer<CChar>? {
    do {
        let presets = try AVAssetExportSession.exportPresets(compatibleWith: exportAsset(from: pathPtr)).map { preset in
            guard let encoded = encodeExportPreset(preset) else {
                throw BridgeError.message("unknown compatible export preset returned by the runtime: \(preset)")
            }
            return encoded
        }
        return ffiString(try encodeJson(presets))
    } catch {
        return nil
    }
}

@_cdecl("av_export_session_determine_compatibility")
public func av_export_session_determine_compatibility(
    _ pathPtr: UnsafePointer<CChar>,
    _ presetPtr: UnsafePointer<CChar>,
    _ outputFileTypePtr: UnsafePointer<CChar>?,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let presetRaw = String(cString: presetPtr)
    guard let preset = decodeExportPreset(presetRaw) else {
        outErrorMessage?.pointee = ffiString("unknown export preset '\(presetRaw)'")
        return AVW_INVALID_ARGUMENT
    }
    let outputFileType: AVFileType?
    if let outputFileTypePtr {
        let fileTypeRaw = String(cString: outputFileTypePtr)
        guard let decoded = decodeFileType(fileTypeRaw) else {
            outErrorMessage?.pointee = ffiString("unknown output file type '\(fileTypeRaw)'")
            return AVW_INVALID_ARGUMENT
        }
        outputFileType = decoded
    } else {
        outputFileType = nil
    }
    let semaphore = DispatchSemaphore(value: 0)
    var compatible = false
    AVAssetExportSession.determineCompatibility(
        ofExportPreset: preset,
        with: exportAsset(from: pathPtr),
        outputFileType: outputFileType
    ) { isCompatible in
        compatible = isCompatible
        semaphore.signal()
    }
    semaphore.wait()
    return compatible ? 1 : 0
}

@_cdecl("av_export_session_create")
public func av_export_session_create(
    _ pathPtr: UnsafePointer<CChar>,
    _ presetPtr: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    let presetRaw = String(cString: presetPtr)
    guard let preset = decodeExportPreset(presetRaw) else {
        outErrorMessage?.pointee = ffiString("unknown export preset '\(presetRaw)'")
        return nil
    }
    guard let session = AVAssetExportSession(asset: exportAsset(from: pathPtr), presetName: preset) else {
        outErrorMessage?.pointee = ffiString("failed to create AVAssetExportSession for preset '\(presetRaw)'")
        return nil
    }
    return Unmanaged.passRetained(session).toOpaque()
}

@_cdecl("av_export_session_release")
public func av_export_session_release(_ sessionPtr: UnsafeMutableRawPointer?) {
    guard let sessionPtr else { return }
    Unmanaged<AVAssetExportSession>.fromOpaque(sessionPtr).release()
}

@_cdecl("av_export_session_info_json")
public func av_export_session_info_json(
    _ sessionPtr: UnsafeMutableRawPointer
) -> UnsafeMutablePointer<CChar>? {
    let session = Unmanaged<AVAssetExportSession>.fromOpaque(sessionPtr).takeUnretainedValue()
    do {
        return ffiString(try encodeJson(exportSessionInfoPayload(from: session)))
    } catch {
        return nil
    }
}

@_cdecl("av_export_session_set_output_file_type")
public func av_export_session_set_output_file_type(
    _ sessionPtr: UnsafeMutableRawPointer,
    _ outputFileTypePtr: UnsafePointer<CChar>?,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let session = Unmanaged<AVAssetExportSession>.fromOpaque(sessionPtr).takeUnretainedValue()
    if let outputFileTypePtr {
        let raw = String(cString: outputFileTypePtr)
        guard let fileType = decodeFileType(raw) else {
            outErrorMessage?.pointee = ffiString("unknown output file type '\(raw)'")
            return AVW_INVALID_ARGUMENT
        }
        guard session.supportedFileTypes.contains(fileType) else {
            outErrorMessage?.pointee = ffiString("output file type '\(raw)' is not supported by this export session")
            return AVW_INVALID_ARGUMENT
        }
        session.outputFileType = fileType
    } else {
        session.outputFileType = nil
    }
    return AVW_OK
}

@_cdecl("av_export_session_set_output_path")
public func av_export_session_set_output_path(
    _ sessionPtr: UnsafeMutableRawPointer,
    _ outputPathPtr: UnsafePointer<CChar>?,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let session = Unmanaged<AVAssetExportSession>.fromOpaque(sessionPtr).takeUnretainedValue()
    session.outputURL = outputPathPtr.map { URL(fileURLWithPath: String(cString: $0)) }
    _ = outErrorMessage
    return AVW_OK
}

@_cdecl("av_export_session_set_should_optimize_for_network_use")
public func av_export_session_set_should_optimize_for_network_use(
    _ sessionPtr: UnsafeMutableRawPointer,
    _ enabled: Bool,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let session = Unmanaged<AVAssetExportSession>.fromOpaque(sessionPtr).takeUnretainedValue()
    session.shouldOptimizeForNetworkUse = enabled
    _ = outErrorMessage
    return AVW_OK
}

@_cdecl("av_export_session_set_allows_parallelized_export")
public func av_export_session_set_allows_parallelized_export(
    _ sessionPtr: UnsafeMutableRawPointer,
    _ enabled: Bool,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let session = Unmanaged<AVAssetExportSession>.fromOpaque(sessionPtr).takeUnretainedValue()
    guard #available(macOS 14.0, *) else {
        outErrorMessage?.pointee = ffiString("parallelized export requires macOS 14+")
        return AVW_INVALID_STATE
    }
    session.allowsParallelizedExport = enabled
    return AVW_OK
}

@_cdecl("av_export_session_export")
public func av_export_session_export(
    _ sessionPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let session = Unmanaged<AVAssetExportSession>.fromOpaque(sessionPtr).takeUnretainedValue()
    guard let outputURL = session.outputURL else {
        outErrorMessage?.pointee = ffiString("export session outputURL must be set before export")
        return AVW_INVALID_STATE
    }
    guard session.outputFileType != nil else {
        outErrorMessage?.pointee = ffiString("export session outputFileType must be set before export")
        return AVW_INVALID_STATE
    }
    try? FileManager.default.removeItem(at: outputURL)
    let semaphore = DispatchSemaphore(value: 0)
    session.exportAsynchronously {
        semaphore.signal()
    }
    semaphore.wait()
    switch session.status {
    case .completed:
        return AVW_OK
    case .failed:
        outErrorMessage?.pointee = ffiString(session.error?.localizedDescription ?? "export session failed")
        return AVW_FINISH_FAILED
    case .cancelled:
        outErrorMessage?.pointee = ffiString("export session cancelled")
        return AVW_FINISH_FAILED
    default:
        outErrorMessage?.pointee = ffiString("export session finished in unexpected state \(session.status.rawValue)")
        return AVW_INVALID_STATE
    }
}

@_cdecl("av_export_session_cancel")
public func av_export_session_cancel(
    _ sessionPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let session = Unmanaged<AVAssetExportSession>.fromOpaque(sessionPtr).takeUnretainedValue()
    session.cancelExport()
    _ = outErrorMessage
    return AVW_OK
}

@_cdecl("av_export_session_compatible_file_types_json")
public func av_export_session_compatible_file_types_json(
    _ sessionPtr: UnsafeMutableRawPointer
) -> UnsafeMutablePointer<CChar>? {
    let session = Unmanaged<AVAssetExportSession>.fromOpaque(sessionPtr).takeUnretainedValue()
    let semaphore = DispatchSemaphore(value: 0)
    var compatibleFileTypes: [String] = []
    session.determineCompatibleFileTypes { fileTypes in
        compatibleFileTypes = fileTypes.compactMap(encodeFileType)
        semaphore.signal()
    }
    semaphore.wait()
    do {
        return ffiString(try encodeJson(compatibleFileTypes))
    } catch {
        return nil
    }
}

@_cdecl("av_export_session_set_time_range")
public func av_export_session_set_time_range(
    _ sessionPtr: UnsafeMutableRawPointer,
    _ startValue: Int64,
    _ startTimescale: Int32,
    _ startKind: Int32,
    _ durationValue: Int64,
    _ durationTimescale: Int32,
    _ durationKind: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let session = Unmanaged<AVAssetExportSession>.fromOpaque(sessionPtr).takeUnretainedValue()
    session.timeRange = CMTimeRange(
        start: cmTime(value: startValue, timescale: startTimescale, kind: startKind),
        duration: cmTime(value: durationValue, timescale: durationTimescale, kind: durationKind)
    )
    _ = outErrorMessage
    return AVW_OK
}

@_cdecl("av_export_session_set_file_length_limit")
public func av_export_session_set_file_length_limit(
    _ sessionPtr: UnsafeMutableRawPointer,
    _ limit: Int64,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let session = Unmanaged<AVAssetExportSession>.fromOpaque(sessionPtr).takeUnretainedValue()
    session.fileLengthLimit = limit
    _ = outErrorMessage
    return AVW_OK
}

@_cdecl("av_export_session_estimated_maximum_duration_json")
public func av_export_session_estimated_maximum_duration_json(
    _ sessionPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutablePointer<CChar>? {
    let session = Unmanaged<AVAssetExportSession>.fromOpaque(sessionPtr).takeUnretainedValue()
    let semaphore = DispatchSemaphore(value: 0)
    var estimatedDuration: CMTime = .invalid
    var exportError: Error?
    Task {
        do {
            estimatedDuration = try await session.estimatedMaximumDuration
        } catch {
            exportError = error
        }
        semaphore.signal()
    }
    semaphore.wait()
    if let exportError {
        outErrorMessage?.pointee = ffiString(exportError.localizedDescription)
        return nil
    }
    do {
        return ffiString(try encodeJson(encodeTime(estimatedDuration)))
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_export_session_estimated_output_file_length")
public func av_export_session_estimated_output_file_length(
    _ sessionPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int64 {
    let session = Unmanaged<AVAssetExportSession>.fromOpaque(sessionPtr).takeUnretainedValue()
    let semaphore = DispatchSemaphore(value: 0)
    var estimatedLength: Int64 = 0
    var exportError: Error?
    Task {
        do {
            estimatedLength = try await session.estimatedOutputFileLengthInBytes
        } catch {
            exportError = error
        }
        semaphore.signal()
    }
    semaphore.wait()
    if let exportError {
        outErrorMessage?.pointee = ffiString(exportError.localizedDescription)
        return Int64.min
    }
    return estimatedLength
}

@_cdecl("av_export_session_set_metadata_json")
public func av_export_session_set_metadata_json(
    _ sessionPtr: UnsafeMutableRawPointer,
    _ metadataJson: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let session = Unmanaged<AVAssetExportSession>.fromOpaque(sessionPtr).takeUnretainedValue()
    do {
        let payload = try decodeJson(metadataJson, as: [MetadataItemPayload].self)
        session.metadata = try payload.map(avMetadataItem)
        return AVW_OK
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return AVW_INVALID_ARGUMENT
    }
}

@_cdecl("av_export_session_set_can_perform_multiple_passes_over_source_media_data")
public func av_export_session_set_can_perform_multiple_passes_over_source_media_data(
    _ sessionPtr: UnsafeMutableRawPointer,
    _ enabled: Bool,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let session = Unmanaged<AVAssetExportSession>.fromOpaque(sessionPtr).takeUnretainedValue()
    session.canPerformMultiplePassesOverSourceMediaData = enabled
    _ = outErrorMessage
    return AVW_OK
}

@_cdecl("av_export_session_set_directory_for_temporary_files")
public func av_export_session_set_directory_for_temporary_files(
    _ sessionPtr: UnsafeMutableRawPointer,
    _ pathPtr: UnsafePointer<CChar>?,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let session = Unmanaged<AVAssetExportSession>.fromOpaque(sessionPtr).takeUnretainedValue()
    session.directoryForTemporaryFiles = pathPtr.map { URL(fileURLWithPath: String(cString: $0)) }
    _ = outErrorMessage
    return AVW_OK
}

@_cdecl("av_export_session_set_audio_track_group_handling")
public func av_export_session_set_audio_track_group_handling(
    _ sessionPtr: UnsafeMutableRawPointer,
    _ handlingRaw: UInt64,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let session = Unmanaged<AVAssetExportSession>.fromOpaque(sessionPtr).takeUnretainedValue()
    session.audioTrackGroupHandling = AVAssetTrackGroupOutputHandling(rawValue: UInt(handlingRaw))
    _ = outErrorMessage
    return AVW_OK
}
