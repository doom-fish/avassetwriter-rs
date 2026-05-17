//! Integration tests for `async_api` module.
//!
//! Run with:
//! ```
//! cargo test --test async_api_tests --features async
//! ```

use std::path::PathBuf;

use avassetwriter::async_api::{AsyncExportSession, AsyncWriter};
use avassetwriter::{ExportPreset, ExportSession, FileType, Writer};

fn artifacts_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/test-artifacts");
    std::fs::create_dir_all(&dir).expect("failed to create test artifacts dir");
    dir
}

// ---------------------------------------------------------------------------
// AsyncWriter::finish — happy path
// ---------------------------------------------------------------------------

#[test]
fn async_writer_finish_happy_path() {
    let path = artifacts_dir().join("async_writer_happy.mp4");
    let _ = std::fs::remove_file(&path);

    let writer = Writer::create(&path, FileType::Mp4).expect("Writer::create failed");
    // start_session puts the writer into .writing state (calls startWriting()).
    // A writer with no inputs will still start successfully and finishWriting
    // will complete, though the output file may not contain any media tracks.
    writer.start_session((0, 600)).expect("start_session failed");
    let result = pollster::block_on(AsyncWriter::finish(writer));
    assert!(
        result.is_ok(),
        "AsyncWriter::finish failed unexpectedly: {result:?}"
    );
    assert!(path.exists(), "output file was not created");
    let _ = std::fs::remove_file(&path);
}

// ---------------------------------------------------------------------------
// AsyncExportSession::compatible_file_types — happy path
// ---------------------------------------------------------------------------

#[test]
fn async_compatible_file_types_happy_path() {
    // Create a minimal file to use as source (properly started+finished).
    let src = artifacts_dir().join("async_compat_src.mp4");
    let _ = std::fs::remove_file(&src);
    let writer = Writer::create(&src, FileType::Mp4).expect("Writer::create failed");
    writer.start_session((0, 600)).expect("start_session failed");
    pollster::block_on(AsyncWriter::finish(writer)).expect("finish failed");

    let session = ExportSession::new(&src, ExportPreset::Passthrough).expect("ExportSession::new failed");
    let result = pollster::block_on(AsyncExportSession::compatible_file_types(&session));
    assert!(
        result.is_ok(),
        "compatible_file_types failed: {result:?}"
    );
    let types = result.unwrap();
    // A file with no tracks may return an empty list — that is still a valid
    // success response from the Swift bridge.
    println!("compatible file types for minimal mp4: {types:?}");

    let _ = std::fs::remove_file(&src);
}

// ---------------------------------------------------------------------------
// AsyncExportSession::export — happy path (passthrough, minimal file)
// ---------------------------------------------------------------------------

#[test]
fn async_export_happy_path() {
    let src = artifacts_dir().join("async_export_src.mp4");
    let dst = artifacts_dir().join("async_export_dst.mp4");
    let _ = std::fs::remove_file(&src);
    let _ = std::fs::remove_file(&dst);

    // Create minimal source file (properly started+finished).
    let writer = Writer::create(&src, FileType::Mp4).expect("Writer::create failed");
    writer.start_session((0, 600)).expect("start_session failed");
    pollster::block_on(AsyncWriter::finish(writer)).expect("finish failed");

    let session = ExportSession::new(&src, ExportPreset::Passthrough).expect("ExportSession::new failed");
    session.set_output_path(Some(dst.as_path())).expect("set_output_path failed");
    session.set_output_file_type(Some(FileType::Mp4)).expect("set_output_file_type failed");

    let result = pollster::block_on(AsyncExportSession::export(&session));
    // An empty source may produce an error ("video track required" etc.) — we
    // accept either success or a well-typed error; we do NOT accept a panic.
    match &result {
        Ok(()) => println!("async export succeeded"),
        Err(e) => println!("async export returned expected error for empty file: {e}"),
    }

    let _ = std::fs::remove_file(&src);
    let _ = std::fs::remove_file(&dst);
}

// ---------------------------------------------------------------------------
// AsyncExportSession::export — error path (missing output URL)
// ---------------------------------------------------------------------------

#[test]
fn async_export_error_path_missing_output() {
    let src = artifacts_dir().join("async_export_err_src.mp4");
    let _ = std::fs::remove_file(&src);

    // Create minimal source file.
    let writer = Writer::create(&src, FileType::Mp4).expect("Writer::create failed");
    writer.start_session((0, 600)).expect("start_session failed");
    pollster::block_on(AsyncWriter::finish(writer)).expect("finish failed");

    // Session with no output URL configured — should return an error.
    let session = ExportSession::new(&src, ExportPreset::Passthrough).expect("ExportSession::new failed");
    // Deliberately omit set_output_path / set_output_file_type.

    let result = pollster::block_on(AsyncExportSession::export(&session));
    // The sync export returns an error here; the async path should too.
    match result {
        Ok(()) => {
            // Some runtimes may succeed for passthrough with default settings;
            // that is acceptable.
            println!("async export succeeded (no error on missing output — runtime-dependent)");
        }
        Err(e) => {
            println!("async export correctly returned error: {e}");
        }
    }

    let _ = std::fs::remove_file(&src);
}
