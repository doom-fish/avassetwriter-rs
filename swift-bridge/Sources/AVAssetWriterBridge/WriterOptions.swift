import AVFoundation
import CoreMedia
import CoreVideo
import Dispatch
import Foundation
import UniformTypeIdentifiers

@_cdecl("av_writer_set_movie_fragment_interval")
public func av_writer_set_movie_fragment_interval(
    _ writerPtr: UnsafeMutableRawPointer,
    _ intervalValue: Int64,
    _ intervalScale: Int32,
    _ intervalKind: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.configurationStatus("movieFragmentInterval", outErrorMessage) {
        return status
    }
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
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.configurationStatus("initialMovieFragmentInterval", outErrorMessage) {
        return status
    }
    wrapper.writer.initialMovieFragmentInterval = cmTime(value: intervalValue, timescale: intervalScale, kind: intervalKind)
    return AVW_OK
}

@_cdecl("av_writer_set_initial_movie_fragment_sequence_number")
public func av_writer_set_initial_movie_fragment_sequence_number(
    _ writerPtr: UnsafeMutableRawPointer,
    _ sequenceNumber: Int64,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.configurationStatus("initialMovieFragmentSequenceNumber", outErrorMessage) {
        return status
    }
    guard sequenceNumber >= 1 else {
        outErrorMessage?.pointee = ffiString("initialMovieFragmentSequenceNumber must be at least 1")
        return AVW_INVALID_ARGUMENT
    }
    wrapper.writer.initialMovieFragmentSequenceNumber = Int(sequenceNumber)
    return AVW_OK
}

@_cdecl("av_writer_set_produces_combinable_fragments")
public func av_writer_set_produces_combinable_fragments(
    _ writerPtr: UnsafeMutableRawPointer,
    _ enabled: Bool,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.configurationStatus("producesCombinableFragments", outErrorMessage) {
        return status
    }
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
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.configurationStatus("overallDurationHint", outErrorMessage) {
        return status
    }
    let hint = cmTime(value: hintValue, timescale: hintScale, kind: hintKind)
    guard !hint.isValid || (hint.isNumeric && hint >= .zero) else {
        outErrorMessage?.pointee = ffiString("overallDurationHint must be invalid or a non-negative numeric time")
        return AVW_INVALID_ARGUMENT
    }
    wrapper.writer.overallDurationHint = hint
    return AVW_OK
}

@_cdecl("av_writer_set_movie_time_scale")
public func av_writer_set_movie_time_scale(
    _ writerPtr: UnsafeMutableRawPointer,
    _ movieTimeScale: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.configurationStatus("movieTimeScale", outErrorMessage) {
        return status
    }
    guard movieTimeScale >= 0 else {
        outErrorMessage?.pointee = ffiString("movieTimeScale must not be negative")
        return AVW_INVALID_ARGUMENT
    }
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
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.configurationStatus("preferredOutputSegmentInterval", outErrorMessage) {
        return status
    }
    let interval = cmTime(value: intervalValue, timescale: intervalScale, kind: intervalKind)
    guard !interval.isValid || interval.isIndefinite || (interval.isNumeric && interval > .zero) else {
        outErrorMessage?.pointee = ffiString(
            "preferredOutputSegmentInterval must be invalid, indefinite or a positive numeric time"
        )
        return AVW_INVALID_ARGUMENT
    }
    wrapper.writer.preferredOutputSegmentInterval = interval
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
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.configurationStatus("initialSegmentStartTime", outErrorMessage) {
        return status
    }
    wrapper.writer.initialSegmentStartTime = cmTime(value: startValue, timescale: startScale, kind: startKind)
    return AVW_OK
}

@_cdecl("av_writer_set_output_file_type_profile")
public func av_writer_set_output_file_type_profile(
    _ writerPtr: UnsafeMutableRawPointer,
    _ profile: UnsafePointer<CChar>?,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.configurationStatus("outputFileTypeProfile", outErrorMessage) {
        return status
    }
    wrapper.writer.outputFileTypeProfile = decodeFileTypeProfile(profile.map(String.init(cString:)))
    return AVW_OK
}

@_cdecl("av_writer_flush_segment")
public func av_writer_flush_segment(
    _ writerPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    guard wrapper.writer.status == .writing else {
        outErrorMessage?.pointee = ffiString("flush_segment requires a writing session")
        return AVW_INVALID_STATE
    }
    guard wrapper.writer.preferredOutputSegmentInterval == .indefinite else {
        outErrorMessage?.pointee = ffiString(
            "flush_segment requires preferred_output_segment_interval to be indefinite"
        )
        return AVW_INVALID_STATE
    }
    guard wrapper.segmentDelegate != nil else {
        outErrorMessage?.pointee = ffiString("flush_segment requires a writer created for segmented output")
        return AVW_INVALID_STATE
    }
    guard wrapper.runBlockingNativeOperation({
        wrapper.writer.flushSegment()
    }) else {
        outErrorMessage?.pointee = ffiString(
            "flush_segment cannot run reentrantly or while another blocking writer operation is active"
        )
        return AVW_INVALID_STATE
    }
    return AVW_OK
}
