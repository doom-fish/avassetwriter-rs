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
    do {
        let writer = try AVAssetWriter(contentType: outputType)
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
        return Unmanaged.passRetained(wrapper).toOpaque()
    } catch {
        outErrorMessage?.pointee = ffiString("AVAssetWriter segmented init failed: \(error.localizedDescription)")
        return nil
    }
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

@_cdecl("av_writer_attach_pixel_buffer_adaptor_json")
public func av_writer_attach_pixel_buffer_adaptor_json(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ attributesJson: UnsafePointer<CChar>?,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = ffiString("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    do {
        let attributes = try jsonObjectFromCString(attributesJson) as? [String: Any]
        let adaptor = AVAssetWriterInputPixelBufferAdaptor(
            assetWriterInput: wrapper.inputs[Int(inputId)],
            sourcePixelBufferAttributes: attributes
        )
        wrapper.pixelBufferAdaptors[inputId] = adaptor
        return AVW_OK
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return AVW_INVALID_ARGUMENT
    }
}

@_cdecl("av_writer_pixel_buffer_pool")
public func av_writer_pixel_buffer_pool(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32
) -> UnsafeMutableRawPointer? {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard let pool = wrapper.pixelBufferAdaptors[inputId]?.pixelBufferPool else {
        return nil
    }
    return Unmanaged.passRetained(pool).toOpaque()
}

@_cdecl("av_writer_attach_tagged_pixel_buffer_group_adaptor_json")
public func av_writer_attach_tagged_pixel_buffer_group_adaptor_json(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ attributesJson: UnsafePointer<CChar>?,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    guard #available(macOS 14.0, *) else {
        outErrorMessage?.pointee = ffiString("tagged pixel buffer group adaptor requires macOS 14+")
        return AVW_INVALID_STATE
    }
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = ffiString("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    do {
        let attributes = try jsonObjectFromCString(attributesJson) as? [String: Any]
        let adaptor = AVAssetWriterInputTaggedPixelBufferGroupAdaptor(
            assetWriterInput: wrapper.inputs[Int(inputId)],
            sourcePixelBufferAttributes: attributes
        )
        wrapper.taggedPixelBufferGroupAdaptors[inputId] = adaptor
        return AVW_OK
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return AVW_INVALID_ARGUMENT
    }
}

@_cdecl("av_writer_tagged_pixel_buffer_pool")
public func av_writer_tagged_pixel_buffer_pool(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32
) -> UnsafeMutableRawPointer? {
    guard #available(macOS 14.0, *) else { return nil }
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard let adaptor = wrapper.taggedPixelBufferGroupAdaptors[inputId] as? AVAssetWriterInputTaggedPixelBufferGroupAdaptor,
          let pool = adaptor.pixelBufferPool else {
        return nil
    }
    return Unmanaged.passRetained(pool).toOpaque()
}

@_cdecl("av_writer_append_tagged_pixel_buffer_group")
public func av_writer_append_tagged_pixel_buffer_group(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ pixelBuffers: UnsafePointer<UnsafeMutableRawPointer?>,
    _ layerIds: UnsafePointer<Int64>,
    _ count: Int,
    _ ptsValue: Int64,
    _ ptsScale: Int32,
    _ ptsKind: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    guard #available(macOS 14.0, *) else {
        outErrorMessage?.pointee = ffiString("tagged pixel buffer group append requires macOS 14+")
        return AVW_INVALID_STATE
    }
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard let adaptor = wrapper.taggedPixelBufferGroupAdaptors[inputId] as? AVAssetWriterInputTaggedPixelBufferGroupAdaptor else {
        outErrorMessage?.pointee = ffiString("input \(inputId) has no tagged pixel buffer group adaptor")
        return AVW_INVALID_ARGUMENT
    }
    if !adaptor.assetWriterInput.isReadyForMoreMediaData {
        return AVW_INPUT_NOT_READY
    }
    outErrorMessage?.pointee = ffiString("tagged pixel buffer group append is not yet available in this build")
    _ = adaptor
    _ = pixelBuffers
    _ = layerIds
    _ = count
    _ = ptsValue
    _ = ptsScale
    _ = ptsKind
    return AVW_INVALID_STATE
}

@_cdecl("av_writer_attach_metadata_adaptor")
public func av_writer_attach_metadata_adaptor(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = ffiString("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    let input = wrapper.inputs[Int(inputId)]
    do {
        let adaptor = try AVAssetWriterInputMetadataAdaptor(assetWriterInput: input)
        wrapper.metadataAdaptors[inputId] = adaptor
        return AVW_OK
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return AVW_INVALID_ARGUMENT
    }
}

@_cdecl("av_writer_append_timed_metadata_group_json")
public func av_writer_append_timed_metadata_group_json(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ groupJson: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard let adaptor = wrapper.metadataAdaptors[inputId] else {
        outErrorMessage?.pointee = ffiString("input \(inputId) has no metadata adaptor")
        return AVW_INVALID_ARGUMENT
    }
    do {
        let payload = try decodeJson(groupJson, as: TimedMetadataGroupPayload.self)
        return adaptor.append(try timedMetadataGroup(from: payload)) ? AVW_OK : AVW_APPEND_FAILED
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return AVW_INVALID_ARGUMENT
    }
}

@_cdecl("av_writer_attach_caption_adaptor")
public func av_writer_attach_caption_adaptor(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = ffiString("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    let input = wrapper.inputs[Int(inputId)]
    do {
        let adaptor = try AVAssetWriterInputCaptionAdaptor(assetWriterInput: input)
        wrapper.captionAdaptors[inputId] = adaptor
        return AVW_OK
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return AVW_INVALID_ARGUMENT
    }
}

@_cdecl("av_writer_append_caption_json")
public func av_writer_append_caption_json(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ captionJson: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard let adaptor = wrapper.captionAdaptors[inputId] else {
        outErrorMessage?.pointee = ffiString("input \(inputId) has no caption adaptor")
        return AVW_INVALID_ARGUMENT
    }
    do {
        let payload = try decodeJson(captionJson, as: CaptionPayload.self)
        return adaptor.append(caption(from: payload)) ? AVW_OK : AVW_APPEND_FAILED
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return AVW_INVALID_ARGUMENT
    }
}

@_cdecl("av_writer_append_caption_group_json")
public func av_writer_append_caption_group_json(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ captionGroupJson: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard let adaptor = wrapper.captionAdaptors[inputId] else {
        outErrorMessage?.pointee = ffiString("input \(inputId) has no caption adaptor")
        return AVW_INVALID_ARGUMENT
    }
    do {
        let payload = try decodeJson(captionGroupJson, as: CaptionGroupPayload.self)
        return adaptor.append(captionGroup(from: payload)) ? AVW_OK : AVW_APPEND_FAILED
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return AVW_INVALID_ARGUMENT
    }
}

@_cdecl("av_writer_input_info_json")
public func av_writer_input_info_json(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32
) -> UnsafeMutablePointer<CChar>? {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    do {
        return ffiString(try encodeJson(inputInfoPayload(from: wrapper, inputId: inputId)))
    } catch {
        return nil
    }
}

@_cdecl("av_writer_input_source_format_hint")
public func av_writer_input_source_format_hint(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32
) -> UnsafeMutableRawPointer? {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else { return nil }
    guard let sourceFormatHint = wrapper.inputs[Int(inputId)].sourceFormatHint else { return nil }
    return Unmanaged.passRetained(sourceFormatHint).toOpaque()
}

@_cdecl("av_writer_set_input_metadata_json")
public func av_writer_set_input_metadata_json(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ metadataJson: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = ffiString("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    do {
        let payload = try decodeJson(metadataJson, as: [MetadataItemPayload].self)
        wrapper.inputs[Int(inputId)].metadata = try payload.map(avMetadataItem)
        return AVW_OK
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return AVW_INVALID_ARGUMENT
    }
}

@_cdecl("av_writer_input_set_expects_media_data_in_real_time")
public func av_writer_input_set_expects_media_data_in_real_time(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ expectsMediaDataInRealTime: Bool,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = ffiString("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    wrapper.inputs[Int(inputId)].expectsMediaDataInRealTime = expectsMediaDataInRealTime
    return AVW_OK
}

@_cdecl("av_writer_input_request_media_data_when_ready")
public func av_writer_input_request_media_data_when_ready(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ callback: AVWReadyCallback?,
    _ userdata: UnsafeMutableRawPointer?,
    _ dropUserdata: AVWDropCallback?,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = ffiString("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    guard wrapper.readyCallbackBoxes[inputId] == nil else {
        outErrorMessage?.pointee = ffiString("requestMediaDataWhenReady already registered for input \(inputId)")
        return AVW_INVALID_STATE
    }
    let input = wrapper.inputs[Int(inputId)]
    let box = InputCallbackBox(userdata: userdata, dropUserdata: dropUserdata)
    wrapper.readyCallbackBoxes[inputId] = box
    let queue = DispatchQueue(label: "fish.doom.avassetwriter.input.ready.\(inputId)")
    input.requestMediaDataWhenReady(on: queue) {
        callback?(userdata)
    }
    return AVW_OK
}

@_cdecl("av_writer_input_mark_as_finished")
public func av_writer_input_mark_as_finished(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = ffiString("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    wrapper.inputs[Int(inputId)].markAsFinished()
    return AVW_OK
}

@_cdecl("av_writer_input_set_language_code")
public func av_writer_input_set_language_code(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ languageCode: UnsafePointer<CChar>?,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = ffiString("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    wrapper.inputs[Int(inputId)].languageCode = languageCode.map(String.init(cString:))
    return AVW_OK
}

@_cdecl("av_writer_input_set_extended_language_tag")
public func av_writer_input_set_extended_language_tag(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ extendedLanguageTag: UnsafePointer<CChar>?,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = ffiString("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    wrapper.inputs[Int(inputId)].extendedLanguageTag = extendedLanguageTag.map(String.init(cString:))
    return AVW_OK
}

@_cdecl("av_writer_input_set_natural_size")
public func av_writer_input_set_natural_size(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ width: Double,
    _ height: Double,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = ffiString("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    wrapper.inputs[Int(inputId)].naturalSize = CGSize(width: width, height: height)
    return AVW_OK
}

@_cdecl("av_writer_input_set_transform")
public func av_writer_input_set_transform(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ a: Double,
    _ b: Double,
    _ c: Double,
    _ d: Double,
    _ tx: Double,
    _ ty: Double,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = ffiString("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    wrapper.inputs[Int(inputId)].transform = CGAffineTransform(a: a, b: b, c: c, d: d, tx: tx, ty: ty)
    return AVW_OK
}

@_cdecl("av_writer_input_set_preferred_volume")
public func av_writer_input_set_preferred_volume(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ preferredVolume: Float,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = ffiString("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    wrapper.inputs[Int(inputId)].preferredVolume = preferredVolume
    return AVW_OK
}

@_cdecl("av_writer_input_set_marks_output_track_as_enabled")
public func av_writer_input_set_marks_output_track_as_enabled(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ enabled: Bool,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = ffiString("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    wrapper.inputs[Int(inputId)].marksOutputTrackAsEnabled = enabled
    return AVW_OK
}

@_cdecl("av_writer_input_set_media_time_scale")
public func av_writer_input_set_media_time_scale(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ mediaTimeScale: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = ffiString("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    wrapper.inputs[Int(inputId)].mediaTimeScale = mediaTimeScale
    return AVW_OK
}

@_cdecl("av_writer_input_set_preferred_media_chunk_duration")
public func av_writer_input_set_preferred_media_chunk_duration(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ durationValue: Int64,
    _ durationScale: Int32,
    _ durationKind: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = ffiString("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    wrapper.inputs[Int(inputId)].preferredMediaChunkDuration = cmTime(value: durationValue, timescale: durationScale, kind: durationKind)
    return AVW_OK
}

@_cdecl("av_writer_input_set_preferred_media_chunk_alignment")
public func av_writer_input_set_preferred_media_chunk_alignment(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ alignment: Int64,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = ffiString("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    wrapper.inputs[Int(inputId)].preferredMediaChunkAlignment = Int(alignment)
    return AVW_OK
}

@_cdecl("av_writer_input_set_sample_reference_base_url")
public func av_writer_input_set_sample_reference_base_url(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ sampleReferenceBaseURL: UnsafePointer<CChar>?,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = ffiString("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    wrapper.inputs[Int(inputId)].sampleReferenceBaseURL = sampleReferenceBaseURL.map {
        URL(string: String(cString: $0))
    } ?? nil
    return AVW_OK
}

@_cdecl("av_writer_input_set_media_data_location")
public func av_writer_input_set_media_data_location(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ location: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = ffiString("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    let raw = String(cString: location)
    guard let decoded = decodeMediaDataLocation(raw) else {
        outErrorMessage?.pointee = ffiString("unknown media data location: \(raw)")
        return AVW_INVALID_ARGUMENT
    }
    wrapper.inputs[Int(inputId)].mediaDataLocation = decoded
    return AVW_OK
}

@_cdecl("av_writer_input_can_add_track_association")
public func av_writer_input_can_add_track_association(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ otherInputId: Int32,
    _ associationType: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count, otherInputId >= 0, Int(otherInputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = ffiString("input id out of range")
        return AVW_INVALID_ARGUMENT
    }
    let type = decodeTrackAssociationType(String(cString: associationType))
    return wrapper.inputs[Int(inputId)].canAddTrackAssociation(withTrackOf: wrapper.inputs[Int(otherInputId)], type: type) ? 1 : 0
}

@_cdecl("av_writer_input_add_track_association")
public func av_writer_input_add_track_association(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ otherInputId: Int32,
    _ associationType: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count, otherInputId >= 0, Int(otherInputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = ffiString("input id out of range")
        return AVW_INVALID_ARGUMENT
    }
    wrapper.inputs[Int(inputId)].addTrackAssociation(withTrackOf: wrapper.inputs[Int(otherInputId)], type: decodeTrackAssociationType(String(cString: associationType)))
    return AVW_OK
}

@_cdecl("av_writer_input_set_performs_multi_pass_encoding_if_supported")
public func av_writer_input_set_performs_multi_pass_encoding_if_supported(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ enabled: Bool,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = ffiString("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    wrapper.inputs[Int(inputId)].performsMultiPassEncodingIfSupported = enabled
    return AVW_OK
}

@_cdecl("av_writer_input_respond_to_each_pass_description")
public func av_writer_input_respond_to_each_pass_description(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ callback: AVWPassDescriptionCallback?,
    _ userdata: UnsafeMutableRawPointer?,
    _ dropUserdata: AVWDropCallback?,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = ffiString("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    guard wrapper.passDescriptionCallbackBoxes[inputId] == nil else {
        outErrorMessage?.pointee = ffiString("respondToEachPassDescription already registered for input \(inputId)")
        return AVW_INVALID_STATE
    }
    let input = wrapper.inputs[Int(inputId)]
    let box = InputCallbackBox(userdata: userdata, dropUserdata: dropUserdata)
    wrapper.passDescriptionCallbackBoxes[inputId] = box
    let queue = DispatchQueue(label: "fish.doom.avassetwriter.input.pass.\(inputId)")
    input.respondToEachPassDescription(on: queue) {
        let payload = input.currentPassDescription.map { description in
            PassDescriptionPayload(sourceTimeRanges: description.sourceTimeRanges.map { encodeTimeRange($0.timeRangeValue) })
        }
        let payloadString = payload.flatMap { try? encodeJson($0) }
        let cString = payloadString.flatMap(ffiString)
        callback?(cString, userdata)
        if let cString {
            free(cString)
        }
    }
    return AVW_OK
}

@_cdecl("av_writer_input_mark_current_pass_as_finished")
public func av_writer_input_mark_current_pass_as_finished(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = ffiString("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    wrapper.inputs[Int(inputId)].markCurrentPassAsFinished()
    return AVW_OK
}

@_cdecl("av_writer_set_movie_fragment_interval")
public func av_writer_set_movie_fragment_interval(
    _ writerPtr: UnsafeMutableRawPointer,
    _ intervalValue: Int64,
    _ intervalScale: Int32,
    _ intervalKind: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    wrapper.writer.movieFragmentInterval = cmTime(value: intervalValue, timescale: intervalScale, kind: intervalKind)
    return AVW_OK
}

@_cdecl("av_writer_set_initial_movie_fragment_interval")
public func av_writer_set_initial_movie_fragment_interval(
    _ writerPtr: UnsafeMutableRawPointer,
    _ intervalValue: Int64,
    _ intervalScale: Int32,
    _ intervalKind: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    guard #available(macOS 14.0, *) else {
        outErrorMessage?.pointee = ffiString("initialMovieFragmentInterval requires macOS 14+")
        return AVW_INVALID_STATE
    }
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    wrapper.writer.initialMovieFragmentInterval = cmTime(value: intervalValue, timescale: intervalScale, kind: intervalKind)
    return AVW_OK
}

@_cdecl("av_writer_set_initial_movie_fragment_sequence_number")
public func av_writer_set_initial_movie_fragment_sequence_number(
    _ writerPtr: UnsafeMutableRawPointer,
    _ sequenceNumber: Int64,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    wrapper.writer.initialMovieFragmentSequenceNumber = Int(sequenceNumber)
    return AVW_OK
}

@_cdecl("av_writer_set_produces_combinable_fragments")
public func av_writer_set_produces_combinable_fragments(
    _ writerPtr: UnsafeMutableRawPointer,
    _ enabled: Bool,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    wrapper.writer.producesCombinableFragments = enabled
    return AVW_OK
}

@_cdecl("av_writer_set_overall_duration_hint")
public func av_writer_set_overall_duration_hint(
    _ writerPtr: UnsafeMutableRawPointer,
    _ hintValue: Int64,
    _ hintScale: Int32,
    _ hintKind: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    wrapper.writer.overallDurationHint = cmTime(value: hintValue, timescale: hintScale, kind: hintKind)
    return AVW_OK
}

@_cdecl("av_writer_set_movie_time_scale")
public func av_writer_set_movie_time_scale(
    _ writerPtr: UnsafeMutableRawPointer,
    _ movieTimeScale: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    wrapper.writer.movieTimeScale = movieTimeScale
    return AVW_OK
}

@_cdecl("av_writer_set_preferred_output_segment_interval")
public func av_writer_set_preferred_output_segment_interval(
    _ writerPtr: UnsafeMutableRawPointer,
    _ intervalValue: Int64,
    _ intervalScale: Int32,
    _ intervalKind: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    wrapper.writer.preferredOutputSegmentInterval = cmTime(value: intervalValue, timescale: intervalScale, kind: intervalKind)
    return AVW_OK
}

@_cdecl("av_writer_set_initial_segment_start_time")
public func av_writer_set_initial_segment_start_time(
    _ writerPtr: UnsafeMutableRawPointer,
    _ startValue: Int64,
    _ startScale: Int32,
    _ startKind: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    wrapper.writer.initialSegmentStartTime = cmTime(value: startValue, timescale: startScale, kind: startKind)
    return AVW_OK
}

@_cdecl("av_writer_set_output_file_type_profile")
public func av_writer_set_output_file_type_profile(
    _ writerPtr: UnsafeMutableRawPointer,
    _ profile: UnsafePointer<CChar>?,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    wrapper.writer.outputFileTypeProfile = decodeFileTypeProfile(profile.map(String.init(cString:)))
    return AVW_OK
}

@_cdecl("av_writer_flush_segment")
public func av_writer_flush_segment(
    _ writerPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    wrapper.writer.flushSegment()
    return AVW_OK
}
