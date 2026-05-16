use std::path::PathBuf;

use avassetwriter::{FileType, OutputSettingsAssistant, Time, VideoPreset, Writer};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let artifacts = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/example-artifacts");
    std::fs::create_dir_all(&artifacts)?;

    let presets = OutputSettingsAssistant::available_presets()?;
    println!("available output settings presets: {}", presets.len());
    assert!(presets.contains(&VideoPreset::Hd1280x720));

    let assistant = OutputSettingsAssistant::new(VideoPreset::Hd1280x720)?;
    assistant.set_source_audio_format(None)?;
    assistant.set_source_video_format(None)?;
    assistant.set_source_video_average_frame_duration(Time::new(1, 30))?;
    assistant.set_source_video_min_frame_duration(Time::new(1, 60))?;

    let output = artifacts.join("output-settings-smoke.mov");
    if output.exists() {
        std::fs::remove_file(&output)?;
    }
    let writer = Writer::create(&output, FileType::Mov)?;
    let input = writer.add_video_input_from_preset(VideoPreset::Hd1280x720)?;
    let settings = writer
        .input_output_settings(input)?
        .expect("preset-backed input should expose output settings");

    println!(
        "assistant output type={:?}, video settings keys={} average={:?} min={:?}",
        assistant.output_file_type()?,
        settings.as_object().map_or(0, serde_json::Map::len),
        assistant.source_video_average_frame_duration()?,
        assistant.source_video_min_frame_duration()?,
    );
    Ok(())
}
