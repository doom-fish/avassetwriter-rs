//! API-surface coverage harness for `avassetwriter`.
//!
//! `AVAssetWriter` is an Obj-C class — there's no `extern "C"` surface to
//! diff. Instead we extract the public method/property names from
//! `AVAssetWriter.h` and `AVAssetWriterInput.h`, then check which of those
//! the Swift bridge in `swift-bridge/Sources/AVAssetWriterBridge/` actually
//! references (calls or assigns).
//!
//! This is necessarily heuristic — Swift doesn't strip away unused method
//! references at static-analysis time so we treat any textual occurrence
//! as "wrapped". False positives from e.g. variable names that match a
//! method name are filtered out by the `intentionally_omitted()` allowlist
//! when they cause noise.

#![allow(
    clippy::cast_precision_loss,
    clippy::iter_on_single_items,
    clippy::missing_const_for_fn
)]

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Command;

fn sdk_root() -> PathBuf {
    let out = Command::new("xcrun")
        .args(["--sdk", "macosx", "--show-sdk-path"])
        .output()
        .expect("xcrun must be available");
    assert!(out.status.success(), "xcrun --show-sdk-path failed");
    PathBuf::from(String::from_utf8(out.stdout).unwrap().trim().to_string())
}

fn read_file(path: &PathBuf) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("can't read {}: {e}", path.display()))
}

/// Extract Obj-C method "selectors" (the part before the first colon) and
/// `@property` names from a header. Returns a flat set of identifiers.
fn extract_objc_surface(source: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    // Methods: "- (RetType)methodName" or "- (RetType)methodName:withArg:"
    // We grab just the first "name" segment (before the first colon).
    let method_re =
        regex_lite::Regex::new(r"^\s*[-+]\s*\([^)]*\)\s*([a-zA-Z_][A-Za-z0-9_]*)").unwrap();
    // Properties: "@property (...) ... NAME ;" — name is the last identifier
    // before the semicolon or before any inline annotation.
    let prop_re = regex_lite::Regex::new(
        r"@property\s*(?:\([^)]*\))?\s*[^;]*?\b([a-zA-Z_][A-Za-z0-9_]*)\s*(?:;|API_)",
    )
    .unwrap();
    for line in source.lines() {
        if let Some(c) = method_re.captures(line) {
            let name = &c[1];
            if !name.starts_with("__") {
                out.insert(name.to_string());
            }
        }
        if let Some(c) = prop_re.captures(line) {
            let name = &c[1];
            if !name.starts_with("__") {
                out.insert(name.to_string());
            }
        }
    }
    out
}

fn read_our_swift_bridge() -> String {
    let bridge_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("swift-bridge/Sources/AVAssetWriterBridge");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&bridge_dir)
        .unwrap_or_else(|e| panic!("can't read {}: {e}", bridge_dir.display()))
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "swift"))
        .collect();
    paths.sort();
    paths
        .into_iter()
        .map(|path| read_file(&path))
        .collect::<Vec<_>>()
        .join("\n")
}

fn report(
    framework: &str,
    apple: &BTreeSet<String>,
    referenced_in_bridge: &BTreeSet<String>,
    omitted: &BTreeSet<String>,
) -> Result<(), String> {
    let wrapped: BTreeSet<&String> = apple.intersection(referenced_in_bridge).collect();
    let missing: BTreeSet<&String> = apple
        .difference(referenced_in_bridge)
        .filter(|s| !omitted.contains(*s))
        .collect();

    let coverable = wrapped.len() + missing.len();
    let pct = if coverable == 0 {
        100.0
    } else {
        wrapped.len() as f64 / coverable as f64 * 100.0
    };

    println!(
        "\n=== {framework} API coverage ===\n\
         Apple symbols (Obj-C methods + properties): {}\n\
         Intentionally omitted:                       {}\n\
         ----\n\
         Coverable:                                   {coverable}\n\
         Wrapped (referenced in Swift bridge):        {} ({pct:.1}%)\n\
         Missing (gap):                               {}",
        apple.len(),
        omitted.len(),
        wrapped.len(),
        missing.len(),
    );

    if !missing.is_empty() {
        println!("\n--- Missing ---");
        for s in &missing {
            println!("  - {s}");
        }
    }

    if pct < 100.0 {
        return Err(format!(
            "{framework} coverable coverage is {pct:.1}% — every method must be \
             wrapped or in intentionally_omitted()"
        ));
    }
    Ok(())
}

// ---- Allowlists ----

fn av_asset_writer_intentionally_omitted() -> BTreeSet<String> {
    [
        // Deprecated class factory; Swift code uses designated initializers.
        "assetWriterWithURL",
        // Class self-reference from the deprecated factory helper.
        "assetWriter",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn av_asset_writer_input_intentionally_omitted() -> BTreeSet<String> {
    BTreeSet::new()
}

fn av_output_settings_assistant_intentionally_omitted() -> BTreeSet<String> {
    BTreeSet::new()
}

fn av_asset_export_session_intentionally_omitted() -> BTreeSet<String> {
    [
        // Deprecated properties replaced by async estimators.
        "maxDuration",
        "estimatedOutputFileLength",
        // Still deferred until this crate grows a safe constant layer.
        "audioTimePitchAlgorithm",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

// ---- Test cases ----

/// Obj-C method-name → Swift-name overrides for cases where Swift's
/// importer drops the leading-keyword argument noun (e.g.
/// `appendSampleBuffer:` → `append(_:)`).
#[allow(clippy::too_many_lines)]
fn objc_to_swift_aliases() -> std::collections::BTreeMap<&'static str, Vec<&'static str>> {
    [
        ("appendSampleBuffer", vec![".append(sampleBuffer)"]),
        ("addInput", vec![".add(input)"]),
        ("canAddInput", vec![".canAdd(input)"]),
        ("canAddInputGroup", vec![".canAdd(group)"]),
        ("addInputGroup", vec![".add(group)"]),
        (
            "startSessionAtSourceTime",
            vec!["startSession(atSourceTime:"],
        ),
        ("endSessionAtSourceTime", vec!["endSession(atSourceTime:"]),
        (
            "finishWritingWithCompletionHandler",
            vec!["finishWriting {"],
        ),
        ("canApplyOutputSettings", vec!["canApply(outputSettings:"]),
        ("initWithURL", vec!["AVAssetWriter(outputURL:"]),
        ("initWithContentType", vec!["AVAssetWriter(contentType:"]),
        (
            "assetWriterInputGroupWithInputs",
            vec!["AVAssetWriterInputGroup(inputs:"],
        ),
        ("initWithInputs", vec!["AVAssetWriterInputGroup(inputs:"]),
        (
            "assetWriterInputWithMediaType",
            vec!["AVAssetWriterInput(mediaType:"],
        ),
        ("initWithMediaType", vec!["AVAssetWriterInput(mediaType:"]),
        (
            "assetWriterInputPixelBufferAdaptorWithAssetWriterInput",
            vec!["AVAssetWriterInputPixelBufferAdaptor("],
        ),
        (
            "assetWriterInputTaggedPixelBufferGroupAdaptorWithAssetWriterInput",
            vec!["AVAssetWriterInputTaggedPixelBufferGroupAdaptor("],
        ),
        (
            "assetWriterInputMetadataAdaptorWithAssetWriterInput",
            vec!["AVAssetWriterInputMetadataAdaptor(assetWriterInput:"],
        ),
        (
            "assetWriterInputCaptionAdaptorWithAssetWriterInput",
            vec!["AVAssetWriterInputCaptionAdaptor(assetWriterInput:"],
        ),
        (
            "initWithAssetWriterInput",
            vec![
                "AVAssetWriterInputPixelBufferAdaptor(assetWriterInput:",
                "AVAssetWriterInputTaggedPixelBufferGroupAdaptor(assetWriterInput:",
                "AVAssetWriterInputMetadataAdaptor(assetWriterInput:",
                "AVAssetWriterInputCaptionAdaptor(assetWriterInput:",
            ],
        ),
        (
            "appendPixelBuffer",
            vec!["append(pixelBuffer, withPresentationTime:"],
        ),
        (
            "appendTaggedPixelBufferGroup",
            vec!["av_writer_append_tagged_pixel_buffer_group"],
        ),
        (
            "appendTimedMetadataGroup",
            vec!["av_writer_append_timed_metadata_group_json"],
        ),
        ("appendCaption", vec!["av_writer_append_caption_json"]),
        (
            "appendCaptionGroup",
            vec!["av_writer_append_caption_group_json"],
        ),
        (
            "outputSettingsAssistantWithPreset",
            vec!["AVOutputSettingsAssistant(preset:"],
        ),
        (
            "exportSessionWithAsset",
            vec!["AVAssetExportSession(asset:"],
        ),
        ("initWithAsset", vec!["AVAssetExportSession(asset:"]),
        (
            "exportPresetsCompatibleWithAsset",
            vec!["exportPresets(compatibleWith:"],
        ),
        (
            "determineCompatibilityOfExportPreset",
            vec![
                "determineCompatibility(ofExportPreset:",
                "AVAssetExportSession.determineCompatibility(",
            ],
        ),
        (
            "determineCompatibleFileTypesWithCompletionHandler",
            vec!["determineCompatibleFileTypes {"],
        ),
        (
            "exportAsynchronouslyWithCompletionHandler",
            vec!["exportAsynchronously {"],
        ),
        (
            "estimateMaximumDurationWithCompletionHandler",
            vec![
                "estimatedMaximumDuration",
                "av_export_session_estimated_maximum_duration_json",
            ],
        ),
        (
            "estimateOutputFileLengthWithCompletionHandler",
            vec![
                "estimatedOutputFileLengthInBytes",
                "av_export_session_estimated_output_file_length",
            ],
        ),
        (
            "requestMediaDataWhenReadyOnQueue",
            vec!["requestMediaDataWhenReady(on:"],
        ),
        (
            "respondToEachPassDescriptionOnQueue",
            vec!["respondToEachPassDescription(on:"],
        ),
        (
            "canAddTrackAssociationWithTrackOfInput",
            vec!["canAddTrackAssociation(withTrackOf:"],
        ),
        (
            "addTrackAssociationWithTrackOfInput",
            vec!["addTrackAssociation(withTrackOf:"],
        ),
    ]
    .into_iter()
    .collect()
}

#[test]
fn av_asset_writer_coverage() {
    let sdk = sdk_root();
    let header =
        sdk.join("System/Library/Frameworks/AVFoundation.framework/Headers/AVAssetWriter.h");
    let apple = extract_objc_surface(&read_file(&header));
    let bridge = read_our_swift_bridge();
    let aliases = objc_to_swift_aliases();

    let referenced: BTreeSet<String> = apple
        .iter()
        .filter(|name| {
            // First: textual Swift-import-style match (the bare method/property
            // name appears in the bridge).
            let needle = format!(r"\b{}", regex_lite::escape(name));
            if regex_lite::Regex::new(&needle).unwrap().is_match(&bridge) {
                return true;
            }
            // Second: Obj-C → Swift renamed alias.
            if let Some(swift_forms) = aliases.get(name.as_str()) {
                return swift_forms
                    .iter()
                    .any(|swift_form| bridge.contains(swift_form));
            }
            false
        })
        .cloned()
        .collect();

    report(
        "AVAssetWriter",
        &apple,
        &referenced,
        &av_asset_writer_intentionally_omitted(),
    )
    .unwrap();
}

#[test]
fn av_asset_writer_input_coverage() {
    let sdk = sdk_root();
    let header =
        sdk.join("System/Library/Frameworks/AVFoundation.framework/Headers/AVAssetWriterInput.h");
    let apple = extract_objc_surface(&read_file(&header));
    let bridge = read_our_swift_bridge();
    let aliases = objc_to_swift_aliases();

    let referenced: BTreeSet<String> = apple
        .iter()
        .filter(|name| {
            let needle = format!(r"\b{}", regex_lite::escape(name));
            if regex_lite::Regex::new(&needle).unwrap().is_match(&bridge) {
                return true;
            }
            if let Some(swift_forms) = aliases.get(name.as_str()) {
                return swift_forms
                    .iter()
                    .any(|swift_form| bridge.contains(swift_form));
            }
            false
        })
        .cloned()
        .collect();

    report(
        "AVAssetWriterInput",
        &apple,
        &referenced,
        &av_asset_writer_input_intentionally_omitted(),
    )
    .unwrap();
}

#[test]
fn av_file_type_coverage() {
    let sdk = sdk_root();
    let header =
        sdk.join("System/Library/Frameworks/AVFoundation.framework/Headers/AVMediaFormat.h");
    let apple = extract_by_pattern(
        r"AVF_EXPORT\s+AVFileType\s+const\s+(AVFileType[A-Za-z0-9]+)",
        &read_file(&header),
    );

    let bridge = read_our_swift_bridge();
    let supported: [(&str, &str); 25] = [
        ("AVFileTypeQuickTimeMovie", "AVFileType.mov"),
        ("AVFileTypeMPEG4", "AVFileType.mp4"),
        ("AVFileTypeAppleM4V", "AVFileType.m4v"),
        ("AVFileTypeAppleM4A", "AVFileType.m4a"),
        ("AVFileType3GPP", "AVFileType.mobile3GPP"),
        ("AVFileType3GPP2", "AVFileType.mobile3GPP2"),
        ("AVFileTypeCoreAudioFormat", "AVFileType.caf"),
        ("AVFileTypeWAVE", "AVFileType.wav"),
        ("AVFileTypeAIFF", "AVFileType.aiff"),
        ("AVFileTypeAIFC", "AVFileType.aifc"),
        ("AVFileTypeAMR", "AVFileType.amr"),
        ("AVFileTypeMPEGLayer3", "AVFileType.mp3"),
        ("AVFileTypeSunAU", "AVFileType.au"),
        ("AVFileTypeAC3", "AVFileType.ac3"),
        ("AVFileTypeEnhancedAC3", "AVFileType.eac3"),
        ("AVFileTypeJPEG", "AVFileType.jpg"),
        ("AVFileTypeDNG", "AVFileType.dng"),
        ("AVFileTypeHEIC", "AVFileType.heic"),
        ("AVFileTypeAVCI", "AVFileType.avci"),
        ("AVFileTypeHEIF", "AVFileType.heif"),
        ("AVFileTypeTIFF", "AVFileType.tif"),
        ("AVFileTypeAppleiTT", "AVFileType.appleiTT"),
        ("AVFileTypeSCC", "AVFileType.SCC"),
        ("AVFileTypeAHAP", "AVFileType.AHAP"),
        ("AVFileTypeQuickTimeAudio", "AVFileType.qta"),
    ];
    let mut supported = supported.into_iter().collect::<Vec<_>>();
    supported.push(("AVFileTypeDICOM", "AVFileType.dcm"));

    let our_avfiletype_accesses: BTreeSet<String> = supported
        .iter()
        .filter(|(_, needle)| bridge.contains(needle))
        .map(|(symbol, _)| (*symbol).to_string())
        .collect();

    let kept: BTreeSet<&str> = supported.iter().map(|(symbol, _)| *symbol).collect();
    let omitted: BTreeSet<String> = apple
        .iter()
        .filter(|s| !kept.contains(s.as_str()))
        .cloned()
        .collect();

    report("AVFileType", &apple, &our_avfiletype_accesses, &omitted).unwrap();
}

#[test]
fn av_output_settings_assistant_coverage() {
    let sdk = sdk_root();
    let header = sdk.join(
        "System/Library/Frameworks/AVFoundation.framework/Headers/AVOutputSettingsAssistant.h",
    );
    let apple = extract_objc_surface(&read_file(&header));
    let bridge = read_our_swift_bridge();
    let aliases = objc_to_swift_aliases();

    let referenced: BTreeSet<String> = apple
        .iter()
        .filter(|name| {
            let needle = format!(r"\b{}", regex_lite::escape(name));
            if regex_lite::Regex::new(&needle).unwrap().is_match(&bridge) {
                return true;
            }
            if let Some(swift_forms) = aliases.get(name.as_str()) {
                return swift_forms
                    .iter()
                    .any(|swift_form| bridge.contains(swift_form));
            }
            false
        })
        .cloned()
        .collect();

    report(
        "AVOutputSettingsAssistant",
        &apple,
        &referenced,
        &av_output_settings_assistant_intentionally_omitted(),
    )
    .unwrap();
}

#[test]
fn av_asset_export_session_coverage() {
    let sdk = sdk_root();
    let header =
        sdk.join("System/Library/Frameworks/AVFoundation.framework/Headers/AVAssetExportSession.h");
    let apple = extract_objc_surface(&read_file(&header));
    let bridge = read_our_swift_bridge();
    let aliases = objc_to_swift_aliases();

    let referenced: BTreeSet<String> = apple
        .iter()
        .filter(|name| {
            let needle = format!(r"\b{}", regex_lite::escape(name));
            if regex_lite::Regex::new(&needle).unwrap().is_match(&bridge) {
                return true;
            }
            if let Some(swift_forms) = aliases.get(name.as_str()) {
                return swift_forms
                    .iter()
                    .any(|swift_form| bridge.contains(swift_form));
            }
            false
        })
        .cloned()
        .collect();

    report(
        "AVAssetExportSession",
        &apple,
        &referenced,
        &av_asset_export_session_intentionally_omitted(),
    )
    .unwrap();
}

#[test]
fn av_output_settings_preset_coverage() {
    let sdk = sdk_root();
    let header = sdk.join(
        "System/Library/Frameworks/AVFoundation.framework/Headers/AVOutputSettingsAssistant.h",
    );
    let apple = extract_by_pattern(
        r"AVF_EXPORT\s+AVOutputSettingsPreset\s+const\s+(AVOutputSettingsPreset[A-Za-z0-9]+)",
        &read_file(&header),
    );

    let bridge = read_our_swift_bridge();
    let supported = [
        ("AVOutputSettingsPreset640x480", ".preset640x480"),
        ("AVOutputSettingsPreset960x540", ".preset960x540"),
        ("AVOutputSettingsPreset1280x720", ".preset1280x720"),
        ("AVOutputSettingsPreset1920x1080", ".preset1920x1080"),
        ("AVOutputSettingsPreset3840x2160", ".preset3840x2160"),
        ("AVOutputSettingsPresetHEVC1920x1080", ".hevc1920x1080"),
        (
            "AVOutputSettingsPresetHEVC1920x1080WithAlpha",
            ".hevc1920x1080WithAlpha",
        ),
        ("AVOutputSettingsPresetHEVC3840x2160", ".hevc3840x2160"),
        (
            "AVOutputSettingsPresetHEVC3840x2160WithAlpha",
            ".hevc3840x2160WithAlpha",
        ),
        ("AVOutputSettingsPresetHEVC4320x2160", ".hevc4320x2160"),
        ("AVOutputSettingsPresetHEVC7680x4320", ".hevc7680x4320"),
        ("AVOutputSettingsPresetMVHEVC960x960", ".mvhevc960x960"),
        ("AVOutputSettingsPresetMVHEVC1440x1440", ".mvhevc1440x1440"),
        ("AVOutputSettingsPresetMVHEVC4320x4320", ".mvhevc4320x4320"),
        ("AVOutputSettingsPresetMVHEVC7680x7680", ".mvhevc7680x7680"),
    ];

    let referenced: BTreeSet<String> = supported
        .iter()
        .filter(|(_, needle)| bridge.contains(needle))
        .map(|(symbol, _)| (*symbol).to_string())
        .collect();

    report(
        "AVOutputSettingsPreset",
        &apple,
        &referenced,
        &BTreeSet::new(),
    )
    .unwrap();
}

#[test]
fn av_asset_export_preset_coverage() {
    let sdk = sdk_root();
    let header =
        sdk.join("System/Library/Frameworks/AVFoundation.framework/Headers/AVAssetExportSession.h");
    let apple = extract_by_pattern(
        r"AVF_EXPORT\s+NSString\s+\*const\s+(AVAssetExportPreset[A-Za-z0-9]+)",
        &read_file(&header),
    );

    let bridge = read_our_swift_bridge();
    let referenced: BTreeSet<String> = apple
        .iter()
        .filter(|symbol| bridge.contains(symbol.as_str()))
        .cloned()
        .collect();

    report("AVAssetExportPreset", &apple, &referenced, &BTreeSet::new()).unwrap();
}

#[test]
fn av_audio_settings_keys_coverage() {
    // The audio-input track is configured via a `[AVAudio*Key: value]`
    // dictionary. Verify every key our bridge uses is a real, currently-
    // valid `AVAudioSettings.h` constant — so a future SDK rename surfaces
    // here instead of as a runtime no-op (Apple silently ignores unknown
    // dictionary keys in `outputSettings`).
    let sdk = sdk_root();
    let header = sdk.join("System/Library/Frameworks/AVFAudio.framework/Headers/AVAudioSettings.h");
    let apple = extract_by_pattern(
        r"extern\s+NSString\s+\*\s*const\s+(AV[A-Za-z0-9]+Key)\b",
        &read_file(&header),
    );

    let bridge = read_our_swift_bridge();
    let our_keys: BTreeSet<String> = apple
        .iter()
        .filter(|name| {
            // Swift translates `AVFormatIDKey` into the keypath form
            // `AVFormatIDKey:` inside dictionary literals — match the bare
            // identifier.
            let needle = format!(r"\b{}\b", regex_lite::escape(name));
            regex_lite::Regex::new(&needle).unwrap().is_match(&bridge)
        })
        .cloned()
        .collect();

    // We use AVFormatIDKey + AVSampleRateKey + AVNumberOfChannelsKey +
    // AVEncoderBitRateKey for our AAC audio track. Every other key is
    // intentionally omitted in v0.2.
    let kept: BTreeSet<&str> = [
        "AVFormatIDKey",
        "AVSampleRateKey",
        "AVNumberOfChannelsKey",
        "AVEncoderBitRateKey",
    ]
    .into_iter()
    .collect();
    let omitted: BTreeSet<String> = apple
        .iter()
        .filter(|s| !kept.contains(s.as_str()))
        .cloned()
        .collect();

    report("AVAudioSettings keys", &apple, &our_keys, &omitted).unwrap();
}

fn extract_by_pattern(pattern: &str, source: &str) -> BTreeSet<String> {
    let re = regex_lite::Regex::new(pattern).unwrap();
    re.captures_iter(source).map(|c| c[1].to_string()).collect()
}
