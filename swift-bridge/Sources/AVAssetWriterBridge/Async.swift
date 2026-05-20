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
    _ cb: @convention(c) (UnsafeRawPointer?, UnsafePointer<CChar>?, UnsafeMutableRawPointer) -> Void,
    _ ctx: UnsafeMutableRawPointer
) {
    let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()

    // Guard: finishWritingWithCompletionHandler: must only be called when the
    // writer is in .writing status.  Any other status is reported immediately
    // without invoking the underlying API so that no Objective-C exception can
    // cross the Swift→Rust FFI boundary.
    guard wrapper.writer.status == .writing else {
        switch wrapper.writer.status {
        case .completed:
            cb(kSuccessSentinel, nil, ctx)
        case .failed:
            let msg = wrapper.writer.error?.localizedDescription ?? "writer status = failed"
            msg.withCString { cb(nil, $0, ctx) }
        case .cancelled:
            "writer cancelled".withCString { cb(nil, $0, ctx) }
        default:
            let raw = wrapper.writer.status.rawValue
            "writer not in writing state (status=\(raw)); call start_session first".withCString {
                cb(nil, $0, ctx)
            }
        }
        return
    }

    for input in wrapper.inputs {
        input.markAsFinished()
    }
    wrapper.writer.finishWriting {
        switch wrapper.writer.status {
        case .completed:
            cb(kSuccessSentinel, nil, ctx)
        case .failed:
            let msg = wrapper.writer.error?.localizedDescription ?? "writer status = failed"
            msg.withCString { cb(nil, $0, ctx) }
        case .cancelled:
            "writer cancelled".withCString { cb(nil, $0, ctx) }
        default:
            "unexpected writer status \(wrapper.writer.status.rawValue)".withCString { cb(nil, $0, ctx) }
        }
    }
}

// MARK: - AVAssetWriterInput.requestMediaDataWhenReady(on:using:)

final class ReadyStreamBridge: NSObject {
    let writer: AVAssetWriter
    let input: AVAssetWriterInput
    let inputId: Int32
    let cb: @convention(c) (UnsafeMutableRawPointer) -> Void
    let ctx: UnsafeMutableRawPointer
    let queue: DispatchQueue
    let queueKey = DispatchSpecificKey<UInt8>()
    var active = true

    init?(
        writerPtr: UnsafeMutableRawPointer,
        inputId: Int32,
        cb: @escaping @convention(c) (UnsafeMutableRawPointer) -> Void,
        ctx: UnsafeMutableRawPointer,
        outError: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
    ) {
        let wrapper = Unmanaged<Writer>.fromOpaque(writerPtr).takeUnretainedValue()
        guard inputId >= 0, Int(inputId) < wrapper.inputs.count else {
            outError?.pointee = ffiString("invalid input id: \(inputId)")
            return nil
        }
        guard wrapper.readyCallbackBoxes[inputId] == nil else {
            outError?.pointee = ffiString("requestMediaDataWhenReady already registered for input \(inputId)")
            return nil
        }

        self.writer = wrapper.writer
        self.input = wrapper.inputs[Int(inputId)]
        self.inputId = inputId
        self.cb = cb
        self.ctx = ctx
        self.queue = DispatchQueue(label: "fish.doom.avassetwriter.input.ready.async.\(inputId)")
        super.init()

        queue.setSpecific(key: queueKey, value: 1)
        wrapper.readyCallbackBoxes[inputId] = InputCallbackBox(userdata: nil, dropUserdata: nil)
        input.requestMediaDataWhenReady(on: queue) { [weak self] in
            guard let self, self.active else { return }
            self.cb(self.ctx)
        }
    }

    func cancel() {
        let deactivate = { self.active = false }
        if DispatchQueue.getSpecific(key: queueKey) != nil {
            deactivate()
        } else {
            queue.sync(execute: deactivate)
        }
    }
}

@_cdecl("av_writer_input_ready_stream_subscribe")
public func av_writer_input_ready_stream_subscribe(
    _ writerPtr: UnsafeMutableRawPointer,
    _ inputId: Int32,
    _ cb: @convention(c) (UnsafeMutableRawPointer) -> Void,
    _ ctx: UnsafeMutableRawPointer,
    _ outError: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    guard let bridge = ReadyStreamBridge(
        writerPtr: writerPtr,
        inputId: inputId,
        cb: cb,
        ctx: ctx,
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
    _ cb: @convention(c) (UnsafeRawPointer?, UnsafePointer<CChar>?, UnsafeMutableRawPointer) -> Void,
    _ ctx: UnsafeMutableRawPointer
) {
    let session = Unmanaged<AVAssetExportSession>.fromOpaque(sessionPtr).takeUnretainedValue()

    // Guard pre-conditions that would otherwise cause exportAsynchronously to
    // raise an NSInternalInconsistencyException that cannot cross the FFI.
    guard session.outputURL != nil else {
        "export session outputURL must be set before export".withCString { cb(nil, $0, ctx) }
        return
    }
    guard session.outputFileType != nil else {
        "export session outputFileType must be set before export".withCString { cb(nil, $0, ctx) }
        return
    }

    session.exportAsynchronously {
        switch session.status {
        case .completed:
            cb(kSuccessSentinel, nil, ctx)
        case .failed:
            let msg = session.error?.localizedDescription ?? "export failed"
            msg.withCString { cb(nil, $0, ctx) }
        case .cancelled:
            "export cancelled".withCString { cb(nil, $0, ctx) }
        default:
            "export session ended in unexpected status \(session.status.rawValue)".withCString { cb(nil, $0, ctx) }
        }
    }
}

// MARK: - AVAssetExportSession.determineCompatibleFileTypesWithCompletionHandler:

/// Async version of `av_export_session_compatible_file_types_json`.
///
/// Calls `AVAssetExportSession.determineCompatibleFileTypesWithCompletionHandler:`
/// and fires `cb(jsonCStrPtr, nil, ctx)` on success or
/// `cb(nil, errorCStr, ctx)` on error.
///
/// On success the `result` pointer is a heap-allocated C string (produced by
/// `ffiString`) that **must** be freed by the Rust side via `avw_string_free`.
@_cdecl("av_export_session_compatible_file_types_async")
public func av_export_session_compatible_file_types_async(
    _ sessionPtr: UnsafeMutableRawPointer,
    _ cb: @convention(c) (UnsafeRawPointer?, UnsafePointer<CChar>?, UnsafeMutableRawPointer) -> Void,
    _ ctx: UnsafeMutableRawPointer
) {
    let session = Unmanaged<AVAssetExportSession>.fromOpaque(sessionPtr).takeUnretainedValue()
    session.determineCompatibleFileTypes { fileTypes in
        let encoded = fileTypes.compactMap(encodeFileType)
        do {
            guard let jsonPtr = ffiString(try encodeJson(encoded)) else {
                "failed to allocate JSON string".withCString { cb(nil, $0, ctx) }
                return
            }
            cb(UnsafeRawPointer(jsonPtr), nil, ctx)
        } catch {
            error.localizedDescription.withCString { cb(nil, $0, ctx) }
        }
    }
}
