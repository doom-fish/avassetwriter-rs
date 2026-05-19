import AVFoundation
import CoreGraphics
import CoreMedia
import Dispatch
import Foundation

enum CaptionUnitsTypeRaw: String, Codable {
    case unspecified
    case cells
    case percent
}

struct CaptionDimensionPayload: Codable {
    let value: Double
    let units: CaptionUnitsTypeRaw
}

struct CaptionPointPayload: Codable {
    let x: CaptionDimensionPayload
    let y: CaptionDimensionPayload
}

struct CaptionSizePayload: Codable {
    let width: CaptionDimensionPayload
    let height: CaptionDimensionPayload
}

enum CaptionRegionDisplayAlignmentRaw: String, Codable {
    case before
    case center
    case after
}

enum CaptionRegionWritingModeRaw: String, Codable {
    case left_to_right_and_top_to_bottom
    case top_to_bottom_and_right_to_left
}

enum CaptionRegionScrollRaw: String, Codable {
    case none
    case roll_up
}

struct CaptionRegionPayload: Codable {
    let identifier: String?
    let origin: CaptionPointPayload
    let size: CaptionSizePayload
    let scroll: CaptionRegionScrollRaw
    let displayAlignment: CaptionRegionDisplayAlignmentRaw
    let writingMode: CaptionRegionWritingModeRaw
}

enum CaptionTextAlignmentRaw: String, Codable {
    case start
    case end
    case center
    case left
    case right
}

enum CaptionAnimationRaw: String, Codable {
    case none
    case character_reveal
}

enum CaptionRubyPositionRaw: String, Codable {
    case before
    case after
}

enum CaptionRubyAlignmentRaw: String, Codable {
    case start
    case center
    case distribute_space_between
    case distribute_space_around
}

struct CaptionRubyPayload: Codable {
    let text: String
    let position: CaptionRubyPositionRaw
    let alignment: CaptionRubyAlignmentRaw
}

struct CaptionRubySpanPayload: Codable {
    let start: Int
    let length: Int
    let ruby: CaptionRubyPayload
}

struct CaptionBoundsPayload: Codable {
    let x: Double
    let y: Double
    let width: Double
    let height: Double
}

struct CaptionRendererInfoPayload: Codable {
    let captions: [CaptionPayload]
    let bounds: CaptionBoundsPayload
}

struct CaptionRendererScenePayload: Codable {
    let timeRange: TimeRangePayload
    let hasActiveCaptions: Bool
    let needsPeriodicRefresh: Bool
}

struct CaptionConversionSettingsPayload: Codable {
    let mediaSubtype: String
    let timeCodeFrameDuration: TimePayload?
    let useDropFrameTimeCode: Bool
}

struct CaptionConversionAdjustmentPayload: Codable {
    let adjustmentType: String
    let startTimeOffset: TimePayload?
    let durationOffset: TimePayload?
}

struct CaptionConversionWarningPayload: Codable {
    let warningType: String
    let rangeOfCaptionsStart: Int
    let rangeOfCaptionsLength: Int
    let adjustment: CaptionConversionAdjustmentPayload?
}

struct CaptionConversionValidatorInfoPayload: Codable {
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

private func captionRegion(from payload: CaptionRegionPayload) -> AVCaptionRegion {
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

func caption(from payload: CaptionPayload) -> AVCaption {
    let caption = AVMutableCaption(payload.text, timeRange: cmTimeRange(from: payload.timeRange))
    if let region = payload.region {
        caption.region = captionRegion(from: region)
    }
    if let textAlignment = payload.textAlignment {
        caption.textAlignment = avCaptionTextAlignment(from: textAlignment)
    }
    if let animation = payload.animation {
        caption.animation = avCaptionAnimation(from: animation)
    }
    for rubySpan in payload.rubySpans ?? [] {
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
        rubySpans: nil
    )
}

func captionGroup(from payload: CaptionGroupPayload) -> AVCaptionGroup {
    AVCaptionGroup(captions: payload.captions.map(caption(from:)), timeRange: cmTimeRange(from: payload.timeRange))
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

@_cdecl("av_caption_region_predefined_json")
public func av_caption_region_predefined_json(
    _ kindPtr: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutablePointer<CChar>? {
    guard let region = predefinedCaptionRegion(named: String(cString: kindPtr)) else {
        outErrorMessage?.pointee = ffiString("unknown predefined caption region")
        return nil
    }
    do {
        return ffiString(try encodeJson(encodeCaptionRegion(region)))
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_caption_grouper_create")
public func av_caption_grouper_create() -> UnsafeMutableRawPointer? {
    Unmanaged.passRetained(AVCaptionGrouper()).toOpaque()
}

@_cdecl("av_caption_grouper_add_caption_json")
public func av_caption_grouper_add_caption_json(
    _ grouperPtr: UnsafeMutableRawPointer,
    _ captionJson: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let grouper = Unmanaged<AVCaptionGrouper>.fromOpaque(grouperPtr).takeUnretainedValue()
    do {
        let payload = try decodeJson(captionJson, as: CaptionPayload.self)
        grouper.add(caption(from: payload))
        return AVW_OK
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return AVW_INVALID_ARGUMENT
    }
}

@_cdecl("av_caption_grouper_flush_groups_json")
public func av_caption_grouper_flush_groups_json(
    _ grouperPtr: UnsafeMutableRawPointer,
    _ upToTimeValue: Int64,
    _ upToTimeScale: Int32,
    _ upToTimeKind: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutablePointer<CChar>? {
    let grouper = Unmanaged<AVCaptionGrouper>.fromOpaque(grouperPtr).takeUnretainedValue()
    do {
        let groups = grouper.flushAddedCaptions(upTo: cmTime(value: upToTimeValue, timescale: upToTimeScale, kind: upToTimeKind))
        return ffiString(try encodeJson(groups.map(encodeCaptionGroup)))
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
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
    _ settingsJson: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    do {
        let settings = try decodeJson(settingsJson, as: CaptionConversionSettingsPayload.self)
        return Unmanaged.passRetained(
            AVCaptionFormatConformer(conversionSettings: captionConversionSettings(from: settings))
        ).toOpaque()
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
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

@_cdecl("av_caption_format_conformer_conformed_caption_json")
public func av_caption_format_conformer_conformed_caption_json(
    _ conformerPtr: UnsafeMutableRawPointer,
    _ captionJson: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutablePointer<CChar>? {
    let conformer = Unmanaged<AVCaptionFormatConformer>.fromOpaque(conformerPtr).takeUnretainedValue()
    do {
        let payload = try decodeJson(captionJson, as: CaptionPayload.self)
        let conformed = try conformer.conformedCaption(for: caption(from: payload))
        return ffiString(try encodeJson(encodeCaption(conformed)))
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
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

@_cdecl("av_caption_renderer_info_json")
public func av_caption_renderer_info_json(
    _ rendererPtr: UnsafeMutableRawPointer
) -> UnsafeMutablePointer<CChar>? {
    let renderer = Unmanaged<AVCaptionRenderer>.fromOpaque(rendererPtr).takeUnretainedValue()
    do {
        return ffiString(try encodeJson(encodeCaptionRendererInfoPayload(from: renderer)))
    } catch {
        return nil
    }
}

@_cdecl("av_caption_renderer_set_captions_json")
public func av_caption_renderer_set_captions_json(
    _ rendererPtr: UnsafeMutableRawPointer,
    _ captionsJson: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let renderer = Unmanaged<AVCaptionRenderer>.fromOpaque(rendererPtr).takeUnretainedValue()
    do {
        let captions = try decodeJson(captionsJson, as: [CaptionPayload].self).map(caption(from:))
        renderer.captions = captions
        return AVW_OK
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
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

@_cdecl("av_caption_renderer_scene_changes_json")
public func av_caption_renderer_scene_changes_json(
    _ rendererPtr: UnsafeMutableRawPointer,
    _ startValue: Int64,
    _ startScale: Int32,
    _ startKind: Int32,
    _ durationValue: Int64,
    _ durationScale: Int32,
    _ durationKind: Int32,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutablePointer<CChar>? {
    let renderer = Unmanaged<AVCaptionRenderer>.fromOpaque(rendererPtr).takeUnretainedValue()
    do {
        let scenes = renderer.captionSceneChanges(in: CMTimeRange(
            start: cmTime(value: startValue, timescale: startScale, kind: startKind),
            duration: cmTime(value: durationValue, timescale: durationScale, kind: durationKind)
        ))
        return ffiString(try encodeJson(scenes.map(encodeCaptionRendererScene)))
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
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
    _ captionsJson: UnsafePointer<CChar>,
    _ startValue: Int64,
    _ startScale: Int32,
    _ startKind: Int32,
    _ durationValue: Int64,
    _ durationScale: Int32,
    _ durationKind: Int32,
    _ settingsJson: UnsafePointer<CChar>,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> UnsafeMutableRawPointer? {
    do {
        let captions = try decodeJson(captionsJson, as: [CaptionPayload].self).map(caption(from:))
        let settings = try decodeJson(settingsJson, as: CaptionConversionSettingsPayload.self)
        let validator = AVCaptionConversionValidator(
            captions: captions,
            timeRange: CMTimeRange(
                start: cmTime(value: startValue, timescale: startScale, kind: startKind),
                duration: cmTime(value: durationValue, timescale: durationScale, kind: durationKind)
            ),
            conversionSettings: captionConversionSettings(from: settings)
        )
        return Unmanaged.passRetained(validator).toOpaque()
    } catch {
        outErrorMessage?.pointee = ffiString(error.localizedDescription)
        return nil
    }
}

@_cdecl("av_caption_conversion_validator_info_json")
public func av_caption_conversion_validator_info_json(
    _ validatorPtr: UnsafeMutableRawPointer
) -> UnsafeMutablePointer<CChar>? {
    let validator = Unmanaged<AVCaptionConversionValidator>.fromOpaque(validatorPtr).takeUnretainedValue()
    do {
        return ffiString(try encodeJson(encodeCaptionConversionValidatorInfoPayload(from: validator)))
    } catch {
        return nil
    }
}

@_cdecl("av_caption_conversion_validator_validate")
public func av_caption_conversion_validator_validate(
    _ validatorPtr: UnsafeMutableRawPointer,
    _ outErrorMessage: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    let validator = Unmanaged<AVCaptionConversionValidator>.fromOpaque(validatorPtr).takeUnretainedValue()
    guard validator.status == .unknown else {
        outErrorMessage?.pointee = ffiString("caption conversion validator has already been started")
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
