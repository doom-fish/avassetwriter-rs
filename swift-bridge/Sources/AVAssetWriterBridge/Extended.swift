import AVFoundation
import CoreMedia
import CoreVideo
import Foundation
import UniformTypeIdentifiers

final class SegmentDelegate: NSObject, AVAssetWriterDelegate {
    let box: SegmentCallbackBox

    init(box: SegmentCallbackBox) {
        self.box = box
    }

    func assetWriter(
        _ writer: AVAssetWriter,
        didOutputSegmentData segmentData: Data,
        segmentType: AVAssetSegmentType,
        segmentReport: AVAssetSegmentReport?
    ) {
        box.emit(
            data: segmentData,
            segmentType: Int32(segmentType.rawValue),
            report: segmentReport.map(encodeSegmentReport)
        )
    }

    func assetWriter(
        _ writer: AVAssetWriter,
        didOutputSegmentData segmentData: Data,
        segmentType: AVAssetSegmentType
    ) {
        box.emit(data: segmentData, segmentType: Int32(segmentType.rawValue), report: nil)
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
    let delegate = SegmentDelegate(box: box)
    wrapper.segmentCallbackBox = box
    wrapper.segmentDelegate = delegate
    wrapper.writer.delegate = delegate
    _ = outErrorMessage
    return Unmanaged.passRetained(wrapper).toOpaque()
}

@_cdecl("av_writer_info_json")
public func av_writer_info_json(_ writerPtr: UnsafeMutableRawPointer) -> UnsafeMutablePointer<CChar>? {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    do {
        return ffiString(try encodeJson(writerInfoPayload(from: wrapper)))
    } catch {
        return nil
    }
}

@_cdecl("av_writer_set_metadata_json")
public func av_writer_set_metadata_json(
    _ writerPtr: UnsafeMutableRawPointer,
    _ metadataJson: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
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
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
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
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
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
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    let mediaTypeString = String(cString: mediaTypePtr)
    guard let mediaType = decodeMediaType(mediaTypeString) else {
        outErrorMessage?.pointee = ffiString("unknown media type: \(mediaTypeString)")
        return AVW_INVALID_ARGUMENT
    }
    do {
        let outputSettings = try jsonObjectFromCString(outputSettingsJson) as? [String: Any]
        let input: AVAssetWriterInput
        if let sourceFormatHintPtr {
            let sourceFormatHint = Unmanaged<CMFormatDescription>.fromOpaque(sourceFormatHintPtr).takeUnretainedValue()
            input = AVAssetWriterInput(mediaType: mediaType, outputSettings: outputSettings, sourceFormatHint: sourceFormatHint)
        } else {
            input = AVAssetWriterInput(mediaType: mediaType, outputSettings: outputSettings)
        }
        input.expectsMediaDataInRealTime = expectsMediaDataInRealTime
        if !wrapper.writer.canAdd(input) {
            outErrorMessage?.pointee = ffiString("writer cannot add input (status=\(wrapper.writer.status.rawValue))")
            return AVW_INVALID_STATE
        }
        wrapper.writer.add(input)
        let id = Int32(wrapper.inputs.count)
        wrapper.inputs.append(input)
        return id
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
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    let sampleBuffer = Unmanaged<CMSampleBuffer>.fromOpaque(sampleBufferPtr).takeUnretainedValue()
    guard let format = CMSampleBufferGetFormatDescription(sampleBuffer) else {
        outErrorMessage?.pointee = ffiString("sample buffer has no format description")
        return AVW_INVALID_ARGUMENT
    }
    let input = AVAssetWriterInput(mediaType: .audio, outputSettings: nil, sourceFormatHint: format)
    input.expectsMediaDataInRealTime = true
    if !wrapper.writer.canAdd(input) {
        outErrorMessage?.pointee = ffiString("writer cannot add audio input (status=\(wrapper.writer.status.rawValue))")
        return AVW_INVALID_STATE
    }
    wrapper.writer.add(input)
    let id = Int32(wrapper.inputs.count)
    wrapper.inputs.append(input)
    return id
}

@_cdecl("av_writer_add_metadata_input_from_specifications_json")
public func av_writer_add_metadata_input_from_specifications_json(
    _ writerPtr: UnsafeMutableRawPointer,
    _ specificationsJson: UnsafePointer<CChar>,
    _ expectsMediaDataInRealTime: Bool,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    do {
        let specifications = try decodeJson(specificationsJson, as: [MetadataSpecificationPayload].self)
        let formatDescription = try metadataFormatDescription(from: specifications)
        let input = AVAssetWriterInput(mediaType: .metadata, outputSettings: nil, sourceFormatHint: formatDescription)
        input.expectsMediaDataInRealTime = expectsMediaDataInRealTime
        if !wrapper.writer.canAdd(input) {
            outErrorMessage?.pointee = ffiString("writer cannot add metadata input (status=\(wrapper.writer.status.rawValue))")
            return AVW_INVALID_STATE
        }
        wrapper.writer.add(input)
        let inputId = Int32(wrapper.inputs.count)
        wrapper.inputs.append(input)
        wrapper.metadataAdaptors[inputId] = AVAssetWriterInputMetadataAdaptor(assetWriterInput: input)
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
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    wrapper.writer.endSession(atSourceTime: cmTime(value: timeValue, timescale: timeScale, kind: timeKind))
    return AVW_OK
}

@_cdecl("av_writer_cancel")
public func av_writer_cancel(
    _ writerPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    wrapper.writer.cancelWriting()
    return AVW_OK
}

