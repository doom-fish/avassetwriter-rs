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
