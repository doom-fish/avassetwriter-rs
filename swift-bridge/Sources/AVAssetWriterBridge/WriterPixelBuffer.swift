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
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()

    let outputSettings: [String: Any] = [
        AVVideoCodecKey: AVVideoCodecType.h264,
        AVVideoWidthKey: width,
        AVVideoHeightKey: height,
    ]
    let input = AVAssetWriterInput(mediaType: .video, outputSettings: outputSettings)
    input.expectsMediaDataInRealTime = true

    let pba = AVAssetWriterInputPixelBufferAdaptor(
        assetWriterInput: input,
        sourcePixelBufferAttributes: [
            kCVPixelBufferPixelFormatTypeKey as String: pixelFormatType,
            kCVPixelBufferWidthKey as String: width,
            kCVPixelBufferHeightKey as String: height,
        ]
    )

    if !wrapper.writer.canAdd(input) {
        outErrorMessage?.pointee = ffiString("writer cannot add pixel-buffer video input")
        return AVW_INVALID_STATE
    }
    wrapper.writer.add(input)

    let id = Int32(wrapper.inputs.count)
    wrapper.inputs.append(input)
    wrapper.pixelBufferAdaptors[id] = pba
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
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard let pba = wrapper.pixelBufferAdaptors[inputId] else {
        outErrorMessage?.pointee = ffiString("input id \(inputId) has no pixel buffer adaptor")
        return AVW_INVALID_ARGUMENT
    }
    if !pba.assetWriterInput.isReadyForMoreMediaData {
        return AVW_INPUT_NOT_READY
    }
    let pixelBuffer = Unmanaged<CVPixelBuffer>.fromOpaque(pixelBufferPtr).takeUnretainedValue()
    let pts = CMTime(value: ptsValue, timescale: ptsTimescale)
    if pba.append(pixelBuffer, withPresentationTime: pts) {
        return AVW_OK
    }
    outErrorMessage?.pointee = ffiString("pixel buffer adaptor append failed")
    return AVW_APPEND_FAILED
}
