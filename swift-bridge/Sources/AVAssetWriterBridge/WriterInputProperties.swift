import AVFoundation
import CoreMedia
import CoreVideo
import Dispatch
import Foundation
import UniformTypeIdentifiers

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

