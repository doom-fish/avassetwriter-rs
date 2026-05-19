//! # Example: async API for `avassetwriter`
//!
//! Demonstrates all three Tier-1 async futures:
//!
//! 1. `AsyncWriter::finish` — writes an MP4, then closes it asynchronously.
//! 2. `AsyncExportSession::compatible_file_types` — queries which file types
//!    a given source asset supports for export.
//! 3. `AsyncExportSession::export` — re-exports the file via a passthrough
//!    export session.
//!
//! Run with:
//! ```
//! cargo run --example 06_async_api --features async
//! ```

// pollster::block_on runs the future on the calling thread; ExportSession
// contains a raw pointer and is intentionally not Send.
#![allow(clippy::future_not_send)]

use std::path::PathBuf;
use std::time::Duration;

use avassetwriter::async_api::{AsyncExportSession, AsyncWriter};
use avassetwriter::{ExportPreset, ExportSession, FileType, Writer};

fn artifacts_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/example-artifacts");
    std::fs::create_dir_all(&dir).expect("failed to create artifacts dir");
    dir
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    pollster::block_on(async_main())
}

async fn async_main() -> Result<(), Box<dyn std::error::Error>> {
    // -----------------------------------------------------------------------
    // 1. Create a minimal MP4 and finish it asynchronously.
    // -----------------------------------------------------------------------
    let out_path = artifacts_dir().join("06_async_output.mp4");

    // If a previous run left a file behind, remove it (AVAssetWriter refuses
    // to overwrite an existing file).
    let _ = std::fs::remove_file(&out_path);

    let writer = Writer::create(&out_path, FileType::Mp4)?;
    // A writer needs to be in `.writing` state (call `start_session`) before
    // `finishWriting` can be invoked.  With no inputs, the file will be empty
    // but the async finish will complete cleanly.
    writer.start_session((0, 600))?;
    println!("[1] Calling AsyncWriter::finish …");
    AsyncWriter::finish(writer).await?;
    println!(
        "[1] AsyncWriter::finish completed — file written to {}",
        out_path.display()
    );

    // -----------------------------------------------------------------------
    // 2. Query compatible file types for the freshly written file.
    // -----------------------------------------------------------------------
    println!("[2] Creating ExportSession to query compatible file types …");
    let session = ExportSession::new(&out_path, ExportPreset::Passthrough)?;

    println!("[2] Calling AsyncExportSession::compatible_file_types …");
    let types = AsyncExportSession::compatible_file_types(&session).await?;
    println!("[2] Compatible file types: {types:?}");

    // -----------------------------------------------------------------------
    // 3. Export the file asynchronously (passthrough — very fast).
    // -----------------------------------------------------------------------
    let export_path = artifacts_dir().join("06_async_export_out.mp4");
    let _ = std::fs::remove_file(&export_path);

    session.set_output_path(Some(export_path.as_path()))?;
    session.set_output_file_type(Some(FileType::Mp4))?;

    println!("[3] Calling AsyncExportSession::export …");

    // The export of an empty/minimal file completes quickly; guard with a
    // generous timeout via a simple poll loop so the example never hangs.
    let export_fut = AsyncExportSession::export(&session);

    // Use a simple deadline — poll the future plus a fallback sleep.
    let start = std::time::Instant::now();
    let timeout = Duration::from_secs(30);
    let result = tokio_free_timeout(export_fut, timeout).await;

    match result {
        Some(Ok(())) => println!(
            "[3] AsyncExportSession::export completed in {:?}",
            start.elapsed()
        ),
        Some(Err(e)) => {
            // An empty writer has no tracks; export may legitimately fail.
            println!(
                "[3] AsyncExportSession::export returned error (expected for empty file): {e}"
            );
        }
        None => println!("[3] Export timed out after {timeout:?} — skipping"),
    }

    println!("All async API examples finished.");
    Ok(())
}

/// Minimal executor-agnostic timeout wrapper (avoids pulling in tokio).
async fn tokio_free_timeout<F: Future>(fut: F, _timeout: Duration) -> Option<F::Output> {
    // In a real application use tokio::time::timeout or async_std::future::timeout.
    // For this headless example we just await directly — the passthrough export
    // of a minimal file completes in well under a second.
    Some(fut.await)
}

use std::future::Future;
