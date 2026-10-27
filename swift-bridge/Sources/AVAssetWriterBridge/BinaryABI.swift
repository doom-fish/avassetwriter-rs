import AVAssetWriterObjCBridge
import Foundation
import CoreFoundation

protocol BridgePayload {
    associatedtype ABI
    func toABI(_ arena: PayloadArena) -> ABI
    static func fromABI(_ raw: ABI) throws -> Self
}

final class PayloadArena {
    private var allocations: [() -> Void] = []

    func allocate<T>(_ value: T) -> UnsafePointer<T> {
        let pointer = UnsafeMutablePointer<T>.allocate(capacity: 1)
        pointer.initialize(to: value)
        allocations.append {
            pointer.deinitialize(count: 1)
            pointer.deallocate()
        }
        return UnsafePointer(pointer)
    }

    func array<T>(_ values: [T]) -> AVWArray {
        guard !values.isEmpty else { return AVWArray(data: nil, length: 0) }
        let pointer = UnsafeMutablePointer<T>.allocate(capacity: values.count)
        pointer.initialize(from: values, count: values.count)
        allocations.append {
            pointer.deinitialize(count: values.count)
            pointer.deallocate()
        }
        return AVWArray(data: UnsafeRawPointer(pointer), length: values.count)
    }

    deinit {
        for release in allocations.reversed() {
            release()
        }
    }
}

func makePayload<T: BridgePayload>(_ value: T) -> UnsafeMutablePointer<AVWOwnedPayload>? {
    let arena = PayloadArena()
    let raw = value.toABI(arena)
    let pointer = arena.allocate(raw)
    let result = UnsafeMutablePointer<AVWOwnedPayload>.allocate(capacity: 1)
    result.initialize(to: AVWOwnedPayload(
        value: UnsafeRawPointer(pointer),
        owner: Unmanaged.passRetained(arena).toOpaque()
    ))
    return result
}

@_cdecl("avw_payload_free")
public func avw_payload_free(_ pointer: UnsafeMutablePointer<AVWOwnedPayload>?) {
    guard let pointer else { return }
    if let owner = pointer.pointee.owner {
        Unmanaged<PayloadArena>.fromOpaque(owner).release()
    }
    pointer.deinitialize(count: 1)
    pointer.deallocate()
}

func readPayload<T: BridgePayload>(
    _ pointer: UnsafePointer<T.ABI>?,
    as type: T.Type
) throws -> T {
    guard let pointer else { throw BridgeError.message("missing binary payload") }
    return try T.fromABI(pointer.pointee)
}

extension String: BridgePayload {
    typealias ABI = AVWString

    init(abi pointer: UnsafePointer<AVWString>?) {
        guard let pointer, pointer.pointee.length > 0, let data = pointer.pointee.data else {
            self = ""
            return
        }
        self = String(decoding: UnsafeBufferPointer(start: data, count: pointer.pointee.length), as: UTF8.self)
    }

    func toABI(_ arena: PayloadArena) -> ABI {
        let bytes = arena.array(Array(utf8))
        return AVWString(data: bytes.data?.assumingMemoryBound(to: UInt8.self), length: bytes.length)
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        guard raw.length > 0 else { return "" }
        guard let data = raw.data,
              let value = String(bytes: UnsafeBufferPointer(start: data, count: raw.length), encoding: .utf8) else {
            throw BridgeError.message("invalid UTF-8 string")
        }
        return value
    }

    func withABIString<T>(_ body: (UnsafePointer<AVWString>) -> T) -> T {
        let arena = PayloadArena()
        let pointer = arena.allocate(toABI(arena))
        return withExtendedLifetime(arena) { body(pointer) }
    }
}

extension Optional: BridgePayload where Wrapped: BridgePayload {
    typealias ABI = UnsafePointer<Wrapped.ABI>?

    func toABI(_ arena: PayloadArena) -> ABI {
        map { arena.allocate($0.toABI(arena)) }
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        try raw.map { try Wrapped.fromABI($0.pointee) }
    }
}

extension Array: BridgePayload where Element: BridgePayload {
    typealias ABI = AVWArray

    func toABI(_ arena: PayloadArena) -> ABI {
        arena.array(map { $0.toABI(arena) })
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        guard raw.length > 0 else { return [] }
        guard let data = raw.data else { throw BridgeError.message("missing array data") }
        let values = UnsafeBufferPointer(start: data.assumingMemoryBound(to: Element.ABI.self), count: raw.length)
        return try values.map(Element.fromABI)
    }
}

extension Bool: BridgePayload {
    typealias ABI = UInt8

    func toABI(_ arena: PayloadArena) -> ABI {
        self ? 1 : 0
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        guard raw <= 1 else { throw BridgeError.message("invalid boolean") }
        return raw != 0
    }
}

indirect enum PropertyValue: BridgePayload {
    case null
    case boolean(Bool)
    case integer(Int64)
    case unsigned(UInt64)
    case float(Double)
    case string(String)
    case data([UInt8])
    case array([PropertyValue])
    case dictionary([String: PropertyValue])

    typealias ABI = AVWPropertyValue

    func toABI(_ arena: PayloadArena) -> ABI {
        var raw = AVWPropertyValue()
        switch self {
        case .null:
            break
        case .boolean(let value):
            raw.kind = 1
            raw.boolean = value ? 1 : 0
        case .integer(let value):
            raw.kind = 2
            raw.signed_value = value
        case .unsigned(let value):
            raw.kind = 3
            raw.unsigned_value = value
        case .float(let value):
            raw.kind = 4
            raw.number = value
        case .string(let value):
            raw.kind = 5
            raw.string = value.toABI(arena)
        case .data(let value):
            raw.kind = 6
            raw.bytes = value.toABI(arena)
        case .array(let value):
            raw.kind = 7
            raw.values = value.toABI(arena)
        case .dictionary(let value):
            raw.kind = 8
            raw.entries = arena.array(value.keys.sorted().map { key in
                AVWPropertyEntry(key: key.toABI(arena), value: value[key]!.toABI(arena))
            })
        }
        return raw
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        switch raw.kind {
        case 0: return .null
        case 1: return .boolean(try Bool.fromABI(raw.boolean))
        case 2: return .integer(raw.signed_value)
        case 3: return .unsigned(raw.unsigned_value)
        case 4: return .float(try Double.fromABI(raw.number))
        case 5: return .string(try String.fromABI(raw.string))
        case 6: return .data(try [UInt8].fromABI(raw.bytes))
        case 7: return .array(try [PropertyValue].fromABI(raw.values))
        case 8:
            guard raw.entries.length > 0 else { return .dictionary([:]) }
            guard let data = raw.entries.data else { throw BridgeError.message("missing dictionary entries") }
            let entries = UnsafeBufferPointer(
                start: data.assumingMemoryBound(to: AVWPropertyEntry.self),
                count: raw.entries.length
            )
            var result: [String: PropertyValue] = [:]
            for entry in entries {
                result[try String.fromABI(entry.key)] = try PropertyValue.fromABI(entry.value)
            }
            return .dictionary(result)
        default:
            throw BridgeError.message("invalid property value")
        }
    }

    var foundationValue: Any {
        switch self {
        case .null: return NSNull()
        case .boolean(let value): return NSNumber(value: value)
        case .integer(let value): return NSNumber(value: value)
        case .unsigned(let value): return NSNumber(value: value)
        case .float(let value): return NSNumber(value: value)
        case .string(let value): return value
        case .data(let value): return Data(value)
        case .array(let value): return value.map(\.foundationValue)
        case .dictionary(let value): return value.mapValues(\.foundationValue)
        }
    }
}

func propertyValue(from object: Any?) -> PropertyValue? {
    guard let object else { return nil }
    switch object as AnyObject {
    case is NSNull:
        return .null
    case let string as String:
        return .string(string)
    case let number as NSNumber:
        if CFGetTypeID(number) == CFBooleanGetTypeID() {
            return .boolean(number.boolValue)
        }
        let type = String(cString: number.objCType)
        if type == "f" || type == "d" {
            return .float(number.doubleValue)
        }
        if ["C", "S", "I", "L", "Q"].contains(type) {
            return .unsigned(number.uint64Value)
        }
        return .integer(number.int64Value)
    case let bytes as Data:
        return .data(Array(bytes))
    case let array as [Any]:
        let values = array.compactMap { propertyValue(from: $0) }
        return values.count == array.count ? .array(values) : nil
    case let dictionary as [String: Any]:
        var values: [String: PropertyValue] = [:]
        for (key, value) in dictionary {
            guard let value = propertyValue(from: value) else { return nil }
            values[key] = value
        }
        return .dictionary(values)
    default:
        return nil
    }
}

func propertyObject(_ pointer: UnsafePointer<AVWPropertyValue>?) throws -> Any? {
    guard let pointer else { return nil }
    return try PropertyValue.fromABI(pointer.pointee).foundationValue
}

extension TimePayload: BridgePayload {
    typealias ABI = AVWTime

    func toABI(_ arena: PayloadArena) -> ABI {
        let tag: Int32
        switch kind {
        case "numeric": tag = 0
        case "indefinite": tag = 2
        case "positive_infinity": tag = 3
        case "negative_infinity": tag = 4
        default: tag = 1
        }
        return AVWTime(kind: tag, value: value ?? 0, timescale: timescale ?? 0)
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        let kind: String
        switch raw.kind {
        case 0: kind = "numeric"
        case 1: kind = "invalid"
        case 2: kind = "indefinite"
        case 3: kind = "positive_infinity"
        case 4: kind = "negative_infinity"
        default: throw BridgeError.message("invalid time kind")
        }
        return Self(
            kind: kind,
            value: raw.kind == 0 ? raw.value : nil,
            timescale: raw.kind == 0 ? raw.timescale : nil
        )
    }
}

extension MetadataValuePayload: BridgePayload {
    typealias ABI = AVWMetadataValue

    func toABI(_ arena: PayloadArena) -> ABI {
        var raw = AVWMetadataValue()
        switch kind {
        case "string":
            raw.kind = 0
            raw.string_value = (stringValue ?? "").toABI(arena)
        case "integer":
            raw.kind = 1
            raw.integer_value = integerValue ?? 0
        case "float":
            raw.kind = 2
            raw.float_value = floatValue ?? 0
        case "boolean":
            raw.kind = 3
            raw.boolean_value = (booleanValue ?? false) ? 1 : 0
        case "data":
            raw.kind = 4
            raw.data_value = (dataValue ?? []).toABI(arena)
        default:
            preconditionFailure("invalid metadata value kind")
        }
        return raw
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        switch raw.kind {
        case 0: return Self(kind: "string", stringValue: try String.fromABI(raw.string_value))
        case 1: return Self(kind: "integer", integerValue: raw.integer_value)
        case 2: return Self(kind: "float", floatValue: try Double.fromABI(raw.float_value))
        case 3: return Self(kind: "boolean", booleanValue: try Bool.fromABI(raw.boolean_value))
        case 4: return Self(kind: "data", dataValue: try [UInt8].fromABI(raw.data_value))
        default: throw BridgeError.message("invalid metadata value kind")
        }
    }
}


extension UInt8: BridgePayload {
    typealias ABI = Self
    func toABI(_ arena: PayloadArena) -> ABI {
        self
    }
    static func fromABI(_ raw: ABI) throws -> Self {
        raw
    }
}

extension Int32: BridgePayload {
    typealias ABI = Self
    func toABI(_ arena: PayloadArena) -> ABI {
        self
    }
    static func fromABI(_ raw: ABI) throws -> Self {
        raw
    }
}

extension UInt32: BridgePayload {
    typealias ABI = Self
    func toABI(_ arena: PayloadArena) -> ABI {
        self
    }
    static func fromABI(_ raw: ABI) throws -> Self {
        raw
    }
}

extension Int64: BridgePayload {
    typealias ABI = Self
    func toABI(_ arena: PayloadArena) -> ABI {
        self
    }
    static func fromABI(_ raw: ABI) throws -> Self {
        raw
    }
}

extension UInt64: BridgePayload {
    typealias ABI = Self
    func toABI(_ arena: PayloadArena) -> ABI {
        self
    }
    static func fromABI(_ raw: ABI) throws -> Self {
        raw
    }
}

extension Int: BridgePayload {
    typealias ABI = Self
    func toABI(_ arena: PayloadArena) -> ABI {
        self
    }
    static func fromABI(_ raw: ABI) throws -> Self {
        raw
    }
}

extension Float: BridgePayload {
    typealias ABI = Self
    func toABI(_ arena: PayloadArena) -> ABI {
        self
    }
    static func fromABI(_ raw: ABI) throws -> Self {
        guard raw.isFinite else { throw BridgeError.message("invalid floating-point value") }
        return raw
    }
}

extension Double: BridgePayload {
    typealias ABI = Self
    func toABI(_ arena: PayloadArena) -> ABI {
        self
    }
    static func fromABI(_ raw: ABI) throws -> Self {
        guard raw.isFinite else { throw BridgeError.message("invalid floating-point value") }
        return raw
    }
}
