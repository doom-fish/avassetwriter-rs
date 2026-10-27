import AVAssetWriterObjCBridge
import AVFoundation
import CoreMedia
import CoreVideo
import Dispatch
import Foundation
import UniformTypeIdentifiers

@_cdecl("av_writer_input_info_payload")
public func av_writer_input_info_payload(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32
) -> UnsafeMutablePointer<AVWOwnedPayload>? {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    do {
        return makePayload(try inputInfoPayload(from: wrapper, inputId: inputId))
    } catch {
        return nil
    }
}

@_cdecl("av_writer_input_source_format_hint")
public func av_writer_input_source_format_hint(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32
) -> UnsafeMutableRawPointer? {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else { return nil }
    guard let sourceFormatHint = wrapper.inputs[Int(inputId)].sourceFormatHint else { return nil }
    return Unmanaged.passRetained(sourceFormatHint).toOpaque()
}

@_cdecl("av_writer_set_input_metadata_payload")
public func av_writer_set_input_metadata_payload(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ metadataPayload: UnsafePointer<AVWArray>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.inputSetterStatus(inputId, "metadata", outErrorMessage) {
        return status
    }
    do {
        let payload = try readPayload(metadataPayload, as: [MetadataItemPayload].self)
        wrapper.inputs[Int(inputId)].metadata = try payload.map(avMetadataItem)
        return AVW_OK
    } catch {
        outErrorMessage?.pointee = makePayload(error.localizedDescription)
        return AVW_INVALID_ARGUMENT
    }
}

@_cdecl("av_writer_input_metadata")
public func av_writer_input_metadata(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> UnsafeMutableRawPointer? {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = makePayload("invalid input id: \(inputId)")
        return nil
    }
    return retainedMetadataItemArray(wrapper.inputs[Int(inputId)].metadata)
}

@_cdecl("av_writer_set_input_metadata_items")
public func av_writer_set_input_metadata_items(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ itemPointers: UnsafePointer<UnsafeMutableRawPointer?>?,
    _ itemCount: Int,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.inputSetterStatus(inputId, "metadata", outErrorMessage) {
        return status
    }
    do {
        wrapper.inputs[Int(inputId)].metadata = try metadataItems(
            from: itemPointers,
            count: itemCount
        )
        return AVW_OK
    } catch {
        outErrorMessage?.pointee = makePayload(error.localizedDescription)
        return AVW_INVALID_ARGUMENT
    }
}

@_cdecl("av_writer_input_set_expects_media_data_in_real_time")
public func av_writer_input_set_expects_media_data_in_real_time(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ expectsMediaDataInRealTime: Bool,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.inputSetterStatus(inputId, "expectsMediaDataInRealTime", outErrorMessage) {
        return status
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
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = makePayload("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    guard wrapper.writer.status == .writing else {
        outErrorMessage?.pointee = makePayload(
            "request_media_data_when_ready requires a writing session"
        )
        return AVW_INVALID_STATE
    }
    guard !wrapper.inputIsFinished(inputId) else {
        outErrorMessage?.pointee = makePayload("input \(inputId) is already finished")
        return AVW_INVALID_STATE
    }
    guard wrapper.readyCallbackBoxes[inputId] == nil else {
        outErrorMessage?.pointee = makePayload("requestMediaDataWhenReady already registered for input \(inputId)")
        return AVW_INVALID_STATE
    }
    let input = wrapper.inputs[Int(inputId)]
    let lifetime = CallbackLifetimeBox(userdata: userdata, dropUserdata: dropUserdata)
    let owner = NativeCallbackOwner(lifetime: lifetime)
    wrapper.readyCallbackBoxes[inputId] = lifetime
    let queue = DispatchQueue(label: "fish.doom.avassetwriter.input.ready.\(inputId)")
    input.requestMediaDataWhenReady(on: queue) { [weak wrapper, owner] in
        guard let wrapper else { return }
        wrapper.invokeCallback(owner.lifetime) { userdata in
            callback?(userdata)
        }
    }
    return AVW_OK
}

@_cdecl("av_writer_input_mark_as_finished")
public func av_writer_input_mark_as_finished(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = makePayload("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    guard wrapper.writer.status == .writing else {
        outErrorMessage?.pointee = makePayload("mark_input_as_finished requires a writing session")
        return AVW_INVALID_STATE
    }
    wrapper.markInputAsFinished(inputId)
    return AVW_OK
}

@_cdecl("av_writer_input_remove_callbacks")
public func av_writer_input_remove_callbacks(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = makePayload("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    guard wrapper.removeInputCallbacks(inputId) else {
        outErrorMessage?.pointee = makePayload(
            "input callbacks can only be removed while writing or after terminal state"
        )
        return AVW_INVALID_STATE
    }
    return AVW_OK
}

@_cdecl("av_writer_input_set_language_code")
public func av_writer_input_set_language_code(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ languageCode: UnsafePointer<AVWString>?,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.inputSetterStatus(inputId, "languageCode", outErrorMessage) {
        return status
    }
    var message: NSString?
    guard AVWXSetLanguageCode(
        wrapper.inputs[Int(inputId)],
        languageCode.map({ String(abi: $0) }),
        &message
    ) else {
        outErrorMessage?.pointee = makePayload((message as String?) ?? "invalid ISO 639-2/T language code")
        return AVW_INVALID_ARGUMENT
    }
    return AVW_OK
}

@_cdecl("av_writer_input_set_extended_language_tag")
public func av_writer_input_set_extended_language_tag(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ extendedLanguageTag: UnsafePointer<AVWString>?,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.inputSetterStatus(inputId, "extendedLanguageTag", outErrorMessage) {
        return status
    }
    var message: NSString?
    guard AVWXSetExtendedLanguageTag(
        wrapper.inputs[Int(inputId)],
        extendedLanguageTag.map({ String(abi: $0) }),
        &message
    ) else {
        outErrorMessage?.pointee = makePayload((message as String?) ?? "invalid BCP 47 language tag")
        return AVW_INVALID_ARGUMENT
    }
    return AVW_OK
}

@_cdecl("av_writer_input_set_natural_size")
public func av_writer_input_set_natural_size(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ width: Double,
    _ height: Double,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.inputSetterStatus(inputId, "naturalSize", outErrorMessage) {
        return status
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
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.inputSetterStatus(inputId, "transform", outErrorMessage) {
        return status
    }
    wrapper.inputs[Int(inputId)].transform = CGAffineTransform(a: a, b: b, c: c, d: d, tx: tx, ty: ty)
    return AVW_OK
}

@_cdecl("av_writer_input_set_preferred_volume")
public func av_writer_input_set_preferred_volume(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ preferredVolume: Float,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.inputSetterStatus(inputId, "preferredVolume", outErrorMessage) {
        return status
    }
    wrapper.inputs[Int(inputId)].preferredVolume = preferredVolume
    return AVW_OK
}

@_cdecl("av_writer_input_set_marks_output_track_as_enabled")
public func av_writer_input_set_marks_output_track_as_enabled(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ enabled: Bool,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.inputSetterStatus(inputId, "marksOutputTrackAsEnabled", outErrorMessage) {
        return status
    }
    let input = wrapper.inputs[Int(inputId)]
    guard !wrapper.inputGroups.contains(where: { group in group.inputs.contains { $0 === input } }) else {
        outErrorMessage?.pointee = makePayload(
            "marksOutputTrackAsEnabled is managed by the input group containing input \(inputId)"
        )
        return AVW_INVALID_STATE
    }
    input.marksOutputTrackAsEnabled = enabled
    return AVW_OK
}

@_cdecl("av_writer_input_set_media_time_scale")
public func av_writer_input_set_media_time_scale(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ mediaTimeScale: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.inputSetterStatus(inputId, "mediaTimeScale", outErrorMessage) {
        return status
    }
    let input = wrapper.inputs[Int(inputId)]
    guard input.mediaType != .audio else {
        outErrorMessage?.pointee = makePayload("mediaTimeScale cannot be set on an audio input")
        return AVW_INVALID_ARGUMENT
    }
    guard mediaTimeScale >= 0 else {
        outErrorMessage?.pointee = makePayload("mediaTimeScale must not be negative")
        return AVW_INVALID_ARGUMENT
    }
    input.mediaTimeScale = mediaTimeScale
    return AVW_OK
}

@_cdecl("av_writer_input_set_preferred_media_chunk_duration")
public func av_writer_input_set_preferred_media_chunk_duration(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ durationValue: Int64,
    _ durationScale: Int32,
    _ durationKind: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.inputSetterStatus(inputId, "preferredMediaChunkDuration", outErrorMessage) {
        return status
    }
    let duration = cmTime(value: durationValue, timescale: durationScale, kind: durationKind)
    guard duration.isNumeric, duration > .zero else {
        outErrorMessage?.pointee = makePayload("preferredMediaChunkDuration must be a positive numeric time")
        return AVW_INVALID_ARGUMENT
    }
    wrapper.inputs[Int(inputId)].preferredMediaChunkDuration = duration
    return AVW_OK
}

@_cdecl("av_writer_input_set_preferred_media_chunk_alignment")
public func av_writer_input_set_preferred_media_chunk_alignment(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ alignment: Int64,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.inputSetterStatus(inputId, "preferredMediaChunkAlignment", outErrorMessage) {
        return status
    }
    guard alignment >= 0, let value = Int(exactly: alignment) else {
        outErrorMessage?.pointee = makePayload("preferredMediaChunkAlignment must not be negative")
        return AVW_INVALID_ARGUMENT
    }
    wrapper.inputs[Int(inputId)].preferredMediaChunkAlignment = value
    return AVW_OK
}

@_cdecl("av_writer_input_set_sample_reference_base_url")
public func av_writer_input_set_sample_reference_base_url(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ sampleReferenceBaseURL: UnsafePointer<AVWString>?,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.inputSetterStatus(inputId, "sampleReferenceBaseURL", outErrorMessage) {
        return status
    }
    guard let sampleReferenceBaseURL else {
        wrapper.inputs[Int(inputId)].sampleReferenceBaseURL = nil
        return AVW_OK
    }
    let raw = String(abi: sampleReferenceBaseURL)
    guard let url = URL(string: raw) else {
        outErrorMessage?.pointee = makePayload("sample reference base URL is not a valid URL: \(raw)")
        return AVW_INVALID_ARGUMENT
    }
    wrapper.inputs[Int(inputId)].sampleReferenceBaseURL = url
    return AVW_OK
}

@_cdecl("av_writer_input_set_media_data_location")
public func av_writer_input_set_media_data_location(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ location: UnsafePointer<AVWString>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.inputSetterStatus(inputId, "mediaDataLocation", outErrorMessage) {
        return status
    }
    let raw = String(abi: location)
    guard let decoded = decodeMediaDataLocation(raw) else {
        outErrorMessage?.pointee = makePayload("unknown media data location: \(raw)")
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
    _ associationType: UnsafePointer<AVWString>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count, otherInputId >= 0, Int(otherInputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = makePayload("input id out of range")
        return AVW_INVALID_ARGUMENT
    }
    let type = decodeTrackAssociationType(String(abi: associationType))
    var message: NSString?
    let result = AVWXCanAddTrackAssociation(
        wrapper.inputs[Int(inputId)],
        wrapper.inputs[Int(otherInputId)],
        type,
        &message
    )
    guard result >= 0 else {
        outErrorMessage?.pointee = makePayload((message as String?) ?? "track association check raised")
        return AVW_INVALID_ARGUMENT
    }
    return Int32(result)
}

@_cdecl("av_writer_input_add_track_association")
public func av_writer_input_add_track_association(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ otherInputId: Int32,
    _ associationType: UnsafePointer<AVWString>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count, otherInputId >= 0, Int(otherInputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = makePayload("input id out of range")
        return AVW_INVALID_ARGUMENT
    }
    if let status = wrapper.configurationStatus("track associations", outErrorMessage) {
        return status
    }
    var message: NSString?
    guard AVWXAddTrackAssociation(
        wrapper.inputs[Int(inputId)],
        wrapper.inputs[Int(otherInputId)],
        decodeTrackAssociationType(String(abi: associationType)),
        &message
    ) else {
        outErrorMessage?.pointee = makePayload((message as String?) ?? "cannot add track association")
        return AVW_INVALID_ARGUMENT
    }
    return AVW_OK
}

@_cdecl("av_writer_input_set_performs_multi_pass_encoding_if_supported")
public func av_writer_input_set_performs_multi_pass_encoding_if_supported(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ enabled: Bool,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    if let status = wrapper.inputSetterStatus(inputId, "performsMultiPassEncodingIfSupported", outErrorMessage) {
        return status
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
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = makePayload("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    guard wrapper.writer.status == .writing else {
        outErrorMessage?.pointee = makePayload(
            "respond_to_each_pass_description requires a writing session"
        )
        return AVW_INVALID_STATE
    }
    guard !wrapper.inputIsFinished(inputId) else {
        outErrorMessage?.pointee = makePayload("input \(inputId) is already finished")
        return AVW_INVALID_STATE
    }
    guard wrapper.passDescriptionCallbackBoxes[inputId] == nil else {
        outErrorMessage?.pointee = makePayload("respondToEachPassDescription already registered for input \(inputId)")
        return AVW_INVALID_STATE
    }
    let input = wrapper.inputs[Int(inputId)]
    let lifetime = CallbackLifetimeBox(userdata: userdata, dropUserdata: dropUserdata)
    let owner = NativeCallbackOwner(lifetime: lifetime)
    wrapper.passDescriptionCallbackBoxes[inputId] = lifetime
    let queue = DispatchQueue(label: "fish.doom.avassetwriter.input.pass.\(inputId)")
    input.respondToEachPassDescription(on: queue) { [weak wrapper, weak input, owner] in
        guard let wrapper, let input else { return }
        wrapper.invokeCallback(owner.lifetime) { userdata in
            let payload = input.currentPassDescription.map { description in
                PassDescriptionPayload(
                    sourceTimeRanges: description.sourceTimeRanges.map {
                        encodeTimeRange($0.timeRangeValue)
                    }
                )
            }
            let pointer = payload.flatMap { makePayload($0) }
            callback?(pointer.map { UnsafePointer($0) }, userdata)
            avw_payload_free(pointer)
        }
    }
    return AVW_OK
}

@_cdecl("av_writer_input_mark_current_pass_as_finished")
public func av_writer_input_mark_current_pass_as_finished(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }
    guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
        outErrorMessage?.pointee = makePayload("invalid input id: \(inputId)")
        return AVW_INVALID_ARGUMENT
    }
    guard wrapper.writer.status == .writing else {
        outErrorMessage?.pointee = makePayload(
            "mark_current_pass_as_finished requires a writing session"
        )
        return AVW_INVALID_STATE
    }
    wrapper.markCurrentPassAsFinished(inputId)
    return AVW_OK
}
