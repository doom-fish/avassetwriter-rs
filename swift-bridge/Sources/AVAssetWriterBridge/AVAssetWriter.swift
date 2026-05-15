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

private struct AudioFormat {
    let sampleRate: Float64
    let channels: Int
    let bitsPerSample: Int
}

// MARK: - Writer object

private final class Writer {
    let writer: AVAssetWriter
    var inputs: [AVAssetWriterInput] = []
    /// Per-input audio format parameters (only populated for inputs added
    /// via `av_writer_add_audio_input_pcm`). Indexed by InputId.
    var audioFormats: [Int32: AudioFormat] = [:]
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

/// Add an audio input that will mux raw little-endian signed-integer linear
/// PCM samples into the output file. The writer will transcode to AAC
/// internally (matches what AVAssetWriter does when the output container is
/// `.mp4` / `.m4v`).
///
/// `bitsPerSample` must be 16 or 32.
/// `channels` is typically 1 (mono) or 2 (stereo).
/// `sampleRate` is the source sample rate in Hz (typically 44100 or 48000).
///
/// Returns a non-negative input id on success.
@_cdecl("av_writer_add_audio_input_pcm")
public func av_writer_add_audio_input_pcm(
    _ writerPtr: UnsafeMutableRawPointer,
    _ sampleRate: Float64,
    _ channels: UInt32,
    _ bitsPerSample: UInt32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()

    guard channels >= 1, channels <= 8 else {
        outErrorMessage?.pointee = ffiString("channels must be in 1...8 (got \(channels))")
        return AVW_INVALID_ARGUMENT
    }
    guard bitsPerSample == 16 || bitsPerSample == 32 else {
        outErrorMessage?.pointee = ffiString("bitsPerSample must be 16 or 32 (got \(bitsPerSample))")
        return AVW_INVALID_ARGUMENT
    }

    // outputSettings tells AVAssetWriter the *destination* encoding. For
    // .mp4 / .m4v containers we ask it to transcode to AAC at 128 kbps,
    // which matches the QuickTime defaults and gives the user a portable
    // result without having to think about codec selection.
    let outputSettings: [String: Any] = [
        AVFormatIDKey: kAudioFormatMPEG4AAC,
        AVSampleRateKey: sampleRate,
        AVNumberOfChannelsKey: Int(channels),
        AVEncoderBitRateKey: 128_000,
    ]

    let input = AVAssetWriterInput(mediaType: .audio, outputSettings: outputSettings)
    input.expectsMediaDataInRealTime = true

    if !wrapper.writer.canAdd(input) {
        outErrorMessage?.pointee = ffiString(
            "writer cannot add audio input (status=\(wrapper.writer.status.rawValue))"
        )
        return AVW_INVALID_STATE
    }
    wrapper.writer.add(input)

    let id = Int32(wrapper.inputs.count)
    wrapper.inputs.append(input)
    // Stash the source format so append_audio_pcm can rebuild a sample
    // buffer with the same format description on every push.
    wrapper.audioFormats[id] = AudioFormat(
        sampleRate: sampleRate,
        channels: Int(channels),
        bitsPerSample: Int(bitsPerSample)
    )
    return id
}

/// Append `frameCount` PCM frames (each frame = `channels` samples) to the
/// audio input identified by `inputId`. `pcmBytes` must point to
/// `frameCount * channels * (bitsPerSample / 8)` bytes of interleaved
/// little-endian signed-integer PCM data.
///
/// `pts` is the presentation time of the first frame (numerator + timescale
/// matching the configured `sampleRate`).
@_cdecl("av_writer_append_audio_pcm")
public func av_writer_append_audio_pcm(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ pcmBytes: UnsafePointer<UInt8>,
    _ pcmByteCount: Int,
    _ frameCount: Int,
    _ ptsValue: Int64,
    _ ptsTimescale: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
    guard inputId >= 0 && Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = ffiString("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    guard let fmt = wrapper.audioFormats[inputId] else {
        outErrorMessage?.pointee = ffiString("input \(inputId) is not an audio input")
        return AVW_INVALID_ARGUMENT
    }
    let input = wrapper.inputs[Int(inputId)]
    if !input.isReadyForMoreMediaData {
        return AVW_INPUT_NOT_READY
    }

    // Build a CMAudioFormatDescription from the cached parameters.
    var asbd = AudioStreamBasicDescription(
        mSampleRate: fmt.sampleRate,
        mFormatID: kAudioFormatLinearPCM,
        mFormatFlags: kLinearPCMFormatFlagIsSignedInteger | kLinearPCMFormatFlagIsPacked,
        mBytesPerPacket: UInt32(fmt.channels * fmt.bitsPerSample / 8),
        mFramesPerPacket: 1,
        mBytesPerFrame: UInt32(fmt.channels * fmt.bitsPerSample / 8),
        mChannelsPerFrame: UInt32(fmt.channels),
        mBitsPerChannel: UInt32(fmt.bitsPerSample),
        mReserved: 0
    )
    var formatDesc: CMAudioFormatDescription?
    var status = CMAudioFormatDescriptionCreate(
        allocator: kCFAllocatorDefault,
        asbd: &asbd,
        layoutSize: 0,
        layout: nil,
        magicCookieSize: 0,
        magicCookie: nil,
        extensions: nil,
        formatDescriptionOut: &formatDesc
    )
    guard status == noErr, let formatDesc = formatDesc else {
        outErrorMessage?.pointee = ffiString("CMAudioFormatDescriptionCreate failed: \(status)")
        return AVW_APPEND_FAILED
    }

    // Wrap the PCM bytes in a CMBlockBuffer that doesn't copy the memory.
    var blockBuffer: CMBlockBuffer?
    status = CMBlockBufferCreateWithMemoryBlock(
        allocator: kCFAllocatorDefault,
        memoryBlock: nil,
        blockLength: pcmByteCount,
        blockAllocator: kCFAllocatorDefault,
        customBlockSource: nil,
        offsetToData: 0,
        dataLength: pcmByteCount,
        flags: kCMBlockBufferAssureMemoryNowFlag,
        blockBufferOut: &blockBuffer
    )
    guard status == noErr, let blockBuffer = blockBuffer else {
        outErrorMessage?.pointee = ffiString("CMBlockBufferCreate failed: \(status)")
        return AVW_APPEND_FAILED
    }
    status = CMBlockBufferReplaceDataBytes(
        with: pcmBytes,
        blockBuffer: blockBuffer,
        offsetIntoDestination: 0,
        dataLength: pcmByteCount
    )
    guard status == noErr else {
        outErrorMessage?.pointee = ffiString("CMBlockBufferReplaceDataBytes failed: \(status)")
        return AVW_APPEND_FAILED
    }

    // Create the CMSampleBuffer with timing info.
    let pts = CMTime(value: ptsValue, timescale: ptsTimescale)
    var sampleBuffer: CMSampleBuffer?
    status = CMAudioSampleBufferCreateReadyWithPacketDescriptions(
        allocator: kCFAllocatorDefault,
        dataBuffer: blockBuffer,
        formatDescription: formatDesc,
        sampleCount: CMItemCount(frameCount),
        presentationTimeStamp: pts,
        packetDescriptions: nil,
        sampleBufferOut: &sampleBuffer
    )
    guard status == noErr, let sampleBuffer = sampleBuffer else {
        outErrorMessage?.pointee = ffiString(
            "CMAudioSampleBufferCreateReadyWithPacketDescriptions failed: \(status)"
        )
        return AVW_APPEND_FAILED
    }

    if !input.append(sampleBuffer) {
        let msg = wrapper.writer.error?.localizedDescription
            ?? "audio append() returned false (status=\(wrapper.writer.status.rawValue))"
        outErrorMessage?.pointee = ffiString(msg)
        return AVW_APPEND_FAILED
    }
    return AVW_OK
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
