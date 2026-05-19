import AVFoundation
import Foundation

private struct AssetInfoPayload: Codable {
    let url: String?
    let duration: TimePayload
    let metadata: [MetadataItemPayload]
}

private struct KeyLoadStatusPayload: Codable {
    let key: String
    let status: Int32
    let errorMessage: String?
}

@_cdecl("av_asset_create_url")
public func av_asset_create_url(
    _ urlPtr: UnsafePointer<CChar>,
    _ isFileURL: Bool,
    _ preferPreciseDurationAndTiming: Bool,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    let urlString = String(cString: urlPtr)
    let url = isFileURL ? URL(fileURLWithPath: urlString) : URL(string: urlString)
    guard let url else {
        outErrorMessage?.pointee = ffiString("invalid URL: \(urlString)")
        return nil
    }
    let asset = AVURLAsset(
        url: url,
        options: [AVURLAssetPreferPreciseDurationAndTimingKey: preferPreciseDurationAndTiming]
    )
    return Unmanaged.passRetained(asset).toOpaque()
}

@_cdecl("av_asset_release")
public func av_asset_release(_ assetPtr: UnsafeMutableRawPointer?) {
    guard let assetPtr else { return }
    Unmanaged<AVAsset>.fromOpaque(assetPtr).release()
}

@_cdecl("av_asset_info_json")
public func av_asset_info_json(
    _ assetPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutablePointer<CChar>? {
    let asset = Unmanaged<AVAsset>.fromOpaque(assetPtr).takeUnretainedValue()
    do {
        return ffiString(try encodeJson(AssetInfoPayload(
            url: (asset as? AVURLAsset)?.url.absoluteString,
            duration: encodeTime(asset.duration),
            metadata: asset.metadata.map(encodeMetadataItem)
        )))
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_asset_status_of_value")
public func av_asset_status_of_value(
    _ assetPtr: UnsafeMutableRawPointer,
    _ keyPtr: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let asset = Unmanaged<AVAsset>.fromOpaque(assetPtr).takeUnretainedValue()
    let key = String(cString: keyPtr)
    var error: NSError?
    let status = asset.statusOfValue(forKey: key, error: &error)
    if let error, status == .failed {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
    }
    return Int32(status.rawValue)
}

@_cdecl("av_asset_load_values_json")
public func av_asset_load_values_json(
    _ assetPtr: UnsafeMutableRawPointer,
    _ keysJson: UnsafePointer<CChar>,
    _ timeoutSeconds: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutablePointer<CChar>? {
    let asset = Unmanaged<AVAsset>.fromOpaque(assetPtr).takeUnretainedValue()
    do {
        let keys = try decodeJson(keysJson, as: [String].self)
        let semaphore = DispatchSemaphore(value: 0)
        asset.loadValuesAsynchronously(forKeys: keys) {
            semaphore.signal()
        }
        let timeout = max(Int(timeoutSeconds), 1)
        if semaphore.wait(timeout: .now() + .seconds(timeout)) == .timedOut {
            outErrorMessage?.pointee = ffiString("timed out waiting for AVAsset key loading")
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
        return ffiString(try encodeJson(statuses))
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}
