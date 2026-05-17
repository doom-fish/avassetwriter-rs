// Async.swift — callback-based FFI thunks wrapping AVAssetWriter and
// AVAssetExportSession completion-handler APIs.
//
// Each thunk takes a C callback `cb` and opaque `ctx` pointer, fires the
// underlying Apple completion handler, then calls `cb(result, error, ctx)`
// exactly once when the operation finishes.  The Rust side converts the
// callback into a `std::future::Future` via `doom_fish_utils::completion`.
//
// Tier-2 note: AVAssetWriterInput.requestMediaDataWhenReady(on:using:) is a
// *multi-fire* handler (fires once per ready window, not once per operation)
// and belongs in a Stream/channel pattern (Tier 2), not here.
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
