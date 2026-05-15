// AVAssetWriter Swift bridge.
//
// Hides AVFoundation's Obj-C / KVO / async-completion-handler surface behind
// a minimal C-callable API:
//
//   av_writer_create(path, file_type) -> Writer*
//   av_writer_add_video_input(Writer*, sample_buffer) -> InputId
//   av_writer_start_session(Writer*, source_time)
//   av_writer_append_sample(Writer*, InputId, sample_buffer) -> ok
//   av_writer_finish(Writer*) -> ok                       (blocks)
//   av_writer_release(Writer*)
//
// The CMSampleBufferRef we accept comes verbatim from videotoolbox-rs's
// EncodedFrame::cm_sample_buffer_ptr — no reconstruction needed.

import AVFoundation
import CoreMedia
import Foundation

// MARK: - Status Codes (mirrored in src/error.rs)

private let AVW_OK: Int32 = 0
private let AVW_INVALID_ARGUMENT: Int32 = -1
private let AVW_WRITER_CREATE_FAILED: Int32 = -2
private let AVW_INPUT_NOT_READY: Int32 = -3
private let AVW_APPEND_FAILED: Int32 = -4
private let AVW_FINISH_FAILED: Int32 = -5
private let AVW_INVALID_STATE: Int32 = -6

// MARK: - String helpers

@_cdecl("avw_string_free")
public func avw_string_free(_ str: UnsafeMutablePointer<CChar>?) {
    guard let str = str else { return }
    free(str)
}

private func ffiString(_ s: String) -> UnsafeMutablePointer<CChar>? {
    return s.withCString { strdup($0) }
}

// MARK: - Writer object

private final class Writer {
    let writer: AVAssetWriter
    var inputs: [AVAssetWriterInput] = []
    var lastError: String? = nil

    init(writer: AVAssetWriter) {
        self.writer = writer
    }
}

// MARK: - Lifetime

/// `fileType` is one of:
///   "mp4"  -> AVFileType.mp4
///   "mov"  -> AVFileType.mov
///   "m4v"  -> AVFileType.m4v
@_cdecl("av_writer_create")
public func av_writer_create(
    _ path: UnsafePointer<CChar>,
    _ fileType: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    let pathStr = String(cString: path)
    let typeStr = String(cString: fileType)

    let avFileType: AVFileType
    switch typeStr {
    case "mp4": avFileType = .mp4
    case "mov": avFileType = .mov
    case "m4v": avFileType = .m4v
    default:
        outErrorMessage?.pointee = ffiString("unknown file type: \(typeStr)")
        return nil
    }

    // Remove any pre-existing file at the destination — AVAssetWriter refuses
    // to overwrite. This matches the convention of every other macOS muxer.
    try? FileManager.default.removeItem(atPath: pathStr)

    let url = URL(fileURLWithPath: pathStr)
    do {
        let writer = try AVAssetWriter(outputURL: url, fileType: avFileType)
        let wrapper = Writer(writer: writer)
        return Unmanaged.passRetained(wrapper).toOpaque()
    } catch {
        outErrorMessage?.pointee = ffiString("AVAssetWriter init failed: \(error.localizedDescription)")
        return nil
    }
}

@_cdecl("av_writer_release")
public func av_writer_release(_ ptr: UnsafeMutableRawPointer?) {
    guard let ptr = ptr else { return }
    Unmanaged<Writer>.fromOpaque(ptr).release()
}

// MARK: - Configure inputs

/// Add a video input whose format is inferred from the first CMSampleBuffer
/// we'll be appending. This is the simplest path — no need for the caller
/// to specify codec/dimensions separately.
///
/// Returns a non-negative input id on success, or a negative status code on
/// failure. `outErrorMessage` is populated when the return is negative.
@_cdecl("av_writer_add_video_input_from_sample")
public func av_writer_add_video_input_from_sample(
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

    let input = AVAssetWriterInput(mediaType: .video, outputSettings: nil, sourceFormatHint: format)
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

// MARK: - Start / append / finish

/// Begin writing. Must be called before any `append`. `sourceTimeValue` and
/// `sourceTimeScale` form the CMTime of the first sample in the file.
@_cdecl("av_writer_start_session")
public func av_writer_start_session(
    _ writerPtr: UnsafeMutableRawPointer,
    _ sourceTimeValue: Int64,
    _ sourceTimeScale: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    if !wrapper.writer.startWriting() {
        let msg = wrapper.writer.error?.localizedDescription ?? "startWriting() returned false"
        outErrorMessage?.pointee = ffiString(msg)
        return AVW_WRITER_CREATE_FAILED
    }
    let t = CMTime(value: sourceTimeValue, timescale: sourceTimeScale)
    wrapper.writer.startSession(atSourceTime: t)
    return AVW_OK
}

/// Append a sample buffer to the input identified by `inputId`. Returns
/// AVW_OK on success, AVW_INPUT_NOT_READY if the input is back-pressuring
/// (caller should retry shortly), or AVW_APPEND_FAILED with an error message.
@_cdecl("av_writer_append_sample")
public func av_writer_append_sample(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ sampleBufferPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard inputId >= 0 && Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = ffiString("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    let input = wrapper.inputs[Int(inputId)]
    let sampleBuffer = Unmanaged<CMSampleBuffer>.fromOpaque(sampleBufferPtr).takeUnretainedValue()

    if !input.isReadyForMoreMediaData {
        return AVW_INPUT_NOT_READY
    }
    if !input.append(sampleBuffer) {
        let msg = wrapper.writer.error?.localizedDescription
            ?? "append() returned false (status=\(wrapper.writer.status.rawValue))"
        outErrorMessage?.pointee = ffiString(msg)
        return AVW_APPEND_FAILED
    }
    return AVW_OK
}

/// Finalise the file. Marks all inputs as finished, blocks until the
/// asynchronous AVAssetWriter completion handler fires, then returns the
/// terminal status. AVW_OK indicates the file is fully written.
@_cdecl("av_writer_finish")
public func av_writer_finish(
    _ writerPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    for input in wrapper.inputs {
        input.markAsFinished()
    }

    let semaphore = DispatchSemaphore(value: 0)
    wrapper.writer.finishWriting {
        semaphore.signal()
    }
    semaphore.wait()

    switch wrapper.writer.status {
    case .completed:
        return AVW_OK
    case .failed:
        let msg = wrapper.writer.error?.localizedDescription ?? "writer status = failed"
        outErrorMessage?.pointee = ffiString(msg)
        return AVW_FINISH_FAILED
    case .cancelled:
        outErrorMessage?.pointee = ffiString("writer cancelled")
        return AVW_FINISH_FAILED
    default:
        outErrorMessage?.pointee = ffiString("writer status = \(wrapper.writer.status.rawValue)")
        return AVW_INVALID_STATE
    }
}
