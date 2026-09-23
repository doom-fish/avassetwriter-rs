import AVAssetWriterObjCBridge
import AVFoundation
import CoreMedia
import CoreVideo
import Dispatch
import Foundation
import UniformTypeIdentifiers

@_cdecl("av_writer_add_caption_input")
public func av_writer_add_caption_input(
    _ writerPtr: UnsafeMutableRawPointer,
    _ mediaTypePtr: UnsafePointer<CChar>,
    _ expectsMediaDataInRealTime: Bool,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    let mediaTypeString = String(cString: mediaTypePtr)
    guard let mediaType = decodeMediaType(mediaTypeString),
          mediaType == .text || mediaType == .closedCaption else {
        outErrorMessage?.pointee = ffiString("caption inputs must use text or closed_caption media types")
        return AVW_INVALID_ARGUMENT
    }
    if let status = wrapper.configurationStatus("inputs", outErrorMessage) {
        return status
    }
    guard let input = wrapper.makeInput(
        mediaType: mediaType,
        outputSettings: nil,
        sourceFormatHint: nil,
        outErrorMessage
    ) else {
        return AVW_INVALID_ARGUMENT
    }
    input.expectsMediaDataInRealTime = expectsMediaDataInRealTime
    var message: NSString?
    guard let adaptor = AVWXMakeCaptionAdaptor(input, &message) else {
        outErrorMessage?.pointee = ffiString((message as String?) ?? "caption adaptor rejected the input")
        return AVW_INVALID_ARGUMENT
    }
    let inputId = wrapper.addInput(input, outErrorMessage)
    guard inputId >= 0 else {
        return inputId
    }
    wrapper.captionAdaptors[inputId] = adaptor
    wrapper.adaptedInputIds.insert(inputId)
    return inputId
}

@_cdecl("av_writer_attach_pixel_buffer_adaptor_json")
public func av_writer_attach_pixel_buffer_adaptor_json(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ attributesJson: UnsafePointer<CChar>?,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.adaptorStatus(inputId, outErrorMessage) {
        return status
    }
    let input = wrapper.inputs[Int(inputId)]
    guard input.mediaType == .video else {
        outErrorMessage?.pointee = ffiString("pixel buffer adaptors require a video input")
        return AVW_INVALID_ARGUMENT
    }
    do {
        let attributes = try jsonObjectFromCString(attributesJson) as? [String: Any]
        var message: NSString?
        guard let adaptor = AVWXMakePixelBufferAdaptor(input, attributes, &message) else {
            outErrorMessage?.pointee = ffiString((message as String?) ?? "pixel buffer adaptor rejected the input")
            return AVW_INVALID_ARGUMENT
        }
        wrapper.pixelBufferAdaptors[inputId] = adaptor
        wrapper.adaptedInputIds.insert(inputId)
        return AVW_OK
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return AVW_INVALID_ARGUMENT
    }
}

@_cdecl("av_writer_pixel_buffer_pool")
public func av_writer_pixel_buffer_pool(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    guard let adaptor = wrapper.pixelBufferAdaptors[inputId] else {
        return nil
    }
    var message: NSString?
    guard let pool = AVWXPixelBufferPool(adaptor, &message) else {
        if let message {
            outErrorMessage?.pointee = ffiString(message as String)
        }
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
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.adaptorStatus(inputId, outErrorMessage) {
        return status
    }
    let input = wrapper.inputs[Int(inputId)]
    guard input.mediaType == .video else {
        outErrorMessage?.pointee = ffiString("tagged pixel buffer group adaptors require a video input")
        return AVW_INVALID_ARGUMENT
    }
    do {
        let attributes = try jsonObjectFromCString(attributesJson) as? [String: Any]
        var message: NSString?
        guard let adaptor = AVWXMakeTaggedPixelBufferGroupAdaptor(input, attributes, &message) else {
            outErrorMessage?.pointee = ffiString(
                (message as String?) ?? "tagged pixel buffer group adaptor rejected the input"
            )
            return AVW_INVALID_ARGUMENT
        }
        wrapper.taggedPixelBufferGroupAdaptors[inputId] = adaptor
        wrapper.adaptedInputIds.insert(inputId)
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
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
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
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    guard let adaptor = wrapper.taggedPixelBufferGroupAdaptors[inputId] as? AVAssetWriterInputTaggedPixelBufferGroupAdaptor else {
        outErrorMessage?.pointee = ffiString("input \(inputId) has no tagged pixel buffer group adaptor")
        return AVW_INVALID_ARGUMENT
    }
    if let status = wrapper.appendStatus(inputId, outErrorMessage) {
        return status
    }
    let presentationTime = cmTime(value: ptsValue, timescale: ptsScale, kind: ptsKind)
    guard presentationTime.isNumeric else {
        outErrorMessage?.pointee = ffiString("presentation time must be numeric with a positive timescale")
        return AVW_INVALID_ARGUMENT
    }
    guard count > 0 else {
        outErrorMessage?.pointee = ffiString("tagged pixel buffer group must contain at least one pixel buffer")
        return AVW_INVALID_ARGUMENT
    }
    if !adaptor.assetWriterInput.isReadyForMoreMediaData {
        return AVW_INPUT_NOT_READY
    }
    let videoLayerTagCategory: UInt32 = 0x766C6179
    var taggedBuffers: [CMTaggedBuffer] = []
    taggedBuffers.reserveCapacity(count)
    for index in 0..<count {
        guard let pixelBufferPtr = pixelBuffers[index] else {
            outErrorMessage?.pointee = ffiString("tagged pixel buffer group contained a null pixel buffer")
            return AVW_INVALID_ARGUMENT
        }
        let pixelBuffer = Unmanaged<CVPixelBuffer>.fromOpaque(pixelBufferPtr).takeUnretainedValue()
        let tag = CMTag(rawCategory: videoLayerTagCategory, rawTagValue: .int64(layerIds[index]))
        taggedBuffers.append(CMTaggedBuffer(tags: [tag], pixelBuffer: pixelBuffer))
    }
    if adaptor.appendTaggedBuffers(taggedBuffers, withPresentationTime: presentationTime) {
        return AVW_OK
    }
    outErrorMessage?.pointee = ffiString("tagged pixel buffer group append failed")
    return AVW_APPEND_FAILED
}

@_cdecl("av_writer_attach_metadata_adaptor")
public func av_writer_attach_metadata_adaptor(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.adaptorStatus(inputId, outErrorMessage) {
        return status
    }
    let input = wrapper.inputs[Int(inputId)]
    guard input.mediaType == .metadata else {
        outErrorMessage?.pointee = ffiString("metadata adaptors require a metadata input")
        return AVW_INVALID_ARGUMENT
    }
    guard let hint = input.sourceFormatHint,
          CMFormatDescriptionGetMediaSubType(hint) == kCMMetadataFormatType_Boxed else {
        outErrorMessage?.pointee = ffiString(
            "metadata adaptors require a boxed metadata source format hint on the input"
        )
        return AVW_INVALID_ARGUMENT
    }
    var message: NSString?
    guard let adaptor = AVWXMakeMetadataAdaptor(input, &message) else {
        outErrorMessage?.pointee = ffiString((message as String?) ?? "metadata adaptor rejected the input")
        return AVW_INVALID_ARGUMENT
    }
    wrapper.metadataAdaptors[inputId] = adaptor
    wrapper.adaptedInputIds.insert(inputId)
    return AVW_OK
}

@_cdecl("av_writer_append_timed_metadata_group_json")
public func av_writer_append_timed_metadata_group_json(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ groupJson: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    guard let adaptor = wrapper.metadataAdaptors[inputId] else {
        outErrorMessage?.pointee = ffiString("input \(inputId) has no metadata adaptor")
        return AVW_INVALID_ARGUMENT
    }
    if let status = wrapper.appendStatus(inputId, outErrorMessage) {
        return status
    }
    do {
        let payload = try decodeJson(groupJson, as: TimedMetadataGroupPayload.self)
        return adaptor.append(try timedMetadataGroup(from: payload)) ? AVW_OK : AVW_APPEND_FAILED
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return AVW_INVALID_ARGUMENT
    }
}

@_cdecl("av_writer_append_timed_metadata_group")
public func av_writer_append_timed_metadata_group(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ groupPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    guard let adaptor = wrapper.metadataAdaptors[inputId] else {
        outErrorMessage?.pointee = ffiString("input \(inputId) has no metadata adaptor")
        return AVW_INVALID_ARGUMENT
    }
    if let status = wrapper.appendStatus(inputId, outErrorMessage) {
        return status
    }
    let group = Unmanaged<AVTimedMetadataGroup>.fromOpaque(groupPtr).takeUnretainedValue()
    return adaptor.append(group) ? AVW_OK : AVW_APPEND_FAILED
}

@_cdecl("av_writer_attach_caption_adaptor")
public func av_writer_attach_caption_adaptor(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.adaptorStatus(inputId, outErrorMessage) {
        return status
    }
    var message: NSString?
    guard let adaptor = AVWXMakeCaptionAdaptor(wrapper.inputs[Int(inputId)], &message) else {
        outErrorMessage?.pointee = ffiString((message as String?) ?? "caption adaptor rejected the input")
        return AVW_INVALID_ARGUMENT
    }
    wrapper.captionAdaptors[inputId] = adaptor
    wrapper.adaptedInputIds.insert(inputId)
    return AVW_OK
}

@_cdecl("av_writer_append_caption_json")
public func av_writer_append_caption_json(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ captionJson: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    guard let adaptor = wrapper.captionAdaptors[inputId] else {
        outErrorMessage?.pointee = ffiString("input \(inputId) has no caption adaptor")
        return AVW_INVALID_ARGUMENT
    }
    if let status = wrapper.appendStatus(inputId, outErrorMessage) {
        return status
    }
    do {
        let payload = try decodeJson(captionJson, as: CaptionPayload.self)
        var message: NSString?
        let result = try AVWXAppendCaption(adaptor, caption(from: payload), &message)
        return wrapper.appendOutcome(result, message, "caption append", outErrorMessage)
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
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    guard let adaptor = wrapper.captionAdaptors[inputId] else {
        outErrorMessage?.pointee = ffiString("input \(inputId) has no caption adaptor")
        return AVW_INVALID_ARGUMENT
    }
    if let status = wrapper.appendStatus(inputId, outErrorMessage) {
        return status
    }
    do {
        let payload = try decodeJson(captionGroupJson, as: CaptionGroupPayload.self)
        var message: NSString?
        let result = try AVWXAppendCaptionGroup(adaptor, captionGroup(from: payload), &message)
        return wrapper.appendOutcome(result, message, "caption group append", outErrorMessage)
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return AVW_INVALID_ARGUMENT
    }
}
