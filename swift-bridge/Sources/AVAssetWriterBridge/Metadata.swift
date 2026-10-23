import AVFoundation
import Foundation

private let metadataISO8601Formatter = ISO8601DateFormatter()

private struct MetadataGroupPayload: Codable {
    let items: [MetadataItemPayload]
    let classifyingLabel: String?
    let uniqueID: String?
}

private struct TimedMetadataGroupInfoPayload: Codable {
    let items: [MetadataItemPayload]
    let timeRange: TimeRangePayload
    let classifyingLabel: String?
    let uniqueID: String?
}

private struct TimedMetadataGroupPropertiesPayload: Codable {
    let timeRange: TimeRangePayload
    let classifyingLabel: String?
    let uniqueID: String?
}

private struct DateRangeMetadataGroupPayload: Codable {
    let items: [MetadataItemPayload]
    let startDate: String
    let endDate: String?
    let classifyingLabel: String?
    let uniqueID: String?
}

private struct DateRangeMetadataGroupPropertiesPayload: Codable {
    let startDate: String
    let endDate: String?
    let classifyingLabel: String?
    let uniqueID: String?
}

private struct KeyLoadStatusPayload: Codable {
    let key: String
    let status: Int32
    let errorMessage: String?
}

private struct MetadataItemPropertiesPayload: Codable {
    let identifier: String
    let dataType: String?
    let extendedLanguageTag: String?
    let localeIdentifier: String?
}

private final class MetadataItemValueLoaderBox {
    let callback: AVWMetadataItemValueRequestCallback?
    let userdata: UnsafeMutableRawPointer?
    let dropUserdata: AVWDropCallback?

    init(
        callback: AVWMetadataItemValueRequestCallback?,
        userdata: UnsafeMutableRawPointer?,
        dropUserdata: AVWDropCallback?
    ) {
        self.callback = callback
        self.userdata = userdata
        self.dropUserdata = dropUserdata
    }

    func emit(request: AVMetadataItemValueRequest) {
        callback?(Unmanaged.passUnretained(request).toOpaque(), userdata)
    }

    deinit {
        if let userdata, let dropUserdata {
            dropUserdata(userdata)
        }
    }
}

private func metadataItemProperties(_ item: AVMetadataItem) -> MetadataItemPropertiesPayload {
    MetadataItemPropertiesPayload(
        identifier: item.identifier?.rawValue ?? "",
        dataType: item.dataType,
        extendedLanguageTag: item.extendedLanguageTag,
        localeIdentifier: item.locale?.identifier
    )
}

private func strictMetadataValuePayload(from value: Any?) throws -> MetadataValuePayload {
    switch value {
    case let string as String:
        return MetadataValuePayload(kind: "string", stringValue: string)
    case let number as NSNumber:
        let type = String(cString: number.objCType)
        if type == "c" || type == "B" {
            return MetadataValuePayload(kind: "boolean", booleanValue: number.boolValue)
        }
        if type.contains("f") || type.contains("d") {
            return MetadataValuePayload(kind: "float", floatValue: number.doubleValue)
        }
        return MetadataValuePayload(kind: "integer", integerValue: number.int64Value)
    case let data as Data:
        return MetadataValuePayload(kind: "data", dataValue: Array(data))
    case let data as NSData:
        return MetadataValuePayload(kind: "data", dataValue: Array(data as Data))
    case nil:
        throw BridgeError.message("metadata item has no value")
    default:
        throw BridgeError.message("unsupported metadata value type: \(String(describing: type(of: value!)))")
    }
}

private func copyMetadataItem(
    _ item: AVMetadataItem,
    mutation: (AVMutableMetadataItem) -> Void
) -> AVMetadataItem {
    let mutableItem = item.mutableCopy() as! AVMutableMetadataItem
    mutation(mutableItem)
    return mutableItem.copy() as! AVMetadataItem
}

private func metadataGroupLabels(_ group: AVMetadataGroup) -> (String?, String?) {
    if #available(macOS 10.11.3, *) {
        return (group.classifyingLabel, group.uniqueID)
    }
    return (nil, nil)
}

private func encodeMetadataGroupBase(_ group: AVMetadataGroup) -> MetadataGroupPayload {
    let labels = metadataGroupLabels(group)
    return MetadataGroupPayload(
        items: group.items.map(encodeMetadataItem),
        classifyingLabel: labels.0,
        uniqueID: labels.1
    )
}

private func encodeTimedMetadataGroup(_ group: AVTimedMetadataGroup) -> TimedMetadataGroupInfoPayload {
    let base = encodeMetadataGroupBase(group)
    return TimedMetadataGroupInfoPayload(
        items: base.items,
        timeRange: encodeTimeRange(group.timeRange),
        classifyingLabel: base.classifyingLabel,
        uniqueID: base.uniqueID
    )
}

private func encodeDateRangeMetadataGroup(_ group: AVDateRangeMetadataGroup) -> DateRangeMetadataGroupPayload {
    let base = encodeMetadataGroupBase(group)
    return DateRangeMetadataGroupPayload(
        items: base.items,
        startDate: metadataISO8601Formatter.string(from: group.startDate),
        endDate: group.endDate.map(metadataISO8601Formatter.string(from:)),
        classifyingLabel: base.classifyingLabel,
        uniqueID: base.uniqueID
    )
}

private func dateRangeMetadataGroup(from payload: DateRangeMetadataGroupPayload, mutable: Bool) throws -> AVDateRangeMetadataGroup {
    let items = try payload.items.map(avMetadataItem)
    guard let startDate = metadataISO8601Formatter.date(from: payload.startDate) else {
        throw BridgeError.message("invalid ISO-8601 startDate: \(payload.startDate)")
    }
    let endDate = payload.endDate.flatMap(metadataISO8601Formatter.date(from:))
    if mutable {
        return AVMutableDateRangeMetadataGroup(items: items, start: startDate, end: endDate)
    }
    return AVDateRangeMetadataGroup(items: items, start: startDate, end: endDate)
}

public typealias AVWMetadataItemValueRequestCallback = @convention(c) (
    UnsafeMutableRawPointer?,
    UnsafeMutableRawPointer?
) -> Void

@_cdecl("av_metadata_item_create_json")
public func av_metadata_item_create_json(
    _ payloadJson: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    do {
        let payload = try decodeJson(payloadJson, as: MetadataItemPayload.self)
        let item = try avMetadataItem(from: payload)
        return Unmanaged.passRetained(item.copy() as! AVMetadataItem).toOpaque()
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_metadata_item_retain")
public func av_metadata_item_retain(
    _ itemPtr: UnsafeMutableRawPointer
) -> UnsafeMutableRawPointer {
    let item = Unmanaged<AVMetadataItem>.fromOpaque(itemPtr).takeUnretainedValue()
    return Unmanaged.passRetained(item).toOpaque()
}

@_cdecl("av_metadata_item_properties_json")
public func av_metadata_item_properties_json(
    _ itemPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutablePointer<CChar>? {
    let item = Unmanaged<AVMetadataItem>.fromOpaque(itemPtr).takeUnretainedValue()
    do {
        return ffiString(try encodeJson(metadataItemProperties(item)))
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_metadata_item_value_json")
public func av_metadata_item_value_json(
    _ itemPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutablePointer<CChar>? {
    let item = Unmanaged<AVMetadataItem>.fromOpaque(itemPtr).takeUnretainedValue()
    do {
        return ffiString(try encodeJson(strictMetadataValuePayload(from: item.value)))
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_metadata_item_copy_with_data_type")
public func av_metadata_item_copy_with_data_type(
    _ itemPtr: UnsafeMutableRawPointer,
    _ bytes: UnsafePointer<UInt8>?,
    _ count: Int
) -> UnsafeMutableRawPointer {
    let item = Unmanaged<AVMetadataItem>.fromOpaque(itemPtr).takeUnretainedValue()
    let dataType = stringFromUTF8(bytes, count: count)
    return Unmanaged.passRetained(copyMetadataItem(item) { $0.dataType = dataType }).toOpaque()
}

@_cdecl("av_metadata_item_copy_with_extended_language_tag")
public func av_metadata_item_copy_with_extended_language_tag(
    _ itemPtr: UnsafeMutableRawPointer,
    _ bytes: UnsafePointer<UInt8>?,
    _ count: Int
) -> UnsafeMutableRawPointer {
    let item = Unmanaged<AVMetadataItem>.fromOpaque(itemPtr).takeUnretainedValue()
    let tag = stringFromUTF8(bytes, count: count)
    return Unmanaged.passRetained(copyMetadataItem(item) { $0.extendedLanguageTag = tag }).toOpaque()
}

@_cdecl("av_metadata_item_copy_with_locale_identifier")
public func av_metadata_item_copy_with_locale_identifier(
    _ itemPtr: UnsafeMutableRawPointer,
    _ bytes: UnsafePointer<UInt8>?,
    _ count: Int
) -> UnsafeMutableRawPointer {
    let item = Unmanaged<AVMetadataItem>.fromOpaque(itemPtr).takeUnretainedValue()
    let identifier = stringFromUTF8(bytes, count: count)
    return Unmanaged.passRetained(copyMetadataItem(item) {
        $0.locale = Locale(identifier: identifier)
    }).toOpaque()
}

@_cdecl("av_metadata_item_info_json")
public func av_metadata_item_info_json(
    _ itemPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutablePointer<CChar>? {
    let item = Unmanaged<AVMetadataItem>.fromOpaque(itemPtr).takeUnretainedValue()
    do {
        return ffiString(try encodeJson(encodeMetadataItem(item)))
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_metadata_item_release")
public func av_metadata_item_release(_ itemPtr: UnsafeMutableRawPointer?) {
    guard let itemPtr else { return }
    Unmanaged<AVMetadataItem>.fromOpaque(itemPtr).release()
}

@_cdecl("av_metadata_item_array_count")
public func av_metadata_item_array_count(_ arrayPtr: UnsafeMutableRawPointer) -> Int {
    Unmanaged<NSArray>.fromOpaque(arrayPtr).takeUnretainedValue().count
}

@_cdecl("av_metadata_item_array_item_retain")
public func av_metadata_item_array_item_retain(
    _ arrayPtr: UnsafeMutableRawPointer,
    _ index: Int
) -> UnsafeMutableRawPointer? {
    let array = Unmanaged<NSArray>.fromOpaque(arrayPtr).takeUnretainedValue()
    guard index >= 0, index < array.count, let item = array[index] as? AVMetadataItem else {
        return nil
    }
    return Unmanaged.passRetained(item).toOpaque()
}

@_cdecl("av_metadata_item_array_release")
public func av_metadata_item_array_release(_ arrayPtr: UnsafeMutableRawPointer?) {
    guard let arrayPtr else { return }
    Unmanaged<NSArray>.fromOpaque(arrayPtr).release()
}

@_cdecl("av_metadata_item_status_of_value")
public func av_metadata_item_status_of_value(
    _ itemPtr: UnsafeMutableRawPointer,
    _ keyPtr: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let item = Unmanaged<AVMetadataItem>.fromOpaque(itemPtr).takeUnretainedValue()
    let key = String(cString: keyPtr)
    var error: NSError?
    let status = item.statusOfValue(forKey: key, error: &error)
    if let error, status == .failed {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
    }
    return Int32(status.rawValue)
}

@_cdecl("av_metadata_item_load_values_json")
public func av_metadata_item_load_values_json(
    _ itemPtr: UnsafeMutableRawPointer,
    _ keysJson: UnsafePointer<CChar>,
    _ timeoutSeconds: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutablePointer<CChar>? {
    let item = Unmanaged<AVMetadataItem>.fromOpaque(itemPtr).takeUnretainedValue()
    do {
        let keys = try decodeJson(keysJson, as: [String].self)
        let semaphore = DispatchSemaphore(value: 0)
        item.loadValuesAsynchronously(forKeys: keys) {
            semaphore.signal()
        }
        let timeout = max(Int(timeoutSeconds), 1)
        if semaphore.wait(timeout: .now() + .seconds(timeout)) == .timedOut {
            outErrorMessage?.pointee = ffiString("timed out waiting for AVMetadataItem key loading")
            return nil
        }
        let statuses = keys.map { key -> KeyLoadStatusPayload in
            var error: NSError?
            let status = item.statusOfValue(forKey: key, error: &error)
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

@_cdecl("av_metadata_item_filter_preferred_languages_json")
public func av_metadata_item_filter_preferred_languages_json(
    _ itemsJson: UnsafePointer<CChar>,
    _ languagesJson: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutablePointer<CChar>? {
    do {
        let payloads = try decodeJson(itemsJson, as: [MetadataItemPayload].self)
        let languages = try decodeJson(languagesJson, as: [String].self)
        let items = try payloads.map(avMetadataItem)
        let filtered = AVMetadataItem.metadataItems(
            from: items,
            filteredAndSortedAccordingToPreferredLanguages: languages
        )
        return ffiString(try encodeJson(filtered.map(encodeMetadataItem)))
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_metadata_item_filter_preferred_languages")
public func av_metadata_item_filter_preferred_languages(
    _ itemPointers: UnsafePointer<UnsafeMutableRawPointer?>?,
    _ itemCount: Int,
    _ languagesJson: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    do {
        let items = try metadataItems(from: itemPointers, count: itemCount)
        let languages = try decodeJson(languagesJson, as: [String].self)
        return retainedMetadataItemArray(AVMetadataItem.metadataItems(
            from: items,
            filteredAndSortedAccordingToPreferredLanguages: languages
        ))
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_metadata_item_filter_identifier_json")
public func av_metadata_item_filter_identifier_json(
    _ itemsJson: UnsafePointer<CChar>,
    _ identifierPtr: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutablePointer<CChar>? {
    do {
        let payloads = try decodeJson(itemsJson, as: [MetadataItemPayload].self)
        let items = try payloads.map(avMetadataItem)
        let identifier = AVMetadataIdentifier(rawValue: String(cString: identifierPtr))
        let filtered = AVMetadataItem.metadataItems(from: items, filteredByIdentifier: identifier)
        return ffiString(try encodeJson(filtered.map(encodeMetadataItem)))
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_metadata_item_filter_identifier")
public func av_metadata_item_filter_identifier(
    _ itemPointers: UnsafePointer<UnsafeMutableRawPointer?>?,
    _ itemCount: Int,
    _ identifierPtr: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    do {
        let items = try metadataItems(from: itemPointers, count: itemCount)
        let identifier = AVMetadataIdentifier(rawValue: String(cString: identifierPtr))
        return retainedMetadataItemArray(
            AVMetadataItem.metadataItems(from: items, filteredByIdentifier: identifier)
        )
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_metadata_item_filter_metadata_item_filter_json")
public func av_metadata_item_filter_metadata_item_filter_json(
    _ itemsJson: UnsafePointer<CChar>,
    _ filterPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutablePointer<CChar>? {
    let filter = Unmanaged<AVMetadataItemFilter>.fromOpaque(filterPtr).takeUnretainedValue()
    do {
        let payloads = try decodeJson(itemsJson, as: [MetadataItemPayload].self)
        let items = try payloads.map(avMetadataItem)
        let filtered = AVMetadataItem.metadataItems(from: items, filteredBy: filter)
        return ffiString(try encodeJson(filtered.map(encodeMetadataItem)))
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_metadata_item_filter_metadata_item_filter")
public func av_metadata_item_filter_metadata_item_filter(
    _ itemPointers: UnsafePointer<UnsafeMutableRawPointer?>?,
    _ itemCount: Int,
    _ filterPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    let filter = Unmanaged<AVMetadataItemFilter>.fromOpaque(filterPtr).takeUnretainedValue()
    do {
        let items = try metadataItems(from: itemPointers, count: itemCount)
        return retainedMetadataItemArray(AVMetadataItem.metadataItems(from: items, filteredBy: filter))
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_metadata_item_create_lazy_json")
public func av_metadata_item_create_lazy_json(
    _ baseItemJson: UnsafePointer<CChar>,
    _ callback: AVWMetadataItemValueRequestCallback?,
    _ userdata: UnsafeMutableRawPointer?,
    _ dropUserdata: AVWDropCallback?,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    do {
        let payload = try decodeJson(baseItemJson, as: MetadataItemPayload.self)
        let baseItem = try avMetadataItem(from: payload)
        let box = MetadataItemValueLoaderBox(callback: callback, userdata: userdata, dropUserdata: dropUserdata)
        let item = AVMetadataItem(propertiesOf: baseItem) { request in
            box.emit(request: request)
        }
        return Unmanaged.passRetained(item).toOpaque()
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_metadata_item_create_lazy")
public func av_metadata_item_create_lazy(
    _ baseItemPtr: UnsafeMutableRawPointer,
    _ callback: AVWMetadataItemValueRequestCallback?,
    _ userdata: UnsafeMutableRawPointer?,
    _ dropUserdata: AVWDropCallback?,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    let baseItem = Unmanaged<AVMetadataItem>.fromOpaque(baseItemPtr).takeUnretainedValue()
    let box = MetadataItemValueLoaderBox(
        callback: callback,
        userdata: userdata,
        dropUserdata: dropUserdata
    )
    let item = AVMetadataItem(propertiesOf: baseItem) { request in
        box.emit(request: request)
    }
    _ = outErrorMessage
    return Unmanaged.passRetained(item).toOpaque()
}

@_cdecl("av_metadata_item_value_request_retain")
public func av_metadata_item_value_request_retain(
    _ requestPtr: UnsafeMutableRawPointer
) -> UnsafeMutableRawPointer {
    let request = Unmanaged<AVMetadataItemValueRequest>.fromOpaque(requestPtr).takeUnretainedValue()
    return Unmanaged.passRetained(request).toOpaque()
}

@_cdecl("av_metadata_item_value_request_release")
public func av_metadata_item_value_request_release(_ requestPtr: UnsafeMutableRawPointer?) {
    guard let requestPtr else { return }
    Unmanaged<AVMetadataItemValueRequest>.fromOpaque(requestPtr).release()
}

@_cdecl("av_metadata_item_value_request_metadata_item_json")
public func av_metadata_item_value_request_metadata_item_json(
    _ requestPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutablePointer<CChar>? {
    let request = Unmanaged<AVMetadataItemValueRequest>.fromOpaque(requestPtr).takeUnretainedValue()
    guard let item = request.metadataItem else {
        return nil
    }
    do {
        return ffiString(try encodeJson(encodeMetadataItem(item)))
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_metadata_item_value_request_metadata_item")
public func av_metadata_item_value_request_metadata_item(
    _ requestPtr: UnsafeMutableRawPointer
) -> UnsafeMutableRawPointer? {
    let request = Unmanaged<AVMetadataItemValueRequest>.fromOpaque(requestPtr).takeUnretainedValue()
    guard let item = request.metadataItem else {
        return nil
    }
    return Unmanaged.passRetained(item).toOpaque()
}

@_cdecl("av_metadata_item_value_request_respond_with_value_json")
public func av_metadata_item_value_request_respond_with_value_json(
    _ requestPtr: UnsafeMutableRawPointer,
    _ valueJson: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let request = Unmanaged<AVMetadataItemValueRequest>.fromOpaque(requestPtr).takeUnretainedValue()
    do {
        let value = try decodeJson(valueJson, as: MetadataValuePayload.self)
        request.respond(value: try metadataValueObject(from: value))
        return AVW_OK
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return AVW_INVALID_ARGUMENT
    }
}

@_cdecl("av_metadata_item_value_request_respond_with_error")
public func av_metadata_item_value_request_respond_with_error(
    _ requestPtr: UnsafeMutableRawPointer,
    _ errorMessagePtr: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let request = Unmanaged<AVMetadataItemValueRequest>.fromOpaque(requestPtr).takeUnretainedValue()
    request.respond(error: NSError(
        domain: "avassetwriter",
        code: -1,
        userInfo: [NSLocalizedDescriptionKey: String(cString: errorMessagePtr)]
    ))
    _ = outErrorMessage
    return AVW_OK
}

@_cdecl("av_timed_metadata_group_create_json")
public func av_timed_metadata_group_create_json(
    _ payloadJson: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    do {
        let payload = try decodeJson(payloadJson, as: TimedMetadataGroupPayload.self)
        return Unmanaged.passRetained(try timedMetadataGroup(from: payload)).toOpaque()
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_timed_metadata_group_create_items")
public func av_timed_metadata_group_create_items(
    _ itemPointers: UnsafePointer<UnsafeMutableRawPointer?>?,
    _ itemCount: Int,
    _ startValue: Int64,
    _ startScale: Int32,
    _ startKind: Int32,
    _ durationValue: Int64,
    _ durationScale: Int32,
    _ durationKind: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    do {
        let items = try metadataItems(from: itemPointers, count: itemCount)
        let timeRange = CMTimeRange(
            start: cmTime(value: startValue, timescale: startScale, kind: startKind),
            duration: cmTime(value: durationValue, timescale: durationScale, kind: durationKind)
        )
        return Unmanaged.passRetained(
            AVTimedMetadataGroup(items: items, timeRange: timeRange)
        ).toOpaque()
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_timed_metadata_group_retain")
public func av_timed_metadata_group_retain(
    _ groupPtr: UnsafeMutableRawPointer
) -> UnsafeMutableRawPointer {
    let group = Unmanaged<AVTimedMetadataGroup>.fromOpaque(groupPtr).takeUnretainedValue()
    return Unmanaged.passRetained(group).toOpaque()
}

@_cdecl("av_metadata_group_items")
public func av_metadata_group_items(
    _ groupPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    let object = Unmanaged<NSObject>.fromOpaque(groupPtr).takeUnretainedValue()
    guard let group = object as? AVMetadataGroup else {
        outErrorMessage?.pointee = ffiString("object is not an AVMetadataGroup")
        return nil
    }
    return retainedMetadataItemArray(group.items)
}

@_cdecl("av_timed_metadata_group_properties_json")
public func av_timed_metadata_group_properties_json(
    _ groupPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutablePointer<CChar>? {
    let group = Unmanaged<AVTimedMetadataGroup>.fromOpaque(groupPtr).takeUnretainedValue()
    let labels = metadataGroupLabels(group)
    do {
        return ffiString(try encodeJson(TimedMetadataGroupPropertiesPayload(
            timeRange: encodeTimeRange(group.timeRange),
            classifyingLabel: labels.0,
            uniqueID: labels.1
        )))
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("avw_timed_metadata_group_info_json")
public func avw_timed_metadata_group_info_json(
    _ groupPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutablePointer<CChar>? {
    let group = Unmanaged<AVTimedMetadataGroup>.fromOpaque(groupPtr).takeUnretainedValue()
    do {
        return ffiString(try encodeJson(encodeTimedMetadataGroup(group)))
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_timed_metadata_group_release")
public func av_timed_metadata_group_release(_ groupPtr: UnsafeMutableRawPointer?) {
    guard let groupPtr else { return }
    Unmanaged<AVTimedMetadataGroup>.fromOpaque(groupPtr).release()
}

@_cdecl("av_date_range_metadata_group_create_json")
public func av_date_range_metadata_group_create_json(
    _ payloadJson: UnsafePointer<CChar>,
    _ mutableGroup: Bool,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    do {
        let payload = try decodeJson(payloadJson, as: DateRangeMetadataGroupPayload.self)
        return Unmanaged.passRetained(try dateRangeMetadataGroup(from: payload, mutable: mutableGroup)).toOpaque()
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_date_range_metadata_group_create_items")
public func av_date_range_metadata_group_create_items(
    _ itemPointers: UnsafePointer<UnsafeMutableRawPointer?>?,
    _ itemCount: Int,
    _ startDatePtr: UnsafePointer<CChar>,
    _ endDatePtr: UnsafePointer<CChar>?,
    _ mutableGroup: Bool,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    do {
        let items = try metadataItems(from: itemPointers, count: itemCount)
        let startDateRaw = String(cString: startDatePtr)
        guard let startDate = metadataISO8601Formatter.date(from: startDateRaw) else {
            throw BridgeError.message("invalid ISO-8601 startDate: \(startDateRaw)")
        }
        let endDate = endDatePtr.flatMap {
            metadataISO8601Formatter.date(from: String(cString: $0))
        }
        if mutableGroup {
            let group = AVMutableDateRangeMetadataGroup(
                items: items,
                start: startDate,
                end: endDate
            )
            return Unmanaged.passRetained(group).toOpaque()
        }
        let group = AVDateRangeMetadataGroup(items: items, start: startDate, end: endDate)
        return Unmanaged.passRetained(group).toOpaque()
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_date_range_metadata_group_retain")
public func av_date_range_metadata_group_retain(
    _ groupPtr: UnsafeMutableRawPointer
) -> UnsafeMutableRawPointer {
    let group = Unmanaged<NSObject>.fromOpaque(groupPtr).takeUnretainedValue()
    return Unmanaged.passRetained(group).toOpaque()
}

@_cdecl("av_date_range_metadata_group_mutable_copy")
public func av_date_range_metadata_group_mutable_copy(
    _ groupPtr: UnsafeMutableRawPointer
) -> UnsafeMutableRawPointer {
    let group = Unmanaged<AVDateRangeMetadataGroup>.fromOpaque(groupPtr).takeUnretainedValue()
    let copy = AVMutableDateRangeMetadataGroup(
        items: group.items,
        start: group.startDate,
        end: group.endDate
    )
    return Unmanaged.passRetained(copy).toOpaque()
}

@_cdecl("av_date_range_metadata_group_immutable_copy")
public func av_date_range_metadata_group_immutable_copy(
    _ groupPtr: UnsafeMutableRawPointer
) -> UnsafeMutableRawPointer {
    let group = Unmanaged<AVDateRangeMetadataGroup>.fromOpaque(groupPtr).takeUnretainedValue()
    let copy = AVDateRangeMetadataGroup(
        items: group.items,
        start: group.startDate,
        end: group.endDate
    )
    return Unmanaged.passRetained(copy).toOpaque()
}

@_cdecl("av_date_range_metadata_group_properties_json")
public func av_date_range_metadata_group_properties_json(
    _ groupPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutablePointer<CChar>? {
    let group = Unmanaged<AVDateRangeMetadataGroup>.fromOpaque(groupPtr).takeUnretainedValue()
    let labels = metadataGroupLabels(group)
    do {
        return ffiString(try encodeJson(DateRangeMetadataGroupPropertiesPayload(
            startDate: metadataISO8601Formatter.string(from: group.startDate),
            endDate: group.endDate.map(metadataISO8601Formatter.string(from:)),
            classifyingLabel: labels.0,
            uniqueID: labels.1
        )))
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("avw_date_range_metadata_group_info_json")
public func avw_date_range_metadata_group_info_json(
    _ groupPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutablePointer<CChar>? {
    let object = Unmanaged<NSObject>.fromOpaque(groupPtr).takeUnretainedValue()
    guard let group = object as? AVDateRangeMetadataGroup else {
        outErrorMessage?.pointee = ffiString("object is not an AVDateRangeMetadataGroup")
        return nil
    }
    do {
        return ffiString(try encodeJson(encodeDateRangeMetadataGroup(group)))
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_date_range_metadata_group_release")
public func av_date_range_metadata_group_release(_ groupPtr: UnsafeMutableRawPointer?) {
    guard let groupPtr else { return }
    Unmanaged<NSObject>.fromOpaque(groupPtr).release()
}
