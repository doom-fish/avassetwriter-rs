import AVAssetWriterObjCBridge
import AVFoundation
import Foundation

private struct AssetInfoPayload {
    let url: String?
    let duration: TimePayload
}

private struct KeyLoadStatusPayload {
    let key: String
    let status: Int32
    let errorMessage: String?
}

@_cdecl("av_asset_create_url")
public func av_asset_create_url(
    _ urlPtr: UnsafePointer<AVWString>,
    _ isFileURL: Bool,
    _ preferPreciseDurationAndTiming: Bool,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> UnsafeMutableRawPointer? {
    let urlString = String(abi: urlPtr)
    let url = isFileURL ? URL(fileURLWithPath: urlString) : URL(string: urlString)
    guard let url else {
        outErrorMessage?.pointee = makePayload("invalid URL: \(urlString)")
        return nil
    }
    let asset = AVURLAsset(
        url: url,
        options: [AVURLAssetPreferPreciseDurationAndTimingKey: preferPreciseDurationAndTiming]
    )
    return Unmanaged.passRetained(asset).toOpaque()
}

@_cdecl("avw_asset_release")
public func avw_asset_release(_ assetPtr: UnsafeMutableRawPointer?) {
    guard let assetPtr else { return }
    Unmanaged<AVAsset>.fromOpaque(assetPtr).release()
}

@_cdecl("avw_asset_info_payload")
public func avw_asset_info_payload(
    _ assetPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> UnsafeMutablePointer<AVWOwnedPayload>? {
    let asset = Unmanaged<AVAsset>.fromOpaque(assetPtr).takeUnretainedValue()
    do {
        return makePayload(AssetInfoPayload(
            url: (asset as? AVURLAsset)?.url.absoluteString,
            duration: encodeTime(asset.duration)
        ))
    } catch {
        outErrorMessage?.pointee = makePayload(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_asset_metadata")
public func av_asset_metadata(
    _ assetPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> UnsafeMutableRawPointer? {
    let asset = Unmanaged<AVAsset>.fromOpaque(assetPtr).takeUnretainedValue()
    _ = outErrorMessage
    return retainedMetadataItemArray(asset.metadata)
}

@_cdecl("avw_asset_status_of_value")
public func avw_asset_status_of_value(
    _ assetPtr: UnsafeMutableRawPointer,
    _ keyPtr: UnsafePointer<AVWString>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let asset = Unmanaged<AVAsset>.fromOpaque(assetPtr).takeUnretainedValue()
    let key = String(abi: keyPtr)
    var error: NSError?
    let status = asset.statusOfValue(forKey: key, error: &error)
    if let error, status == .failed {
        outErrorMessage?.pointee = makePayload(error.localizedDescription)
    }
    return Int32(status.rawValue)
}

@_cdecl("avw_asset_load_values_payload")
public func avw_asset_load_values_payload(
    _ assetPtr: UnsafeMutableRawPointer,
    _ keysPayload: UnsafePointer<AVWArray>,
    _ timeoutSeconds: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> UnsafeMutablePointer<AVWOwnedPayload>? {
    let asset = Unmanaged<AVAsset>.fromOpaque(assetPtr).takeUnretainedValue()
    do {
        let keys = try readPayload(keysPayload, as: [String].self)
        let semaphore = DispatchSemaphore(value: 0)
        asset.loadValuesAsynchronously(forKeys: keys) {
            semaphore.signal()
        }
        let timeout = max(Int(timeoutSeconds), 1)
        if semaphore.wait(timeout: .now() + .seconds(timeout)) == .timedOut {
            outErrorMessage?.pointee = makePayload("timed out waiting for AVAsset key loading")
            return nil
        }
        let statuses = keys.map { key -> KeyLoadStatusPayload in
            var error: NSError?
            let status = asset.statusOfValue(forKey: key, error: &error)
            return KeyLoadStatusPayload(
                key: key,
                status: Int32(status.rawValue),
                errorMessage: error?.localizedDescription
            )
        }
        return makePayload(statuses)
    } catch {
        outErrorMessage?.pointee = makePayload(error.localizedDescription)
        return nil
    }
}

extension KeyLoadStatusPayload: BridgePayload {
    typealias ABI = AVWKeyLoadStatus

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            key: key.toABI(arena),
            status: status.toABI(arena),
            error_message: errorMessage.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            key: try .fromABI(raw.key),
            status: try .fromABI(raw.status),
            errorMessage: try .fromABI(raw.error_message)
        )
    }
}

extension AssetInfoPayload: BridgePayload {
    typealias ABI = AVWAssetInfo

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            url: url.toABI(arena),
            duration: duration.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            url: try .fromABI(raw.url),
            duration: try .fromABI(raw.duration)
        )
    }
}
