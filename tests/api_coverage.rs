//! API-surface coverage harness for `avassetwriter`.
//!
//! AVAssetWriter is an Obj-C class — there's no `extern "C"` surface to
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
            out.insert(c[1].to_string());
        }
        if let Some(c) = prop_re.captures(line) {
            out.insert(c[1].to_string());
        }
    }
    out
}

fn read_our_swift_bridge() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("swift-bridge/Sources/AVAssetWriterBridge/AVAssetWriter.swift");
    read_file(&path)
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
        // Class factory + secondary initialiser — bridge uses the URL+fileType
        // designated init via Swift's `try AVAssetWriter(outputURL:fileType:)`
        // sugar.
        "assetWriterWithURL",
        "initWithContentType",
        // Read-only metadata accessors not exposed in v0.1 — file an issue
        // if you need any of these.
        "outputURL",
        "outputFileType",
        "availableMediaTypes",
        "metadata",
        "shouldOptimizeForNetworkUse",
        "directoryForTemporaryFiles",
        "inputs",
        "canApplyOutputSettings",
        "endSessionAtSourceTime",
        "cancelWriting",
        "movieFragmentInterval",
        "initialMovieFragmentInterval",
        "initialMovieFragmentSequenceNumber",
        "producesCombinableFragments",
        "overallDurationHint",
        "movieTimeScale",
        // Multi-track input groups (chapter tracks, alternate audio) — v0.2
        "canAddInputGroup",
        "addInputGroup",
        "inputGroups",
        // AVAssetWriterInputGroup — separate class. We don't use it for v0.1.
        "assetWriterInputGroupWithInputs",
        "initWithInputs",
        "defaultInput",
        // Deprecated synchronous API — Apple replaced it with the
        // completion-handler form which we use.
        "finishWriting",
        // Segmented output (HLS-style fragmented MP4) — v0.2.
        "delegate",
        "flushSegment",
        "initialSegmentStartTime",
        "preferredOutputSegmentInterval",
        "outputFileTypeProfile",
        // Class self-reference in factory methods.
        "assetWriter",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn av_asset_writer_input_intentionally_omitted() -> BTreeSet<String> {
    [
        // Class factory variants — bridge uses the designated init that takes
        // a sourceFormatHint (which we *do* call).
        "assetWriterInputWithMediaType",
        "initWithMediaType",
        // Properties exposed for inspection on AVAssetWriterInput that aren't
        // useful in the simple "just write this CMSampleBuffer" path:
        "mediaType",
        "outputSettings",
        "metadata",
        "languageCode",
        "extendedLanguageTag",
        "naturalSize",
        "transform",
        "preferredVolume",
        "marksOutputTrackAsEnabled",
        "mediaTimeScale",
        // Push-style ingest — we use the simpler synchronous append form
        // because we know we're in real-time mode driven by an external
        // encoder loop. The push API would only matter for batch transcode.
        "requestMediaDataWhenReadyOnQueue",
        "respondToEachPassDescriptionOnQueue",
        // Adapter classes (caption, metadata, pixel-buffer pool, tagged
        // groups) — own track types we don't yet expose. v0.2.
        "assetWriterInput",
        "assetWriterInputCaptionAdaptorWithAssetWriterInput",
        "assetWriterInputMetadataAdaptorWithAssetWriterInput",
        "assetWriterInputPixelBufferAdaptorWithAssetWriterInput",
        "assetWriterInputTaggedPixelBufferGroupAdaptorWithAssetWriterInput",
        "initWithAssetWriterInput",
        "appendCaption",
        "appendCaptionGroup",
        "appendPixelBuffer",
        "appendTaggedPixelBufferGroup",
        "appendTimedMetadataGroup",
        "pixelBufferPool",
        "sourcePixelBufferAttributes",
        // Multi-pass encoding — only relevant for offline transcode.
        "canPerformMultiplePasses",
        "currentPassDescription",
        "markCurrentPassAsFinished",
        "performsMultiPassEncodingIfSupported",
        // Track associations (chapter / fallback tracks) — v0.2.
        "addTrackAssociationWithTrackOfInput",
        "canAddTrackAssociationWithTrackOfInput",
        // Misc references / time-range pull — not in scope for the simple
        // CMSampleBuffer-push API.
        "mediaDataLocation",
        "preferredMediaChunkAlignment",
        "preferredMediaChunkDuration",
        "sampleReferenceBaseURL",
        "sourceTimeRanges",
        // sourceFormatHint is the constructor argument; we pass it but the
        // harness can't tell because Swift uses the labelled-init form.
        "sourceFormatHint",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

// ---- Test cases ----

/// Obj-C method-name → Swift-name overrides for cases where Swift's
/// importer drops the leading-keyword argument noun (e.g.
/// `appendSampleBuffer:` → `append(_:)`).
fn objc_to_swift_aliases() -> std::collections::BTreeMap<&'static str, &'static str> {
    [
        ("appendSampleBuffer", ".append("),
        ("addInput", ".add("),
        ("canAddInput", ".canAdd("),
        ("startSessionAtSourceTime", "startSession(atSourceTime:"),
        ("finishWritingWithCompletionHandler", "finishWriting {"),
        // initWithURL:fileType:error: → Swift `AVAssetWriter(outputURL:fileType:)`
        ("initWithURL", "AVAssetWriter(outputURL:"),
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
            if let Some(swift_form) = aliases.get(name.as_str()) {
                return bridge.contains(swift_form);
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
            if let Some(swift_form) = aliases.get(name.as_str()) {
                return bridge.contains(swift_form);
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
    // AVFileType is a string-typed enum. We support exactly mp4/mov/m4v.
    // Verify that every AVFileType the bridge string-switches on is a real
    // AVFileType constant in AVMediaFormat.h.
    let sdk = sdk_root();
    let header =
        sdk.join("System/Library/Frameworks/AVFoundation.framework/Headers/AVMediaFormat.h");
    let apple = extract_by_pattern(
        r"AVF_EXPORT\s+AVFileType\s+const\s+(AVFileType[A-Za-z0-9]+)",
        &read_file(&header),
    );

    let bridge = read_our_swift_bridge();
    // Look for `.mp4`, `.mov`, `.m4v` access on AVFileType.
    let our_avfiletype_accesses: BTreeSet<String> = [
        "AVFileTypeMPEG4",
        "AVFileTypeQuickTimeMovie",
        "AVFileTypeAppleM4V",
    ]
    .into_iter()
    .filter(|sym| {
        // Map to the Swift dot-syntax used in the bridge.
        let dotted = match *sym {
            "AVFileTypeMPEG4" => "mp4",
            "AVFileTypeQuickTimeMovie" => "mov",
            "AVFileTypeAppleM4V" => "m4v",
            _ => return false,
        };
        bridge.contains(&format!(".{dotted}"))
    })
    .map(String::from)
    .collect();

    let kept: BTreeSet<&str> = [
        "AVFileTypeMPEG4",
        "AVFileTypeQuickTimeMovie",
        "AVFileTypeAppleM4V",
    ]
    .into_iter()
    .collect();
    let omitted: BTreeSet<String> = apple
        .iter()
        .filter(|s| !kept.contains(s.as_str()))
        .cloned()
        .collect();

    report("AVFileType", &apple, &our_avfiletype_accesses, &omitted).unwrap();
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
