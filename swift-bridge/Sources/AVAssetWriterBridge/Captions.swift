import AVAssetWriterObjCBridge
import AVFoundation
import CoreGraphics
import CoreMedia
import Dispatch
import Foundation

enum CaptionUnitsTypeRaw: String {
    case unspecified
    case cells
    case percent
}

struct CaptionDimensionPayload {
    let value: Double
    let units: CaptionUnitsTypeRaw
}

struct CaptionPointPayload {
    let x: CaptionDimensionPayload
    let y: CaptionDimensionPayload
}

struct CaptionSizePayload {
    let width: CaptionDimensionPayload
    let height: CaptionDimensionPayload
}

enum CaptionRegionDisplayAlignmentRaw: String {
    case before
    case center
    case after
}

enum CaptionRegionWritingModeRaw: String {
    case left_to_right_and_top_to_bottom
    case top_to_bottom_and_right_to_left
}

enum CaptionRegionScrollRaw: String {
    case none
    case roll_up
}

struct CaptionRegionPayload {
    let identifier: String?
    let origin: CaptionPointPayload
    let size: CaptionSizePayload
    let scroll: CaptionRegionScrollRaw
    let displayAlignment: CaptionRegionDisplayAlignmentRaw
    let writingMode: CaptionRegionWritingModeRaw
}

enum CaptionTextAlignmentRaw: String {
    case start
    case end
    case center
    case left
    case right
}

enum CaptionAnimationRaw: String {
    case none
    case character_reveal
}

enum CaptionRubyPositionRaw: String {
    case before
    case after
}

enum CaptionRubyAlignmentRaw: String {
    case start
    case center
    case distribute_space_between
    case distribute_space_around
}

struct CaptionRubyPayload {
    let text: String
    let position: CaptionRubyPositionRaw
    let alignment: CaptionRubyAlignmentRaw
}

struct CaptionRubySpanPayload {
    let start: Int
    let length: Int
    let ruby: CaptionRubyPayload
}

struct CaptionBoundsPayload {
    let x: Double
    let y: Double
    let width: Double
    let height: Double
}

struct CaptionRendererInfoPayload {
    let captions: [CaptionPayload]
    let bounds: CaptionBoundsPayload
}

struct CaptionRendererScenePayload {
    let timeRange: TimeRangePayload
    let hasActiveCaptions: Bool
    let needsPeriodicRefresh: Bool
}

struct CaptionConversionSettingsPayload {
    let mediaSubtype: String
    let timeCodeFrameDuration: TimePayload?
    let useDropFrameTimeCode: Bool
}

struct CaptionConversionAdjustmentPayload {
    let adjustmentType: String
    let startTimeOffset: TimePayload?
    let durationOffset: TimePayload?
}

struct CaptionConversionWarningPayload {
    let warningType: String
    let rangeOfCaptionsStart: Int
    let rangeOfCaptionsLength: Int
    let adjustment: CaptionConversionAdjustmentPayload?
}

struct CaptionConversionValidatorInfoPayload {
    let status: Int
    let captions: [CaptionPayload]
    let timeRange: TimeRangePayload
    let warnings: [CaptionConversionWarningPayload]
}

private func avCaptionUnitsType(from raw: CaptionUnitsTypeRaw) -> AVCaptionUnitsType {
    switch raw {
    case .unspecified:
        return .unspecified
    case .cells:
        return .cells
    case .percent:
        return .percent
    }
}

private func encodeCaptionUnitsType(_ units: AVCaptionUnitsType) -> CaptionUnitsTypeRaw {
    switch units {
    case .cells:
        return .cells
    case .percent:
        return .percent
    default:
        return .unspecified
    }
}

private func avCaptionDimension(from payload: CaptionDimensionPayload) -> AVCaptionDimension {
    AVCaptionDimension(value: payload.value, units: avCaptionUnitsType(from: payload.units))
}

private func encodeCaptionDimension(_ dimension: AVCaptionDimension) -> CaptionDimensionPayload {
    CaptionDimensionPayload(value: dimension.value, units: encodeCaptionUnitsType(dimension.units))
}

private func avCaptionPoint(from payload: CaptionPointPayload) -> AVCaptionPoint {
    AVCaptionPoint(x: avCaptionDimension(from: payload.x), y: avCaptionDimension(from: payload.y))
}

private func encodeCaptionPoint(_ point: AVCaptionPoint) -> CaptionPointPayload {
    CaptionPointPayload(x: encodeCaptionDimension(point.x), y: encodeCaptionDimension(point.y))
}

private func avCaptionSize(from payload: CaptionSizePayload) -> AVCaptionSize {
    AVCaptionSize(width: avCaptionDimension(from: payload.width), height: avCaptionDimension(from: payload.height))
}

private func encodeCaptionSize(_ size: AVCaptionSize) -> CaptionSizePayload {
    CaptionSizePayload(width: encodeCaptionDimension(size.width), height: encodeCaptionDimension(size.height))
}

private func avCaptionRegionDisplayAlignment(
    from raw: CaptionRegionDisplayAlignmentRaw
) -> AVCaptionRegion.DisplayAlignment {
    switch raw {
    case .before:
        return .before
    case .center:
        return .center
    case .after:
        return .after
    }
}

private func encodeCaptionRegionDisplayAlignment(
    _ alignment: AVCaptionRegion.DisplayAlignment
) -> CaptionRegionDisplayAlignmentRaw {
    switch alignment {
    case .center:
        return .center
    case .after:
        return .after
    default:
        return .before
    }
}

private func avCaptionRegionWritingMode(from raw: CaptionRegionWritingModeRaw) -> AVCaptionRegion.WritingMode {
    switch raw {
    case .left_to_right_and_top_to_bottom:
        return .leftToRightAndTopToBottom
    case .top_to_bottom_and_right_to_left:
        return .topToBottomAndRightToLeft
    }
}

private func encodeCaptionRegionWritingMode(
    _ writingMode: AVCaptionRegion.WritingMode
) -> CaptionRegionWritingModeRaw {
    switch writingMode {
    case .topToBottomAndRightToLeft:
        return .top_to_bottom_and_right_to_left
    default:
        return .left_to_right_and_top_to_bottom
    }
}

private func avCaptionRegionScroll(from raw: CaptionRegionScrollRaw) -> AVCaptionRegion.Scroll {
    switch raw {
    case .none:
        return .none
    case .roll_up:
        return .rollUp
    }
}

private func encodeCaptionRegionScroll(_ scroll: AVCaptionRegion.Scroll) -> CaptionRegionScrollRaw {
    switch scroll {
    case .rollUp:
        return .roll_up
    default:
        return .none
    }
}

private func captionRegion(from payload: CaptionRegionPayload) throws -> AVCaptionRegion {
    guard payload.size.width.units == payload.size.height.units else {
        throw BridgeError.message("caption region width and height must use the same units")
    }
    let region = payload.identifier.map(AVMutableCaptionRegion.init(identifier:)) ?? AVMutableCaptionRegion()
    region.origin = avCaptionPoint(from: payload.origin)
    region.size = avCaptionSize(from: payload.size)
    region.scroll = avCaptionRegionScroll(from: payload.scroll)
    region.displayAlignment = avCaptionRegionDisplayAlignment(from: payload.displayAlignment)
    region.writingMode = avCaptionRegionWritingMode(from: payload.writingMode)
    return region
}

private func encodeCaptionRegion(_ region: AVCaptionRegion) -> CaptionRegionPayload {
    CaptionRegionPayload(
        identifier: region.identifier,
        origin: encodeCaptionPoint(region.origin),
        size: encodeCaptionSize(region.size),
        scroll: encodeCaptionRegionScroll(region.scroll),
        displayAlignment: encodeCaptionRegionDisplayAlignment(region.displayAlignment),
        writingMode: encodeCaptionRegionWritingMode(region.writingMode)
    )
}

private func avCaptionTextAlignment(from raw: CaptionTextAlignmentRaw) -> AVCaption.TextAlignment {
    switch raw {
    case .start:
        return .start
    case .end:
        return .end
    case .center:
        return .center
    case .left:
        return .left
    case .right:
        return .right
    }
}

private func encodeCaptionTextAlignment(_ alignment: AVCaption.TextAlignment) -> CaptionTextAlignmentRaw {
    switch alignment {
    case .end:
        return .end
    case .center:
        return .center
    case .left:
        return .left
    case .right:
        return .right
    default:
        return .start
    }
}

private func avCaptionAnimation(from raw: CaptionAnimationRaw) -> AVCaption.Animation {
    switch raw {
    case .none:
        return .none
    case .character_reveal:
        return .characterReveal
    }
}

private func encodeCaptionAnimation(_ animation: AVCaption.Animation) -> CaptionAnimationRaw {
    switch animation {
    case .characterReveal:
        return .character_reveal
    default:
        return .none
    }
}

private func avCaptionRubyPosition(from raw: CaptionRubyPositionRaw) -> AVCaption.Ruby.Position {
    switch raw {
    case .before:
        return .before
    case .after:
        return .after
    }
}

private func encodeCaptionRubyPosition(_ position: AVCaption.Ruby.Position) -> CaptionRubyPositionRaw {
    switch position {
    case .after:
        return .after
    default:
        return .before
    }
}

private func avCaptionRubyAlignment(from raw: CaptionRubyAlignmentRaw) -> AVCaption.Ruby.Alignment {
    switch raw {
    case .start:
        return .start
    case .center:
        return .center
    case .distribute_space_between:
        return .distributeSpaceBetween
    case .distribute_space_around:
        return .distributeSpaceAround
    }
}

private func encodeCaptionRubyAlignment(_ alignment: AVCaption.Ruby.Alignment) -> CaptionRubyAlignmentRaw {
    switch alignment {
    case .start:
        return .start
    case .center:
        return .center
    case .distributeSpaceAround:
        return .distribute_space_around
    default:
        return .distribute_space_between
    }
}

private func captionRuby(from payload: CaptionRubyPayload) -> AVCaption.Ruby {
    AVCaption.Ruby(
        text: payload.text,
        position: avCaptionRubyPosition(from: payload.position),
        alignment: avCaptionRubyAlignment(from: payload.alignment)
    )
}

private func encodeCaptionRuby(_ ruby: AVCaption.Ruby) -> CaptionRubyPayload {
    CaptionRubyPayload(
        text: ruby.text,
        position: encodeCaptionRubyPosition(ruby.position),
        alignment: encodeCaptionRubyAlignment(ruby.alignment)
    )
}

private func encodeCaptionBounds(_ bounds: CGRect) -> CaptionBoundsPayload {
    CaptionBoundsPayload(
        x: Double(bounds.origin.x),
        y: Double(bounds.origin.y),
        width: Double(bounds.size.width),
        height: Double(bounds.size.height)
    )
}

private func captionBounds(from payload: CaptionBoundsPayload) -> CGRect {
    CGRect(x: payload.x, y: payload.y, width: payload.width, height: payload.height)
}

private func captionConversionSettings(
    from payload: CaptionConversionSettingsPayload
) -> [AVCaptionSettingsKey: Any] {
    var settings: [AVCaptionSettingsKey: Any] = [
        .mediaType: AVMediaType.closedCaption,
        .mediaSubType: payload.mediaSubtype,
        .useDropFrameTimeCode: payload.useDropFrameTimeCode,
    ]
    if let timeCodeFrameDuration = payload.timeCodeFrameDuration {
        settings[.timeCodeFrameDuration] = NSValue(time: cmTime(from: timeCodeFrameDuration))
    }
    return settings
}

private func encodeCaptionConversionAdjustment(
    _ adjustment: AVCaptionConversionAdjustment
) -> CaptionConversionAdjustmentPayload {
    if let adjustment = adjustment as? AVCaptionConversionTimeRangeAdjustment {
        return CaptionConversionAdjustmentPayload(
            adjustmentType: adjustment.adjustmentType.rawValue,
            startTimeOffset: encodeTime(adjustment.startTimeOffset),
            durationOffset: encodeTime(adjustment.durationOffset)
        )
    }
    return CaptionConversionAdjustmentPayload(
        adjustmentType: adjustment.adjustmentType.rawValue,
        startTimeOffset: nil,
        durationOffset: nil
    )
}

private func encodeCaptionConversionWarning(
    _ warning: AVCaptionConversionWarning
) -> CaptionConversionWarningPayload {
    CaptionConversionWarningPayload(
        warningType: warning.warningType.rawValue,
        rangeOfCaptionsStart: warning.rangeOfCaptions.location,
        rangeOfCaptionsLength: warning.rangeOfCaptions.length,
        adjustment: warning.adjustment.map(encodeCaptionConversionAdjustment)
    )
}

private func encodeCaptionRendererScene(_ scene: AVCaptionRenderer.Scene) -> CaptionRendererScenePayload {
    CaptionRendererScenePayload(
        timeRange: encodeTimeRange(scene.timeRange),
        hasActiveCaptions: scene.hasActiveCaptions,
        needsPeriodicRefresh: scene.needsPeriodicRefresh
    )
}

private func encodeCaptionRendererInfoPayload(from renderer: AVCaptionRenderer) -> CaptionRendererInfoPayload {
    CaptionRendererInfoPayload(
        captions: renderer.captions.map(encodeCaption),
        bounds: encodeCaptionBounds(renderer.bounds)
    )
}

private func encodeCaptionConversionValidatorInfoPayload(
    from validator: AVCaptionConversionValidator
) -> CaptionConversionValidatorInfoPayload {
    CaptionConversionValidatorInfoPayload(
        status: validator.status.rawValue,
        captions: validator.captions.map(encodeCaption),
        timeRange: encodeTimeRange(validator.timeRange),
        warnings: validator.warnings.map(encodeCaptionConversionWarning)
    )
}

func caption(from payload: CaptionPayload) throws -> AVCaption {
    let caption = AVMutableCaption(payload.text, timeRange: cmTimeRange(from: payload.timeRange))
    if let region = payload.region {
        var message: NSString?
        guard AVWXSetCaptionRegion(caption, try captionRegion(from: region), &message) else {
            throw BridgeError.message((message as String?) ?? "caption region was rejected")
        }
    }
    if let textAlignment = payload.textAlignment {
        caption.textAlignment = avCaptionTextAlignment(from: textAlignment)
    }
    if let animation = payload.animation {
        caption.animation = avCaptionAnimation(from: animation)
    }
    let textLength = (payload.text as NSString).length
    for rubySpan in payload.rubySpans {
        guard rubySpan.start >= 0, rubySpan.length >= 0, rubySpan.start <= textLength - rubySpan.length else {
            throw BridgeError.message(
                "ruby span \(rubySpan.start)+\(rubySpan.length) is outside the caption text (\(textLength) UTF-16 units)"
            )
        }
        caption.setRuby(captionRuby(from: rubySpan.ruby), in: NSRange(location: rubySpan.start, length: rubySpan.length))
    }
    return caption
}

func encodeCaption(_ caption: AVCaption) -> CaptionPayload {
    CaptionPayload(
        text: caption.text,
        timeRange: encodeTimeRange(caption.timeRange),
        region: caption.region.map(encodeCaptionRegion),
        textAlignment: encodeCaptionTextAlignment(caption.textAlignment),
        animation: encodeCaptionAnimation(caption.animation),
        rubySpans: []
    )
}

func captionGroup(from payload: CaptionGroupPayload) throws -> AVCaptionGroup {
    var message: NSString?
    guard let group = AVWXMakeCaptionGroup(
        try payload.captions.map(caption(from:)),
        cmTimeRange(from: payload.timeRange),
        &message
    ) else {
        throw BridgeError.message((message as String?) ?? "caption group was rejected")
    }
    return group
}

func encodeCaptionGroup(_ group: AVCaptionGroup) -> CaptionGroupPayload {
    CaptionGroupPayload(captions: group.captions.map(encodeCaption), timeRange: encodeTimeRange(group.timeRange))
}

private func predefinedCaptionRegion(named kind: String) -> AVCaptionRegion? {
    switch kind {
    case "apple_itt_top":
        return AVCaptionRegion.appleITTTop
    case "apple_itt_bottom":
        return AVCaptionRegion.appleITTBottom
    case "apple_itt_left":
        return AVCaptionRegion.appleITTLeft
    case "apple_itt_right":
        return AVCaptionRegion.appleITTRight
    case "sub_rip_text_bottom":
        return AVCaptionRegion.subRipTextBottom
    default:
        return nil
    }
}

@_cdecl("av_caption_region_predefined_payload")
public func av_caption_region_predefined_payload(
    _ kindPtr: UnsafePointer<AVWString>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> UnsafeMutablePointer<AVWOwnedPayload>? {
    guard let region = predefinedCaptionRegion(named: String(abi: kindPtr)) else {
        outErrorMessage?.pointee = makePayload("unknown predefined caption region")
        return nil
    }
    do {
        return makePayload(encodeCaptionRegion(region))
    } catch {
        outErrorMessage?.pointee = makePayload(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_caption_grouper_create")
public func av_caption_grouper_create() -> UnsafeMutableRawPointer? {
    Unmanaged.passRetained(AVCaptionGrouper()).toOpaque()
}

@_cdecl("av_caption_grouper_add_caption_payload")
public func av_caption_grouper_add_caption_payload(
    _ grouperPtr: UnsafeMutableRawPointer,
    _ captionPayload: UnsafePointer<AVWCaption>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let grouper = Unmanaged<AVCaptionGrouper>.fromOpaque(grouperPtr).takeUnretainedValue()
    do {
        let payload = try readPayload(captionPayload, as: CaptionPayload.self)
        try grouper.add(caption(from: payload))
        return AVW_OK
    } catch {
        outErrorMessage?.pointee = makePayload(error.localizedDescription)
        return AVW_INVALID_ARGUMENT
    }
}

@_cdecl("av_caption_grouper_flush_groups_payload")
public func av_caption_grouper_flush_groups_payload(
    _ grouperPtr: UnsafeMutableRawPointer,
    _ upToTimeValue: Int64,
    _ upToTimeScale: Int32,
    _ upToTimeKind: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> UnsafeMutablePointer<AVWOwnedPayload>? {
    let grouper = Unmanaged<AVCaptionGrouper>.fromOpaque(grouperPtr).takeUnretainedValue()
    do {
        let groups = grouper.flushAddedCaptions(upTo: cmTime(value: upToTimeValue, timescale: upToTimeScale, kind: upToTimeKind))
        return makePayload(groups.map(encodeCaptionGroup))
    } catch {
        outErrorMessage?.pointee = makePayload(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_caption_grouper_release")
public func av_caption_grouper_release(_ grouperPtr: UnsafeMutableRawPointer?) {
    guard let grouperPtr else { return }
    Unmanaged<AVCaptionGrouper>.fromOpaque(grouperPtr).release()
}

@_cdecl("av_caption_format_conformer_create")
public func av_caption_format_conformer_create(
    _ settingsPayload: UnsafePointer<AVWCaptionConversionSettings>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> UnsafeMutableRawPointer? {
    do {
        let settings = try readPayload(settingsPayload, as: CaptionConversionSettingsPayload.self)
        var message: NSString?
        guard let conformer = AVWXMakeCaptionFormatConformer(captionConversionSettings(from: settings), &message) else {
            outErrorMessage?.pointee = makePayload(
                (message as String?) ?? "caption format conformer rejected its conversion settings"
            )
            return nil
        }
        return Unmanaged.passRetained(conformer).toOpaque()
    } catch {
        outErrorMessage?.pointee = makePayload(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_caption_format_conformer_conforms_captions_to_time_range")
public func av_caption_format_conformer_conforms_captions_to_time_range(
    _ conformerPtr: UnsafeMutableRawPointer
) -> Bool {
    let conformer = Unmanaged<AVCaptionFormatConformer>.fromOpaque(conformerPtr).takeUnretainedValue()
    return conformer.conformsCaptionsToTimeRange
}

@_cdecl("av_caption_format_conformer_set_conforms_captions_to_time_range")
public func av_caption_format_conformer_set_conforms_captions_to_time_range(
    _ conformerPtr: UnsafeMutableRawPointer,
    _ conforms: Bool
) {
    let conformer = Unmanaged<AVCaptionFormatConformer>.fromOpaque(conformerPtr).takeUnretainedValue()
    conformer.conformsCaptionsToTimeRange = conforms
}

@_cdecl("av_caption_format_conformer_conformed_caption_payload")
public func av_caption_format_conformer_conformed_caption_payload(
    _ conformerPtr: UnsafeMutableRawPointer,
    _ captionPayload: UnsafePointer<AVWCaption>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> UnsafeMutablePointer<AVWOwnedPayload>? {
    let conformer = Unmanaged<AVCaptionFormatConformer>.fromOpaque(conformerPtr).takeUnretainedValue()
    do {
        let payload = try readPayload(captionPayload, as: CaptionPayload.self)
        let conformed = try conformer.conformedCaption(for: caption(from: payload))
        return makePayload(encodeCaption(conformed))
    } catch {
        outErrorMessage?.pointee = makePayload(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_caption_format_conformer_release")
public func av_caption_format_conformer_release(_ conformerPtr: UnsafeMutableRawPointer?) {
    guard let conformerPtr else { return }
    Unmanaged<AVCaptionFormatConformer>.fromOpaque(conformerPtr).release()
}

@_cdecl("av_caption_renderer_create")
public func av_caption_renderer_create() -> UnsafeMutableRawPointer? {
    Unmanaged.passRetained(AVCaptionRenderer()).toOpaque()
}

@_cdecl("av_caption_renderer_info_payload")
public func av_caption_renderer_info_payload(
    _ rendererPtr: UnsafeMutableRawPointer
) -> UnsafeMutablePointer<AVWOwnedPayload>? {
    let renderer = Unmanaged<AVCaptionRenderer>.fromOpaque(rendererPtr).takeUnretainedValue()
    do {
        return makePayload(encodeCaptionRendererInfoPayload(from: renderer))
    } catch {
        return nil
    }
}

@_cdecl("av_caption_renderer_set_captions_payload")
public func av_caption_renderer_set_captions_payload(
    _ rendererPtr: UnsafeMutableRawPointer,
    _ captionsPayload: UnsafePointer<AVWArray>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let renderer = Unmanaged<AVCaptionRenderer>.fromOpaque(rendererPtr).takeUnretainedValue()
    do {
        let captions = try readPayload(captionsPayload, as: [CaptionPayload].self).map(caption(from:))
        renderer.captions = captions
        return AVW_OK
    } catch {
        outErrorMessage?.pointee = makePayload(error.localizedDescription)
        return AVW_INVALID_ARGUMENT
    }
}

@_cdecl("av_caption_renderer_set_bounds")
public func av_caption_renderer_set_bounds(
    _ rendererPtr: UnsafeMutableRawPointer,
    _ x: Double,
    _ y: Double,
    _ width: Double,
    _ height: Double
) {
    let renderer = Unmanaged<AVCaptionRenderer>.fromOpaque(rendererPtr).takeUnretainedValue()
    renderer.bounds = CGRect(x: x, y: y, width: width, height: height)
}

@_cdecl("av_caption_renderer_scene_changes_payload")
public func av_caption_renderer_scene_changes_payload(
    _ rendererPtr: UnsafeMutableRawPointer,
    _ startValue: Int64,
    _ startScale: Int32,
    _ startKind: Int32,
    _ durationValue: Int64,
    _ durationScale: Int32,
    _ durationKind: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> UnsafeMutablePointer<AVWOwnedPayload>? {
    let renderer = Unmanaged<AVCaptionRenderer>.fromOpaque(rendererPtr).takeUnretainedValue()
    do {
        let scenes = renderer.captionSceneChanges(in: CMTimeRange(
            start: cmTime(value: startValue, timescale: startScale, kind: startKind),
            duration: cmTime(value: durationValue, timescale: durationScale, kind: durationKind)
        ))
        return makePayload(scenes.map(encodeCaptionRendererScene))
    } catch {
        outErrorMessage?.pointee = makePayload(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_caption_renderer_release")
public func av_caption_renderer_release(_ rendererPtr: UnsafeMutableRawPointer?) {
    guard let rendererPtr else { return }
    Unmanaged<AVCaptionRenderer>.fromOpaque(rendererPtr).release()
}

@_cdecl("av_caption_conversion_validator_create")
public func av_caption_conversion_validator_create(
    _ captionsPayload: UnsafePointer<AVWArray>,
    _ startValue: Int64,
    _ startScale: Int32,
    _ startKind: Int32,
    _ durationValue: Int64,
    _ durationScale: Int32,
    _ durationKind: Int32,
    _ settingsPayload: UnsafePointer<AVWCaptionConversionSettings>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> UnsafeMutableRawPointer? {
    do {
        let captions = try readPayload(captionsPayload, as: [CaptionPayload].self).map(caption(from:))
        let settings = try readPayload(settingsPayload, as: CaptionConversionSettingsPayload.self)
        var message: NSString?
        guard let validator = AVWXMakeCaptionConversionValidator(
            captions,
            CMTimeRange(
                start: cmTime(value: startValue, timescale: startScale, kind: startKind),
                duration: cmTime(value: durationValue, timescale: durationScale, kind: durationKind)
            ),
            captionConversionSettings(from: settings),
            &message
        ) else {
            outErrorMessage?.pointee = makePayload(
                (message as String?) ?? "caption conversion validator rejected its conversion settings"
            )
            return nil
        }
        return Unmanaged.passRetained(validator).toOpaque()
    } catch {
        outErrorMessage?.pointee = makePayload(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_caption_conversion_validator_info_payload")
public func av_caption_conversion_validator_info_payload(
    _ validatorPtr: UnsafeMutableRawPointer
) -> UnsafeMutablePointer<AVWOwnedPayload>? {
    let validator = Unmanaged<AVCaptionConversionValidator>.fromOpaque(validatorPtr).takeUnretainedValue()
    do {
        return makePayload(encodeCaptionConversionValidatorInfoPayload(from: validator))
    } catch {
        return nil
    }
}

@_cdecl("av_caption_conversion_validator_validate")
public func av_caption_conversion_validator_validate(
    _ validatorPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<AVWOwnedPayload>?>?
) -> Int32 {
    let validator = Unmanaged<AVCaptionConversionValidator>.fromOpaque(validatorPtr).takeUnretainedValue()
    guard validator.status == .unknown else {
        outErrorMessage?.pointee = makePayload("caption conversion validator has already been started")
        return AVW_INVALID_STATE
    }
    let semaphore = DispatchSemaphore(value: 0)
    validator.validateCaptionConversion { warning in
        if warning == nil {
            semaphore.signal()
        }
    }
    semaphore.wait()
    return AVW_OK
}

@_cdecl("av_caption_conversion_validator_stop_validating")
public func av_caption_conversion_validator_stop_validating(
    _ validatorPtr: UnsafeMutableRawPointer
) {
    let validator = Unmanaged<AVCaptionConversionValidator>.fromOpaque(validatorPtr).takeUnretainedValue()
    validator.stopValidating()
}

@_cdecl("av_caption_conversion_validator_release")
public func av_caption_conversion_validator_release(_ validatorPtr: UnsafeMutableRawPointer?) {
    guard let validatorPtr else { return }
    Unmanaged<AVCaptionConversionValidator>.fromOpaque(validatorPtr).release()
}

extension CaptionDimensionPayload: BridgePayload {
    typealias ABI = AVWCaptionDimension

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            value: value.toABI(arena),
            units: units.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            value: try .fromABI(raw.value),
            units: try .fromABI(raw.units)
        )
    }
}

extension CaptionPointPayload: BridgePayload {
    typealias ABI = AVWCaptionPoint

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            x: x.toABI(arena),
            y: y.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            x: try .fromABI(raw.x),
            y: try .fromABI(raw.y)
        )
    }
}

extension CaptionSizePayload: BridgePayload {
    typealias ABI = AVWCaptionSize

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            width: width.toABI(arena),
            height: height.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            width: try .fromABI(raw.width),
            height: try .fromABI(raw.height)
        )
    }
}

extension CaptionRegionPayload: BridgePayload {
    typealias ABI = AVWCaptionRegion

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            identifier: identifier.toABI(arena),
            origin: origin.toABI(arena),
            size: size.toABI(arena),
            scroll: scroll.toABI(arena),
            display_alignment: displayAlignment.toABI(arena),
            writing_mode: writingMode.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            identifier: try .fromABI(raw.identifier),
            origin: try .fromABI(raw.origin),
            size: try .fromABI(raw.size),
            scroll: try .fromABI(raw.scroll),
            displayAlignment: try .fromABI(raw.display_alignment),
            writingMode: try .fromABI(raw.writing_mode)
        )
    }
}

extension CaptionRubyPayload: BridgePayload {
    typealias ABI = AVWCaptionRuby

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            text: text.toABI(arena),
            position: position.toABI(arena),
            alignment: alignment.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            text: try .fromABI(raw.text),
            position: try .fromABI(raw.position),
            alignment: try .fromABI(raw.alignment)
        )
    }
}

extension CaptionRubySpanPayload: BridgePayload {
    typealias ABI = AVWCaptionRubySpan

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            start: start.toABI(arena),
            length: length.toABI(arena),
            ruby: ruby.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            start: try .fromABI(raw.start),
            length: try .fromABI(raw.length),
            ruby: try .fromABI(raw.ruby)
        )
    }
}

extension CaptionBoundsPayload: BridgePayload {
    typealias ABI = AVWCaptionBounds

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            x: x.toABI(arena),
            y: y.toABI(arena),
            width: width.toABI(arena),
            height: height.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            x: try .fromABI(raw.x),
            y: try .fromABI(raw.y),
            width: try .fromABI(raw.width),
            height: try .fromABI(raw.height)
        )
    }
}

extension CaptionRendererInfoPayload: BridgePayload {
    typealias ABI = AVWCaptionRendererInfo

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            captions: captions.toABI(arena),
            bounds: bounds.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            captions: try .fromABI(raw.captions),
            bounds: try .fromABI(raw.bounds)
        )
    }
}

extension CaptionRendererScenePayload: BridgePayload {
    typealias ABI = AVWCaptionRendererScene

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            time_range: timeRange.toABI(arena),
            has_active_captions: hasActiveCaptions.toABI(arena),
            needs_periodic_refresh: needsPeriodicRefresh.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            timeRange: try .fromABI(raw.time_range),
            hasActiveCaptions: try .fromABI(raw.has_active_captions),
            needsPeriodicRefresh: try .fromABI(raw.needs_periodic_refresh)
        )
    }
}

extension CaptionConversionSettingsPayload: BridgePayload {
    typealias ABI = AVWCaptionConversionSettings

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            media_subtype: mediaSubtype.toABI(arena),
            time_code_frame_duration: timeCodeFrameDuration.toABI(arena),
            use_drop_frame_time_code: useDropFrameTimeCode.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            mediaSubtype: try .fromABI(raw.media_subtype),
            timeCodeFrameDuration: try .fromABI(raw.time_code_frame_duration),
            useDropFrameTimeCode: try .fromABI(raw.use_drop_frame_time_code)
        )
    }
}

extension CaptionConversionAdjustmentPayload: BridgePayload {
    typealias ABI = AVWCaptionConversionAdjustment

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            adjustment_type: adjustmentType.toABI(arena),
            start_time_offset: startTimeOffset.toABI(arena),
            duration_offset: durationOffset.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            adjustmentType: try .fromABI(raw.adjustment_type),
            startTimeOffset: try .fromABI(raw.start_time_offset),
            durationOffset: try .fromABI(raw.duration_offset)
        )
    }
}

extension CaptionConversionWarningPayload: BridgePayload {
    typealias ABI = AVWCaptionConversionWarning

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            warning_type: warningType.toABI(arena),
            range_of_captions_start: rangeOfCaptionsStart.toABI(arena),
            range_of_captions_length: rangeOfCaptionsLength.toABI(arena),
            adjustment: adjustment.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            warningType: try .fromABI(raw.warning_type),
            rangeOfCaptionsStart: try .fromABI(raw.range_of_captions_start),
            rangeOfCaptionsLength: try .fromABI(raw.range_of_captions_length),
            adjustment: try .fromABI(raw.adjustment)
        )
    }
}

extension CaptionConversionValidatorInfoPayload: BridgePayload {
    typealias ABI = AVWCaptionConversionValidatorInfo

    func toABI(_ arena: PayloadArena) -> ABI {
        ABI(
            status: Int32(status),
            captions: captions.toABI(arena),
            time_range: timeRange.toABI(arena),
            warnings: warnings.toABI(arena)
        )
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        Self(
            status: Int(raw.status),
            captions: try .fromABI(raw.captions),
            timeRange: try .fromABI(raw.time_range),
            warnings: try .fromABI(raw.warnings)
        )
    }
}

extension CaptionUnitsTypeRaw: BridgePayload {
    typealias ABI = Int32

    func toABI(_ arena: PayloadArena) -> ABI {
        switch self {
        case .unspecified: return 0
        case .cells: return 1
        case .percent: return 2
        }
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        switch raw {
        case 0: return .unspecified
        case 1: return .cells
        case 2: return .percent
        default: throw BridgeError.message("invalid caption enum value")
        }
    }
}

extension CaptionRegionDisplayAlignmentRaw: BridgePayload {
    typealias ABI = Int32

    func toABI(_ arena: PayloadArena) -> ABI {
        switch self {
        case .before: return 0
        case .center: return 1
        case .after: return 2
        }
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        switch raw {
        case 0: return .before
        case 1: return .center
        case 2: return .after
        default: throw BridgeError.message("invalid caption enum value")
        }
    }
}

extension CaptionRegionWritingModeRaw: BridgePayload {
    typealias ABI = Int32

    func toABI(_ arena: PayloadArena) -> ABI {
        switch self {
        case .left_to_right_and_top_to_bottom: return 0
        case .top_to_bottom_and_right_to_left: return 1
        }
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        switch raw {
        case 0: return .left_to_right_and_top_to_bottom
        case 1: return .top_to_bottom_and_right_to_left
        default: throw BridgeError.message("invalid caption enum value")
        }
    }
}

extension CaptionRegionScrollRaw: BridgePayload {
    typealias ABI = Int32

    func toABI(_ arena: PayloadArena) -> ABI {
        switch self {
        case .none: return 0
        case .roll_up: return 1
        }
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        switch raw {
        case 0: return .none
        case 1: return .roll_up
        default: throw BridgeError.message("invalid caption enum value")
        }
    }
}

extension CaptionTextAlignmentRaw: BridgePayload {
    typealias ABI = Int32

    func toABI(_ arena: PayloadArena) -> ABI {
        switch self {
        case .start: return 0
        case .end: return 1
        case .center: return 2
        case .left: return 3
        case .right: return 4
        }
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        switch raw {
        case 0: return .start
        case 1: return .end
        case 2: return .center
        case 3: return .left
        case 4: return .right
        default: throw BridgeError.message("invalid caption enum value")
        }
    }
}

extension CaptionAnimationRaw: BridgePayload {
    typealias ABI = Int32

    func toABI(_ arena: PayloadArena) -> ABI {
        switch self {
        case .none: return 0
        case .character_reveal: return 1
        }
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        switch raw {
        case 0: return .none
        case 1: return .character_reveal
        default: throw BridgeError.message("invalid caption enum value")
        }
    }
}

extension CaptionRubyPositionRaw: BridgePayload {
    typealias ABI = Int32

    func toABI(_ arena: PayloadArena) -> ABI {
        switch self {
        case .before: return 0
        case .after: return 1
        }
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        switch raw {
        case 0: return .before
        case 1: return .after
        default: throw BridgeError.message("invalid caption enum value")
        }
    }
}

extension CaptionRubyAlignmentRaw: BridgePayload {
    typealias ABI = Int32

    func toABI(_ arena: PayloadArena) -> ABI {
        switch self {
        case .start: return 0
        case .center: return 1
        case .distribute_space_between: return 2
        case .distribute_space_around: return 3
        }
    }

    static func fromABI(_ raw: ABI) throws -> Self {
        switch raw {
        case 0: return .start
        case 1: return .center
        case 2: return .distribute_space_between
        case 3: return .distribute_space_around
        default: throw BridgeError.message("invalid caption enum value")
        }
    }
}
