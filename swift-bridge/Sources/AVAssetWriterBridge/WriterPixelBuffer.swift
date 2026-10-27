import AVAssetWriterObjCBridge
import AVFoundation
import CoreMedia
import CoreVideo
import Dispatch
import Foundation
import UniformTypeIdentifiers

// MARK: - Pixel buffer adaptor (zero-copy CVPixelBuffer ingest)

/// Add a video input + AVAssetWriterInputPixelBufferAdaptor pair for
/// zero-copy CVPixelBuffer ingest. Useful when you have raw frames
/// (e.g. from a render pipeline) instead of pre-encoded sample buffers.
///
/// `width` and `height` are the source pixel dimensions.
/// `pixelFormatType` is a kCVPixelFormatType_* FourCC (typically 'BGRA').
@_cdecl("av_writer_add_video_input_pixel_buffer")
public func av_writer_add_video_input_pixel_buffer(
    _ writerPtr: UnsafeMutableRawPointer,
    _ width: Int32,
    _ height: Int32,
    _ pixelFormatType: UInt32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }

    if let status = wrapper.configurationStatus("inputs", outErrorMessage) {
        return status
    }
    guard width > 0, height > 0 else {
        outErrorMessage?.pointee = makePayload("width and height must be positive (got \(width)x\(height))")
        return AVW_INVALID_ARGUMENT
    }
    let outputSettings: [String: Any] = [
        AVVideoCodecKey: AVVideoCodecType.h264,
        AVVideoWidthKey: width,
        AVVideoHeightKey: height,
    ]
    guard let input = wrapper.makeInput(
        mediaType: .video,
        outputSettings: outputSettings,
        sourceFormatHint: nil,
        outErrorMessage
    ) else {
        return AVW_INVALID_ARGUMENT
    }
    input.expectsMediaDataInRealTime = true

    var message: NSString?
    guard let pba = AVWXMakePixelBufferAdaptor(
        input,
        [
            kCVPixelBufferPixelFormatTypeKey as String: pixelFormatType,
            kCVPixelBufferWidthKey as String: width,
            kCVPixelBufferHeightKey as String: height,
        ],
        &message
    ) else {
        outErrorMessage?.pointee = makePayload((message as String?) ?? "pixel buffer adaptor rejected the input")
        return AVW_INVALID_ARGUMENT
    }

    let id = wrapper.addInput(input, outErrorMessage)
    guard id >= 0 else {
        return id
    }
    wrapper.pixelBufferAdaptors[id] = pba
    wrapper.adaptedInputIds.insert(id)
    return id
}

/// Append a CVPixelBuffer through the previously-added pixel-buffer
/// adaptor. `pts` is presentation time in (value, timescale) units.
@_cdecl("av_writer_append_pixel_buffer")
public func av_writer_append_pixel_buffer(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ pixelBufferPtr: UnsafeMutableRawPointer,
    _ ptsValue: Int64,
    _ ptsTimescale: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    guard let pba = wrapper.pixelBufferAdaptors[inputId] else {
        outErrorMessage?.pointee = makePayload("input id \(inputId) has no pixel buffer adaptor")
        return AVW_INVALID_ARGUMENT
    }
    if let status = wrapper.appendStatus(inputId, outErrorMessage) {
        return status
    }
    guard ptsTimescale > 0 else {
        outErrorMessage?.pointee = makePayload("presentation time must have a positive timescale")
        return AVW_INVALID_ARGUMENT
    }
    if !pba.assetWriterInput.isReadyForMoreMediaData {
        return AVW_INPUT_NOT_READY
    }
    let pixelBuffer = Unmanaged<CVPixelBuffer>.fromOpaque(pixelBufferPtr).takeUnretainedValue()
    let pts = CMTime(value: ptsValue, timescale: ptsTimescale)
    var message: NSString?
    let result = AVWXAppendPixelBuffer(pba, pixelBuffer, pts, &message)
    return wrapper.appendOutcome(result, message, "pixel buffer adaptor append", outErrorMessage)
}
