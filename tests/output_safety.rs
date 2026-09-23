use apple_cf::cm::CMTime;
use std::path::{Path, PathBuf};

use avassetwriter::{AVWriterError, ExportPreset, ExportSession, FileType, OutputPolicy, Writer};

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/test-artifacts")
        .join(format!("output-safety-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

fn write_silence(writer: Writer) -> Result<(), AVWriterError> {
    let input = writer.add_audio_input_pcm(48_000.0, 1, 16)?;
    writer.start_session(CMTime::new(0, 48_000))?;
    writer.append_audio_pcm(input, &[0_u8; 9_600], 4_800, CMTime::new(0, 48_000))?;
    writer.finish()
}

fn seed_directory(dir: &Path) {
    std::fs::create_dir_all(dir).expect("seed directory");
    std::fs::write(dir.join("clip.mov"), b"precious").expect("seed directory contents");
}

fn assert_directory_intact(dir: &Path) {
    assert!(dir.is_dir());
    assert_eq!(
        std::fs::read(dir.join("clip.mov")).expect("directory contents kept"),
        b"precious"
    );
}

#[test]
fn writer_refuses_an_existing_file_and_leaves_it_intact() {
    let dir = scratch("writer-file");
    let path = dir.join("existing.m4a");
    std::fs::write(&path, b"keep me").expect("seed file");

    let result = Writer::create(&path, FileType::M4a);

    assert!(matches!(result, Err(AVWriterError::OutputExists(_))));
    assert_eq!(std::fs::read(&path).expect("file kept"), b"keep me");
    std::fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn writer_never_removes_a_directory() {
    let dir = scratch("writer-directory");
    let movies = dir.join("Movies");
    seed_directory(&movies);

    for policy in [OutputPolicy::FailIfExists, OutputPolicy::ReplaceFile] {
        let result = Writer::create_with_policy(&movies, FileType::Mp4, policy);
        assert!(matches!(result, Err(AVWriterError::OutputExists(_))));
        assert_directory_intact(&movies);
    }
    std::fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn writer_replace_policy_replaces_a_regular_file() {
    let dir = scratch("writer-replace");
    let path = dir.join("take.m4a");
    std::fs::write(&path, b"stale").expect("seed file");

    let writer = Writer::create_with_policy(&path, FileType::M4a, OutputPolicy::ReplaceFile)
        .expect("regular file is replaced");
    write_silence(writer).expect("write replacement");

    let written = std::fs::read(&path).expect("replacement written");
    assert!(written.len() > b"stale".len());
    assert_eq!(&written[4..8], b"ftyp");
    std::fs::remove_dir_all(dir).expect("cleanup");
}

fn audio_export_session(dir: &Path) -> (PathBuf, ExportSession) {
    let source = dir.join("source.m4a");
    write_silence(Writer::create(&source, FileType::M4a).expect("create source"))
        .expect("write source");
    let presets = ExportSession::compatible_presets(&source).expect("compatible presets");
    assert!(presets.contains(&ExportPreset::AppleM4A));
    let session = ExportSession::new(&source, ExportPreset::AppleM4A).expect("export session");
    let file_type = session
        .compatible_file_types()
        .expect("compatible file types")
        .into_iter()
        .find(|file_type| *file_type == FileType::M4a)
        .expect("m4a output");
    session
        .set_output_file_type(Some(file_type))
        .expect("set output file type");
    (source, session)
}

#[test]
fn export_refuses_an_existing_output_unless_replacing_a_file() {
    let dir = scratch("export-file");
    let (_source, session) = audio_export_session(&dir);
    let output = dir.join("exported.m4a");
    std::fs::write(&output, b"keep me").expect("seed output");
    session.set_output_path(Some(&output)).expect("set output");

    assert_eq!(session.output_policy(), OutputPolicy::FailIfExists);
    assert!(matches!(
        session.export(),
        Err(AVWriterError::OutputExists(_))
    ));
    assert_eq!(std::fs::read(&output).expect("output kept"), b"keep me");

    session.set_output_policy(OutputPolicy::ReplaceFile);
    session.export().expect("export replaces the file");
    let written = std::fs::read(&output).expect("export written");
    assert_eq!(&written[4..8], b"ftyp");
    std::fs::remove_dir_all(dir).expect("cleanup");
}

#[test]
fn export_never_removes_a_directory_or_its_own_source() {
    let dir = scratch("export-directory");
    let (source, session) = audio_export_session(&dir);
    let movies = dir.join("Movies");
    seed_directory(&movies);
    let source_bytes = std::fs::read(&source).expect("source bytes");

    session.set_output_path(Some(&movies)).expect("set output");
    for policy in [OutputPolicy::FailIfExists, OutputPolicy::ReplaceFile] {
        session.set_output_policy(policy);
        assert!(matches!(
            session.export(),
            Err(AVWriterError::OutputExists(_))
        ));
        assert_directory_intact(&movies);
    }

    session.set_output_path(Some(&source)).expect("set output");
    assert!(matches!(
        session.export(),
        Err(AVWriterError::InvalidArgument(_))
    ));
    assert_eq!(std::fs::read(&source).expect("source kept"), source_bytes);
    std::fs::remove_dir_all(dir).expect("cleanup");
}

#[cfg(feature = "async")]
#[test]
fn async_export_refuses_an_existing_output() {
    use avassetwriter::async_api::AsyncExportSession;

    let dir = scratch("export-async");
    let (_source, session) = audio_export_session(&dir);
    let output = dir.join("exported.m4a");
    std::fs::write(&output, b"keep me").expect("seed output");
    session.set_output_path(Some(&output)).expect("set output");

    let result = pollster::block_on(AsyncExportSession::export(&session));

    assert!(matches!(result, Err(AVWriterError::OutputExists(_))));
    assert_eq!(std::fs::read(&output).expect("output kept"), b"keep me");
    std::fs::remove_dir_all(dir).expect("cleanup");
}
