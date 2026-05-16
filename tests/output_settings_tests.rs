use std::path::PathBuf;

use avassetwriter::{FileType, MediaType, OutputSettingsAssistant, Time, VideoPreset, Writer};

fn artifacts_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/test-artifacts")
}

#[test]
fn output_settings_assistant_smoke() -> Result<(), Box<dyn std::error::Error>> {
    let presets = OutputSettingsAssistant::available_presets()?;
    assert!(presets.contains(&VideoPreset::Hd1280x720));

    let assistant = OutputSettingsAssistant::new(VideoPreset::Hd1280x720)?;
    assert!(assistant.video_settings()?.is_some());
    assert!(assistant.output_file_type()?.is_some());

    assistant.set_source_audio_format(None)?;
    assistant.set_source_video_format(None)?;
    assistant.set_source_video_average_frame_duration(Time::new(1, 30))?;
    assistant.set_source_video_min_frame_duration(Time::new(1, 60))?;

    assert_eq!(
        assistant.source_video_average_frame_duration()?,
        Time::new(1, 30)
    );
    assert_eq!(
        assistant.source_video_min_frame_duration()?,
        Time::new(1, 60)
    );
    Ok(())
}

#[test]
fn writer_can_add_preset_backed_input() -> Result<(), Box<dyn std::error::Error>> {
    let artifacts = artifacts_dir();
    std::fs::create_dir_all(&artifacts)?;
    let output = artifacts.join("preset-writer.mov");
    if output.exists() {
        std::fs::remove_file(&output)?;
    }

    let writer = Writer::create(&output, FileType::Mov)?;
    let input = writer.add_video_input_from_preset(VideoPreset::Hd1280x720)?;
    assert_eq!(writer.input_media_type(input)?, MediaType::Video);
    assert!(writer.input_output_settings(input)?.is_some());
    Ok(())
}
