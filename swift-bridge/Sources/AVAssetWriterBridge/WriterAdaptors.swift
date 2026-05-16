import AVFoundation
import CoreMedia
import CoreVideo
import Dispatch
import Foundation
import UniformTypeIdentifiers

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
    guard count > 0 else {
        outErrorMessage?.pointee = ffiString("tagged pixel buffer group must contain at least one pixel buffer")
        return AVW_INVALID_ARGUMENT
    }
    let presentationTime = cmTime(value: ptsValue, timescale: ptsScale, kind: ptsKind)
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
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = ffiString("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    let input = wrapper.inputs[Int(inputId)]
    let adaptor = AVAssetWriterInputMetadataAdaptor(assetWriterInput: input)
    wrapper.metadataAdaptors[inputId] = adaptor
    _ = outErrorMessage
    return AVW_OK
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
    let adaptor = AVAssetWriterInputCaptionAdaptor(assetWriterInput: input)
    wrapper.captionAdaptors[inputId] = adaptor
    _ = outErrorMessage
    return AVW_OK
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

