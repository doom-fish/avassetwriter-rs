import AVAssetWriterObjCBridge
import AVFoundation
import CoreMedia
import CoreVideo
import Foundation
import UniformTypeIdentifiers

final class SegmentDelegate: NSObject, AVAssetWriterDelegate {
    weak var writerBox: Writer?
    let box: SegmentCallbackBox

    init(writerBox: Writer, box: SegmentCallbackBox) {
        self.writerBox = writerBox
        self.box = box
    }

    func assetWriter(
        _ writer: AVAssetWriter,
        didOutputSegmentData segmentData: Data,
        segmentType: AVAssetSegmentType,
        segmentReport: AVAssetSegmentReport?
    ) {
        guard let writerBox else { return }
        writerBox.invokeCallback(box.lifetime) { userdata in
            box.emit(
                data: segmentData,
                segmentType: Int32(segmentType.rawValue),
                report: segmentReport.map(encodeSegmentReport),
                userdata: userdata
            )
        }
    }

    func assetWriter(
        _ writer: AVAssetWriter,
        didOutputSegmentData segmentData: Data,
        segmentType: AVAssetSegmentType
    ) {
        guard let writerBox else { return }
        writerBox.invokeCallback(box.lifetime) { userdata in
            box.emit(
                data: segmentData,
                segmentType: Int32(segmentType.rawValue),
                report: nil,
                userdata: userdata
            )
        }
    }
}

@_cdecl("av_writer_create_segmented")
public func av_writer_create_segmented(
    _ fileType: UnsafePointer<CChar>,
    _ profile: UnsafePointer<CChar>?,
    _ callback: AVWSegmentCallback?,
    _ userdata: UnsafeMutableRawPointer?,
    _ dropUserdata: AVWDropCallback?,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    let fileTypeRaw = String(cString: fileType)
    guard fileTypeRaw == "mp4" else {
        outErrorMessage?.pointee = ffiString("segmented output currently supports mp4 only")
        return nil
    }
    guard let outputType = UTType(filenameExtension: "mp4") else {
        outErrorMessage?.pointee = ffiString("failed to resolve UTType for mp4")
        return nil
    }
    let writer = AVAssetWriter(contentType: outputType)
    let wrapper = Writer(writer: writer)
    if let profileRaw = profile {
        let profileString = String(cString: profileRaw)
        wrapper.writer.outputFileTypeProfile = decodeFileTypeProfile(profileString)
    }
    let box = SegmentCallbackBox(callback: callback, userdata: userdata, dropUserdata: dropUserdata)
    let delegate = SegmentDelegate(writerBox: wrapper, box: box)
    wrapper.segmentCallbackBox = box
    wrapper.segmentDelegate = delegate
    wrapper.writer.delegate = delegate
    _ = outErrorMessage
    return Unmanaged.passRetained(wrapper).toOpaque()
}

@_cdecl("av_writer_info_json")
public func av_writer_info_json(_ writerPtr: UnsafeMutableRawPointer) -> UnsafeMutablePointer<CChar>? {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    do {
        return ffiString(try encodeJson(writerInfoPayload(from: wrapper)))
    } catch {
        return nil
    }
}

@_cdecl("av_writer_metadata")
public func av_writer_metadata(
    _ writerPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    _ = outErrorMessage
    return retainedMetadataItemArray(wrapper.writer.metadata)
}

@_cdecl("av_writer_set_metadata_items")
public func av_writer_set_metadata_items(
    _ writerPtr: UnsafeMutableRawPointer,
    _ itemPointers: UnsafePointer<UnsafeMutableRawPointer?>?,
    _ itemCount: Int,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.configurationStatus("metadata", outErrorMessage) {
        return status
    }
    do {
        wrapper.writer.metadata = try metadataItems(from: itemPointers, count: itemCount)
        return AVW_OK
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return AVW_INVALID_ARGUMENT
    }
}

@_cdecl("av_writer_set_metadata_json")
public func av_writer_set_metadata_json(
    _ writerPtr: UnsafeMutableRawPointer,
    _ metadataJson: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.configurationStatus("metadata", outErrorMessage) {
        return status
    }
    do {
        let payload = try decodeJson(metadataJson, as: [MetadataItemPayload].self)
        wrapper.writer.metadata = try payload.map(avMetadataItem)
        return AVW_OK
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return AVW_INVALID_ARGUMENT
    }
}

@_cdecl("av_writer_set_directory_for_temporary_files")
public func av_writer_set_directory_for_temporary_files(
    _ writerPtr: UnsafeMutableRawPointer,
    _ directoryPath: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.configurationStatus("directoryForTemporaryFiles", outErrorMessage) {
        return status
    }
    wrapper.writer.directoryForTemporaryFiles = URL(fileURLWithPath: String(cString: directoryPath))
    return AVW_OK
}

@_cdecl("av_writer_can_apply_output_settings_json")
public func av_writer_can_apply_output_settings_json(
    _ writerPtr: UnsafeMutableRawPointer,
    _ mediaType: UnsafePointer<CChar>,
    _ outputSettingsJson: UnsafePointer<CChar>?,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    let mediaTypeString = String(cString: mediaType)
    guard let mediaType = decodeMediaType(mediaTypeString) else {
        outErrorMessage?.pointee = ffiString("unknown media type: \(mediaTypeString)")
        return AVW_INVALID_ARGUMENT
    }
    do {
        let outputSettings = try jsonObjectFromCString(outputSettingsJson) as? [String: Any]
        return wrapper.writer.canApply(outputSettings: outputSettings, forMediaType: mediaType) ? 1 : 0
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return AVW_INVALID_ARGUMENT
    }
}

@_cdecl("av_writer_add_input_json")
public func av_writer_add_input_json(
    _ writerPtr: UnsafeMutableRawPointer,
    _ mediaTypePtr: UnsafePointer<CChar>,
    _ outputSettingsJson: UnsafePointer<CChar>?,
    _ sourceFormatHintPtr: UnsafeMutableRawPointer?,
    _ expectsMediaDataInRealTime: Bool,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    let mediaTypeString = String(cString: mediaTypePtr)
    guard let mediaType = decodeMediaType(mediaTypeString) else {
        outErrorMessage?.pointee = ffiString("unknown media type: \(mediaTypeString)")
        return AVW_INVALID_ARGUMENT
    }
    do {
        let outputSettings = try jsonObjectFromCString(outputSettingsJson) as? [String: Any]
        let sourceFormatHint = sourceFormatHintPtr.map {
            Unmanaged<CMFormatDescription>.fromOpaque($0).takeUnretainedValue()
        }
        return wrapper.addInput(
            mediaType: mediaType,
            outputSettings: outputSettings,
            sourceFormatHint: sourceFormatHint,
            expectsMediaDataInRealTime: expectsMediaDataInRealTime,
            outErrorMessage
        )
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return AVW_INVALID_ARGUMENT
    }
}

@_cdecl("av_writer_add_audio_input_from_sample")
public func av_writer_add_audio_input_from_sample(
    _ writerPtr: UnsafeMutableRawPointer,
    _ sampleBufferPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    let sampleBuffer = Unmanaged<CMSampleBuffer>.fromOpaque(sampleBufferPtr).takeUnretainedValue()
    guard let format = CMSampleBufferGetFormatDescription(sampleBuffer) else {
        outErrorMessage?.pointee = ffiString("sample buffer has no format description")
        return AVW_INVALID_ARGUMENT
    }
    if let status = formatHintStatus(format, expected: .audio, outErrorMessage) {
        return status
    }
    return wrapper.addInput(
        mediaType: .audio,
        outputSettings: nil,
        sourceFormatHint: format,
        expectsMediaDataInRealTime: true,
        outErrorMessage
    )
}

@_cdecl("av_writer_add_metadata_input_from_specifications_json")
public func av_writer_add_metadata_input_from_specifications_json(
    _ writerPtr: UnsafeMutableRawPointer,
    _ specificationsJson: UnsafePointer<CChar>,
    _ expectsMediaDataInRealTime: Bool,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.configurationStatus("inputs", outErrorMessage) {
        return status
    }
    do {
        let specifications = try decodeJson(specificationsJson, as: [MetadataSpecificationPayload].self)
        let formatDescription = try metadataFormatDescription(from: specifications)
        guard let input = wrapper.makeInput(
            mediaType: .metadata,
            outputSettings: nil,
            sourceFormatHint: formatDescription,
            outErrorMessage
        ) else {
            return AVW_INVALID_ARGUMENT
        }
        input.expectsMediaDataInRealTime = expectsMediaDataInRealTime
        var message: NSString?
        guard let adaptor = AVWXMakeMetadataAdaptor(input, &message) else {
            outErrorMessage?.pointee = ffiString((message as String?) ?? "metadata adaptor rejected the input")
            return AVW_INVALID_ARGUMENT
        }
        let inputId = wrapper.addInput(input, outErrorMessage)
        guard inputId >= 0 else {
            return inputId
        }
        wrapper.metadataAdaptors[inputId] = adaptor
        wrapper.adaptedInputIds.insert(inputId)
        return inputId
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return AVW_INVALID_ARGUMENT
    }
}

@_cdecl("av_writer_end_session")
public func av_writer_end_session(
    _ writerPtr: UnsafeMutableRawPointer,
    _ timeValue: Int64,
    _ timeScale: Int32,
    _ timeKind: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    guard wrapper.writer.status == .writing, wrapper.sessionStarted else {
        outErrorMessage?.pointee = ffiString("end_session requires a started sample-writing session")
        return AVW_INVALID_STATE
    }
    guard !wrapper.sessionEnded else {
        outErrorMessage?.pointee = ffiString("the sample-writing session has already ended")
        return AVW_INVALID_STATE
    }
    guard timeKind == 0, timeScale > 0 else {
        outErrorMessage?.pointee = ffiString(
            "end_session requires a numeric time with a positive timescale"
        )
        return AVW_INVALID_ARGUMENT
    }
    let endTime = cmTime(value: timeValue, timescale: timeScale, kind: timeKind)
    guard wrapper.runBlockingNativeOperation({
        wrapper.writer.endSession(atSourceTime: endTime)
    }) else {
        outErrorMessage?.pointee = ffiString(
            "end_session cannot run reentrantly or while another blocking writer operation is active"
        )
        return AVW_INVALID_STATE
    }
    wrapper.sessionEnded = true
    return AVW_OK
}

@_cdecl("av_writer_cancel")
public func av_writer_cancel(
    _ writerPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    wrapper.writer.cancelWriting()
    wrapper.detachCallbacksAfterTerminalState()
    return AVW_OK
}

@_cdecl("av_writer_remove_segment_callback")
public func av_writer_remove_segment_callback(
    _ writerPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    wrapper.detachSegmentCallback()
    _ = outErrorMessage
    return AVW_OK
}
