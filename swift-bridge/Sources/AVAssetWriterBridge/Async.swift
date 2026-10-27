import AVAssetWriterObjCBridge
// Async.swift — callback-based FFI thunks wrapping AVAssetWriter and
// AVAssetExportSession completion-handler APIs.
//
// Each thunk takes a C callback `cb` and opaque `ctx` pointer, fires the
// underlying Apple completion handler, then calls `cb(result, error, ctx)`
// exactly once when the operation finishes.  The Rust side converts the
// callback into a `std::future::Future` via `doom_fish_utils::completion`.
//
// AVAssetWriterInput.requestMediaDataWhenReady(on:using:) is a *multi-fire*
// handler, so Rust exposes it as a bounded async stream via the ready-stream
// bridge below rather than as a one-shot Future.
//
// AVAssetExportSession.export(to:as:isolation:) is the Swift concurrency
// projection of exportAsynchronouslyWithCompletionHandler: added in macOS
// 26.0.  It is covered by av_export_session_export_async below (same
// underlying completion handler, same result semantics).
//
// AVOutputSettingsAssistant.compatibilityTest(forSourceFormat:completionHandler:)
// does not exist in the AVFoundation SDK; AVOutputSettingsAssistant is a
// synchronous settings-recommendation class with no completion-handler
// surface.  No thunk is generated.

import AVFoundation
import Dispatch
import Foundation

/// Sentinel non-null pointer used to signal "success, no meaningful value".
/// The actual address is irrelevant; Rust only checks non-null vs null.
private let kSuccessSentinel = UnsafeRawPointer(bitPattern: 1)!

// MARK: - AVAssetWriter.finishWritingWithCompletionHandler:

/// Async version of `av_writer_finish`.
///
/// Marks all inputs as finished, then calls
/// `AVAssetWriter.finishWritingWithCompletionHandler:` and fires `cb` once
/// the operation completes.
///
/// - Parameters:
///   - writerPtr: opaque Rust `Writer` pointer
///   - cb: C callback `(result, error_cstr, ctx)`:
///     success → `(sentinel, nil, ctx)`, failure → `(nil, c-string, ctx)`
///   - ctx: opaque context pointer forwarded verbatim to `cb`
@_cdecl("av_writer_finish_async")
public func av_writer_finish_async(
    _ writerPtr: UnsafeMutableRawPointer,
    _ cb: @convention(c) (UnsafeRawPointer?, UnsafePointer<AVWString>?, UnsafeMutableRawPointer) -> Void,
    _ ctx: UnsafeMutableRawPointer
) {
    let wrapper = Writer.locked(writerPtr)
    defer { wrapper.unlock() }

    guard wrapper.writer.status == .writing else {
        switch wrapper.writer.status {
        case .completed:
            wrapper.detachCallbacksAfterTerminalState()
            cb(kSuccessSentinel, nil, ctx)
        case .failed:
            wrapper.detachCallbacksAfterTerminalState()
            let msg = wrapper.writer.error?.localizedDescription ?? "writer status = failed"
            msg.withABIString { cb(nil, $0, ctx) }
        case .cancelled:
            wrapper.detachCallbacksAfterTerminalState()
            "writer cancelled".withABIString { cb(nil, $0, ctx) }
        default:
            let raw = wrapper.writer.status.rawValue
            "writer not in writing state (status=\(raw)); call start_session first".withABIString {
                cb(nil, $0, ctx)
            }
        }
        return
    }

    guard wrapper.beginFinishing() else {
        "finish is already in progress".withABIString { cb(nil, $0, ctx) }
        return
    }
    wrapper.writer.finishWriting { [wrapper] in
        wrapper.withGate {
            wrapper.detachCallbacksAfterTerminalState()
            switch wrapper.writer.status {
            case .completed:
                cb(kSuccessSentinel, nil, ctx)
            case .failed:
                let msg = wrapper.writer.error?.localizedDescription ?? "writer status = failed"
                msg.withABIString { cb(nil, $0, ctx) }
            case .cancelled:
                "writer cancelled".withABIString { cb(nil, $0, ctx) }
            default:
                "unexpected writer status \(wrapper.writer.status.rawValue)".withABIString {
                    cb(nil, $0, ctx)
                }
            }
        }
    }
}

// MARK: - AVAssetWriterInput.requestMediaDataWhenReady(on:using:)

final class ReadyStreamBridge: NSObject {
    weak var writerBox: Writer?
    weak var lifetime: CallbackLifetimeBox?
    let inputId: Int32

    init?(
        writerPtr: UnsafeMutableRawPointer,
        inputId: Int32,
        cb: @escaping @convention(c) (UnsafeMutableRawPointer) -> Void,
        ctx: UnsafeMutableRawPointer,
        dropUserdata: AVWDropCallback?,
        outError: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
    ) {
        let wrapper = Writer.locked(writerPtr)
        defer { wrapper.unlock() }
        guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
            outError?.pointee = makePayload("invalid input id: \(inputId)")
            return nil
        }
        guard wrapper.writer.status == .writing else {
            outError?.pointee = makePayload(
                "request_media_data_when_ready requires a writing session"
            )
            return nil
        }
        guard !wrapper.inputIsFinished(inputId) else {
            outError?.pointee = makePayload("input \(inputId) is already finished")
            return nil
        }
        guard wrapper.readyCallbackBoxes[inputId] == nil else {
            outError?.pointee = makePayload("requestMediaDataWhenReady already registered for input \(inputId)")
            return nil
        }

        self.inputId = inputId
        super.init()
        self.writerBox = wrapper

        let input = wrapper.inputs[Int(inputId)]
        let lifetime = CallbackLifetimeBox(userdata: ctx, dropUserdata: dropUserdata)
        let owner = NativeCallbackOwner(lifetime: lifetime)
        self.lifetime = lifetime
        wrapper.readyCallbackBoxes[inputId] = lifetime
        let queue = DispatchQueue(label: "fish.doom.avassetwriter.input.ready.async.\(inputId)")
        input.requestMediaDataWhenReady(on: queue) { [weak wrapper, owner] in
            guard let wrapper else { return }
            wrapper.invokeCallback(owner.lifetime) { userdata in
                guard let userdata else { return }
                cb(userdata)
            }
        }
    }

    func cancel() {
        guard let writerBox, let lifetime else { return }
        writerBox.withGate {
            guard writerBox.readyCallbackBoxes[inputId] === lifetime else { return }
            if !writerBox.removeInputCallbacks(inputId) {
                lifetime.deactivate()
            }
        }
    }
}

@_cdecl("av_writer_input_ready_stream_subscribe")
public func av_writer_input_ready_stream_subscribe(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ cb: @convention(c) (UnsafeMutableRawPointer) -> Void,
    _ ctx: UnsafeMutableRawPointer,
    _ outError: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> UnsafeMutableRawPointer? {
    guard let bridge = ReadyStreamBridge(
        writerPtr: writerPtr,
        inputId: inputId,
        cb: cb,
        ctx: ctx,
        dropUserdata: nil,
        outError: outError
    ) else {
        return nil
    }
    return Unmanaged.passRetained(bridge).toOpaque()
}

@_cdecl("av_writer_input_ready_stream_subscribe_owned")
public func av_writer_input_ready_stream_subscribe_owned(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ cb: @convention(c) (UnsafeMutableRawPointer) -> Void,
    _ ctx: UnsafeMutableRawPointer,
    _ dropUserdata: AVWDropCallback?,
    _ outError: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> UnsafeMutableRawPointer? {
    guard let bridge = ReadyStreamBridge(
        writerPtr: writerPtr,
        inputId: inputId,
        cb: cb,
        ctx: ctx,
        dropUserdata: dropUserdata,
        outError: outError
    ) else {
        return nil
    }
    return Unmanaged.passRetained(bridge).toOpaque()
}

@_cdecl("av_writer_input_ready_stream_unsubscribe")
public func av_writer_input_ready_stream_unsubscribe(_ handle: UnsafeMutableRawPointer?) {
    guard let handle else { return }
    let bridge = Unmanaged<ReadyStreamBridge>.fromOpaque(handle).takeUnretainedValue()
    bridge.cancel()
    Unmanaged<ReadyStreamBridge>.fromOpaque(handle).release()
}

// MARK: - AVAssetExportSession.exportAsynchronouslyWithCompletionHandler:

/// Async version of `av_export_session_export`.
///
/// Calls `AVAssetExportSession.exportAsynchronouslyWithCompletionHandler:`
/// and fires `cb` once the operation completes.
///
/// Also covers `AVAssetExportSession.export(to:as:isolation:)` (the Swift
/// concurrency projection of the same API, available on macOS 26.0+).
@_cdecl("av_export_session_export_async")
public func av_export_session_export_async(
    _ sessionPtr: UnsafeMutableRawPointer,
    _ cb: @convention(c) (UnsafeRawPointer?, UnsafePointer<AVWString>?, UnsafeMutableRawPointer) -> Void,
    _ ctx: UnsafeMutableRawPointer
) {
    let session = Unmanaged<AVAssetExportSession>.fromOpaque(sessionPtr).takeUnretainedValue()

    // Guard pre-conditions that would otherwise cause exportAsynchronously to
    // raise an NSInternalInconsistencyException that cannot cross the FFI.
    guard session.outputURL != nil else {
        "export session outputURL must be set before export".withABIString { cb(nil, $0, ctx) }
        return
    }
    guard session.outputFileType != nil else {
        "export session outputFileType must be set before export".withABIString { cb(nil, $0, ctx) }
        return
    }

    session.exportAsynchronously {
        switch session.status {
        case .completed:
            cb(kSuccessSentinel, nil, ctx)
        case .failed:
            let msg = session.error?.localizedDescription ?? "export failed"
            msg.withABIString { cb(nil, $0, ctx) }
        case .cancelled:
            "export cancelled".withABIString { cb(nil, $0, ctx) }
        default:
            "export session ended in unexpected status \(session.status.rawValue)".withABIString { cb(nil, $0, ctx) }
        }
    }
}

// MARK: - AVAssetExportSession.determineCompatibleFileTypesWithCompletionHandler:

@_cdecl("av_export_session_compatible_file_types_async")
public func av_export_session_compatible_file_types_async(
    _ sessionPtr: UnsafeMutableRawPointer,
    _ cb: @convention(c) (UnsafeRawPointer?, UnsafePointer<AVWString>?, UnsafeMutableRawPointer) -> Void,
    _ ctx: UnsafeMutableRawPointer
) {
    let session = Unmanaged<AVAssetExportSession>.fromOpaque(sessionPtr).takeUnretainedValue()
    session.determineCompatibleFileTypes { fileTypes in
        let encoded = fileTypes.compactMap(encodeFileType)
        let payload = makePayload(encoded)
        cb(payload.map { UnsafeRawPointer($0) }, nil, ctx)
    }
}
