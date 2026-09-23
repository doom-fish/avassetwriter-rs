import AVAssetWriterObjCBridge
import AVFoundation
import CoreMedia
import Foundation

typealias AVWErrorOut = UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?

extension Writer {
    func configurationStatus(_ property: String, _ outErrorMessage: AVWErrorOut) -> Int32? {
        guard writer.status != .unknown else { return nil }
        outErrorMessage?.pointee = ffiString(
            "\(property) cannot be changed after writing has started (status=\(writer.status.rawValue))"
        )
        return AVW_INVALID_STATE
    }

    func inputSetterStatus(_ inputId: Int32, _ property: String, _ outErrorMessage: AVWErrorOut) -> Int32? {
        guard inputId >= 0, Int(inputId) < inputs.count else {
            outErrorMessage?.pointee = ffiString("invalid input id: \(inputId)")
            return AVW_INVALID_ARGUMENT
        }
        return configurationStatus(property, outErrorMessage)
    }

    func appendStatus(_ inputId: Int32, _ outErrorMessage: AVWErrorOut) -> Int32? {
        guard inputId >= 0, Int(inputId) < inputs.count else {
            outErrorMessage?.pointee = ffiString("invalid input id: \(inputId)")
            return AVW_INVALID_ARGUMENT
        }
        let reason: String?
        if writer.status != .writing {
            reason = "writer is not writing (status=\(writer.status.rawValue)); call start_session first"
        } else if !sessionStarted {
            reason = "no sample-writing session has been started"
        } else if sessionEnded {
            reason = "the sample-writing session has already ended"
        } else if inputIsFinished(inputId) {
            reason = "input \(inputId) is already marked as finished"
        } else {
            reason = nil
        }
        guard let reason else { return nil }
        outErrorMessage?.pointee = ffiString(reason)
        return AVW_INVALID_STATE
    }

    func appendOutcome(
        _ result: Int,
        _ message: NSString?,
        _ operation: String,
        _ outErrorMessage: AVWErrorOut
    ) -> Int32 {
        switch result {
        case 1:
            return AVW_OK
        case 0:
            outErrorMessage?.pointee = ffiString(
                writer.error?.localizedDescription
                    ?? "\(operation) returned false (status=\(writer.status.rawValue))"
            )
            return AVW_APPEND_FAILED
        default:
            outErrorMessage?.pointee = ffiString((message as String?) ?? "\(operation) raised an exception")
            return AVW_INVALID_STATE
        }
    }

    func adaptorStatus(_ inputId: Int32, _ outErrorMessage: AVWErrorOut) -> Int32? {
        if let status = inputSetterStatus(inputId, "adaptors", outErrorMessage) {
            return status
        }
        guard !adaptedInputIds.contains(inputId) else {
            outErrorMessage?.pointee = ffiString("input \(inputId) already has an adaptor")
            return AVW_INVALID_STATE
        }
        return nil
    }

    func makeInput(
        mediaType: AVMediaType,
        outputSettings: [String: Any]?,
        sourceFormatHint: CMFormatDescription?,
        _ outErrorMessage: AVWErrorOut
    ) -> AVAssetWriterInput? {
        var message: NSString?
        guard let input = AVWXMakeInput(mediaType, outputSettings, sourceFormatHint, &message) else {
            outErrorMessage?.pointee = ffiString(
                (message as String?) ?? "AVAssetWriterInput rejected its settings"
            )
            return nil
        }
        return input
    }

    func addInput(_ input: AVAssetWriterInput, _ outErrorMessage: AVWErrorOut) -> Int32 {
        if let status = configurationStatus("inputs", outErrorMessage) {
            return status
        }
        var message: NSString?
        guard AVWXAddInput(writer, input, &message) else {
            outErrorMessage?.pointee = ffiString((message as String?) ?? "writer cannot add input")
            return AVW_INVALID_STATE
        }
        let id = Int32(inputs.count)
        inputs.append(input)
        return id
    }

    func addInput(
        mediaType: AVMediaType,
        outputSettings: [String: Any]?,
        sourceFormatHint: CMFormatDescription?,
        expectsMediaDataInRealTime: Bool,
        _ outErrorMessage: AVWErrorOut
    ) -> Int32 {
        if let status = configurationStatus("inputs", outErrorMessage) {
            return status
        }
        guard let input = makeInput(
            mediaType: mediaType,
            outputSettings: outputSettings,
            sourceFormatHint: sourceFormatHint,
            outErrorMessage
        ) else {
            return AVW_INVALID_ARGUMENT
        }
        input.expectsMediaDataInRealTime = expectsMediaDataInRealTime
        return addInput(input, outErrorMessage)
    }
}

func mediaTypeString(of format: CMFormatDescription) -> String {
    let code = CMFormatDescriptionGetMediaType(format)
    let bytes = [24, 16, 8, 0].map { UInt8(truncatingIfNeeded: code >> UInt32($0)) }
    return String(decoding: bytes, as: UTF8.self)
}

func formatHintStatus(
    _ format: CMFormatDescription,
    expected mediaType: AVMediaType,
    _ outErrorMessage: AVWErrorOut
) -> Int32? {
    let formatType = mediaTypeString(of: format)
    guard formatType != mediaType.rawValue else { return nil }
    outErrorMessage?.pointee = ffiString(
        "format description media type '\(formatType)' is not '\(mediaType.rawValue)'"
    )
    return AVW_INVALID_ARGUMENT
}

func sampleMediaTypeStatus(
    _ sampleBuffer: CMSampleBuffer,
    _ input: AVAssetWriterInput,
    _ outErrorMessage: AVWErrorOut
) -> Int32? {
    guard let format = CMSampleBufferGetFormatDescription(sampleBuffer) else { return nil }
    let sampleType = mediaTypeString(of: format)
    guard sampleType != input.mediaType.rawValue else { return nil }
    outErrorMessage?.pointee = ffiString(
        "sample buffer media type '\(sampleType)' does not match the input's '\(input.mediaType.rawValue)'"
    )
    return AVW_INVALID_ARGUMENT
}
