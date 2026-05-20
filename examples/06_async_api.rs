//! # Example: async API for `avassetwriter`
//!
//! Demonstrates all four async wrappers:
//!
//! 1. `AsyncWriterInput::request_media_data_when_ready` — waits for the first
//!    input-ready callback on an audio writer input.
//! 2. `AsyncWriter::finish` — flushes the audio-only output asynchronously.
//! 3. `AsyncExportSession::compatible_file_types` — queries which file types
//!    the generated source asset supports for export.
//! 4. `AsyncExportSession::export` — re-exports the file via a passthrough
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

use avassetwriter::async_api::{AsyncExportSession, AsyncWriter, AsyncWriterInput};
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
    // 1. Create an audio-only M4A, wait for the first ready callback, append
    //    silence, then finish it asynchronously.
    // -----------------------------------------------------------------------
    let out_path = artifacts_dir().join("06_async_output.m4a");
    let _ = std::fs::remove_file(&out_path);

    let writer = Writer::create(&out_path, FileType::M4a)?;
    let audio_input = writer.add_audio_input_pcm(48_000.0, 1, 16)?;
    writer.start_session((0, 48_000))?;

    println!("[1] Waiting for AsyncWriterInput::request_media_data_when_ready …");
    let ready_stream = AsyncWriterInput::request_media_data_when_ready(&writer, audio_input, 8)?;
    let _ = ready_stream.next().await;

    println!("[1] Input ready — appending 1 second of silence …");
    let silence = vec![0_u8; 48_000 * 2];
    writer.append_audio_pcm(audio_input, &silence, 48_000, (0, 48_000))?;
    drop(ready_stream);

    println!("[2] Calling AsyncWriter::finish …");
    AsyncWriter::finish(writer).await?;
    println!(
        "[2] AsyncWriter::finish completed — file written to {}",
        out_path.display()
    );

    // -----------------------------------------------------------------------
    // 2. Query compatible file types for the freshly written file.
    // -----------------------------------------------------------------------
    println!("[3] Creating ExportSession to query compatible file types …");
    let session = ExportSession::new(&out_path, ExportPreset::Passthrough)?;

    println!("[3] Calling AsyncExportSession::compatible_file_types …");
    let types = AsyncExportSession::compatible_file_types(&session).await?;
    println!("[3] Compatible file types: {types:?}");

    // -----------------------------------------------------------------------
    // 3. Export the file asynchronously (passthrough — very fast).
    // -----------------------------------------------------------------------
    let export_path = artifacts_dir().join("06_async_export_out.m4a");
    let _ = std::fs::remove_file(&export_path);

    session.set_output_path(Some(export_path.as_path()))?;
    session.set_output_file_type(Some(FileType::M4a))?;

    println!("[4] Calling AsyncExportSession::export …");
    AsyncExportSession::export(&session).await?;
    println!(
        "[4] Export completed — file written to {}",
        export_path.display()
    );

    println!("All async API examples finished.");
    Ok(())
}
