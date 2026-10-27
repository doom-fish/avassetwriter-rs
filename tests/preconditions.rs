use std::path::PathBuf;

use apple_cf::cm::{CMTime, CMTimeRange};
use apple_cf::cv::CVPixelBuffer;
use apple_cf::iosurface::IOSurface;
use avassetwriter::PropertyValue;
use avassetwriter::{
    AVWriterError, Caption, CaptionConversionSettings, CaptionConversionValidator,
    CaptionDimension, CaptionFormatConformer, CaptionGroup, CaptionPoint, CaptionRegion,
    CaptionRegionDisplayAlignment, CaptionRegionScroll, CaptionRegionWritingMode, CaptionRenderer,
    CaptionRuby, CaptionSize, CaptionUnitsType, ExportPreset, ExportSession, FileType, MediaType,
    MetadataItem, MutableAudioMix, MutableAudioMixInputParameters, TrackAssociationType,
    TrackGroupOutputHandling, Writer,
};
use videotoolbox::prelude::*;

const BGRA: u32 = u32::from_be_bytes(*b"BGRA");
const INDEFINITE: CMTime = CMTime {
    value: 0,
    timescale: 0,
    flags: 0x11,
    epoch: 0,
};

fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/test-artifacts")
        .join(format!("preconditions-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

fn writer(name: &str, file_type: FileType, extension: &str) -> (PathBuf, Writer) {
    let dir = scratch(name);
    let writer = Writer::create(dir.join(format!("out.{extension}")), file_type).expect("writer");
    (dir, writer)
}

#[track_caller]
fn expect_invalid_state<T>(result: Result<T, AVWriterError>) {
    match result {
        Err(AVWriterError::InvalidState(_)) => {}
        Err(other) => panic!("expected InvalidState, got {other:?}"),
        Ok(_) => panic!("expected InvalidState, got Ok"),
    }
}

#[track_caller]
fn expect_invalid_argument<T>(result: Result<T, AVWriterError>) {
    match result {
        Err(AVWriterError::InvalidArgument(_)) => {}
        Err(other) => panic!("expected InvalidArgument, got {other:?}"),
        Ok(_) => panic!("expected InvalidArgument, got Ok"),
    }
}

fn encoded_h264_frame() -> EncodedFrame {
    let surface = IOSurface::create(64, 64, BGRA, 4).expect("surface");
    let encoder = CompressionSession::builder(64, 64, Codec::H264)
        .with_real_time(true)
        .build()
        .expect("encoder");
    encoder
        .encode(&surface, CMTime::new(0, 30))
        .expect("encode")
}

fn video_settings() -> PropertyValue {
    PropertyValue::Dictionary(
        [
            (
                "AVVideoCodecKey".into(),
                PropertyValue::String("avc1".into()),
            ),
            ("AVVideoWidthKey".into(), PropertyValue::Integer(64)),
            ("AVVideoHeightKey".into(), PropertyValue::Integer(64)),
        ]
        .into(),
    )
}

#[test]
fn session_lifecycle_is_enforced() {
    let (_dir, writer) = writer("lifecycle", FileType::M4a, "m4a");
    let input = writer
        .add_audio_input_pcm(48_000.0, 1, 16)
        .expect("audio input");
    let silence = [0_u8; 960];

    expect_invalid_state(writer.end_session(CMTime::new(1, 48_000)));
    expect_invalid_state(writer.append_audio_pcm(input, &silence, 480, CMTime::new(0, 48_000)));

    writer.start_session(CMTime::new(0, 48_000)).expect("start");
    expect_invalid_state(writer.start_session(CMTime::new(0, 48_000)));
    writer
        .append_audio_pcm(input, &silence, 480, CMTime::new(0, 48_000))
        .expect("append inside the session");
    writer.end_session(CMTime::new(480, 48_000)).expect("end");

    expect_invalid_state(writer.end_session(CMTime::new(480, 48_000)));
    expect_invalid_state(writer.start_session(CMTime::new(480, 48_000)));
    expect_invalid_state(writer.append_audio_pcm(input, &silence, 480, CMTime::new(480, 48_000)));
    writer.finish().expect("finish after end_session");
}

#[test]
fn start_writing_failures_surface_as_start_failed() {
    let dir = scratch("start-failed");
    let writer =
        Writer::create(dir.join("missing").join("out.m4a"), FileType::M4a).expect("writer");
    writer
        .add_audio_input_pcm(48_000.0, 1, 16)
        .expect("audio input");
    match writer.start_session(CMTime::new(0, 48_000)) {
        Err(AVWriterError::StartFailed(_)) => {}
        other => panic!("expected StartFailed, got {other:?}"),
    }
}

#[test]
fn appended_samples_must_match_the_input_media_type() {
    let frame = encoded_h264_frame();
    let sample = frame.cm_sample_buffer().expect("sample buffer");
    let format = sample.format_description().expect("format description");
    let (_dir, writer) = writer("media-type", FileType::Mov, "mov");

    expect_invalid_argument(writer.add_audio_input_from_sample(sample));
    expect_invalid_argument(writer.add_input(&MediaType::Audio, None, Some(&format), false));
    let video = writer
        .add_video_input_from_sample(sample)
        .expect("video input");
    let audio = writer
        .add_audio_input_pcm(48_000.0, 1, 16)
        .expect("audio input");
    let captions = writer
        .add_caption_input(&MediaType::ClosedCaption, false)
        .expect("caption input");

    writer.start_session(CMTime::new(0, 30)).expect("start");
    expect_invalid_argument(writer.append_sample(audio, sample));
    expect_invalid_argument(writer.append_sample(captions, sample));
    writer
        .append_sample(video, sample)
        .expect("append to the video input");
}

#[test]
fn append_pixel_buffer_requires_a_numeric_time() {
    let (_dir, writer) = writer("pixel-time", FileType::Mov, "mov");
    expect_invalid_argument(writer.add_video_input_pixel_buffer(0, 64, BGRA));
    let input = writer
        .add_video_input_pixel_buffer(64, 64, BGRA)
        .expect("pixel buffer input");
    let buffer = CVPixelBuffer::create(64, 64, BGRA).expect("pixel buffer");

    expect_invalid_state(writer.append_pixel_buffer(input, &buffer, CMTime::new(0, 30)));
    writer.start_session(CMTime::new(0, 30)).expect("start");
    expect_invalid_argument(writer.append_pixel_buffer(input, &buffer, CMTime::new(0, 0)));
    expect_invalid_argument(writer.append_pixel_buffer(input, &buffer, CMTime::new(1, -30)));
    expect_invalid_argument(writer.append_pixel_buffer(input, &buffer, CMTime::INVALID));
    expect_invalid_argument(writer.append_pixel_buffer(input, &buffer, INDEFINITE));
}

#[test]
fn configuration_is_frozen_once_writing_starts() {
    let (dir, writer) = writer("frozen", FileType::Mov, "mov");
    let video = writer
        .add_video_input_pixel_buffer(64, 64, BGRA)
        .expect("video input");
    let audio = writer
        .add_audio_input_pcm(48_000.0, 1, 16)
        .expect("audio input");
    writer.start_session(CMTime::new(0, 30)).expect("start");

    expect_invalid_state(writer.set_optimize_for_network_use(true));
    expect_invalid_state(writer.set_movie_fragment_interval_seconds(1.0));
    expect_invalid_state(writer.set_movie_fragment_interval(CMTime::new(1, 1)));
    expect_invalid_state(writer.set_movie_time_scale(600));
    expect_invalid_state(writer.set_overall_duration_hint(CMTime::new(1, 1)));
    expect_invalid_state(writer.set_metadata(&[MetadataItem::string(
        "mdta/com.apple.quicktime.title",
        "late",
    )]));
    expect_invalid_state(writer.set_directory_for_temporary_files(&dir));

    expect_invalid_state(writer.add_audio_input_pcm(48_000.0, 1, 16));
    expect_invalid_state(writer.add_video_input_pixel_buffer(64, 64, BGRA));
    expect_invalid_state(writer.add_caption_input(&MediaType::ClosedCaption, false));
    assert!(writer.add_input_group(&[audio], None).is_err());

    expect_invalid_state(writer.set_input_language_code(audio, Some("eng")));
    expect_invalid_state(writer.set_input_preferred_volume(audio, 0.5));
    expect_invalid_state(writer.set_input_expects_media_data_in_real_time(audio, false));
    expect_invalid_state(writer.set_input_natural_size(video, 32.0, 32.0));
    expect_invalid_state(writer.set_input_media_time_scale(video, 600));
    expect_invalid_state(writer.set_input_metadata(
        audio,
        &[MetadataItem::string(
            "mdta/com.apple.quicktime.title",
            "late",
        )],
    ));
    expect_invalid_state(writer.attach_pixel_buffer_adaptor(video, None));
    expect_invalid_state(writer.add_track_association(
        video,
        audio,
        &TrackAssociationType::SelectionFollower,
    ));
}

#[test]
fn writer_values_follow_documented_ranges() {
    let (_dir, writer) = writer("writer-values", FileType::Mov, "mov");
    expect_invalid_argument(writer.set_initial_movie_fragment_sequence_number(0));
    expect_invalid_argument(writer.set_overall_duration_hint(CMTime::new(-1, 1)));
    expect_invalid_argument(writer.set_movie_time_scale(-600));
    expect_invalid_argument(writer.set_preferred_output_segment_interval(CMTime::new(0, 1)));
    expect_invalid_argument(writer.set_movie_fragment_interval_seconds(f64::NAN));

    writer
        .set_overall_duration_hint(CMTime::INVALID)
        .expect("an invalid hint clears it");
    writer
        .set_preferred_output_segment_interval(INDEFINITE)
        .expect("indefinite segment interval");
    writer
        .set_movie_fragment_interval_seconds(0.0)
        .expect("zero disables fragments");
    writer
        .set_initial_movie_fragment_sequence_number(1)
        .expect("sequence numbers start at 1");
}

#[test]
fn input_values_are_checked_before_they_reach_the_input() {
    let (_dir, writer) = writer("input-values", FileType::Mov, "mov");
    let video = writer
        .add_video_input_pixel_buffer(64, 64, BGRA)
        .expect("video input");
    let audio = writer
        .add_audio_input_pcm(48_000.0, 1, 16)
        .expect("audio input");

    expect_invalid_argument(writer.set_input_media_time_scale(audio, 48_000));
    expect_invalid_argument(writer.set_input_media_time_scale(video, -1));
    expect_invalid_argument(
        writer.set_input_preferred_media_chunk_duration(video, CMTime::INVALID),
    );
    expect_invalid_argument(
        writer.set_input_preferred_media_chunk_duration(video, CMTime::new(0, 1)),
    );
    expect_invalid_argument(writer.set_input_preferred_media_chunk_alignment(video, -1));
    expect_invalid_argument(writer.set_input_language_code(audio, Some("not a language")));
    expect_invalid_argument(writer.set_input_extended_language_tag(audio, Some("not a tag")));

    writer
        .set_input_language_code(audio, Some("eng"))
        .expect("ISO 639-2/T code");
    writer
        .set_input_extended_language_tag(audio, Some("en-GB"))
        .expect("BCP 47 tag");
    writer
        .set_input_media_time_scale(video, 600)
        .expect("video media time scale");
    writer
        .set_input_preferred_media_chunk_duration(video, CMTime::new(1, 2))
        .expect("positive chunk duration");
    writer
        .set_input_preferred_media_chunk_alignment(video, 512)
        .expect("chunk alignment");
}

#[test]
fn input_groups_are_validated() {
    let (_dir, writer) = writer("groups", FileType::Mov, "mov");
    let english = writer
        .add_audio_input_pcm(48_000.0, 1, 16)
        .expect("english");
    let french = writer.add_audio_input_pcm(48_000.0, 1, 16).expect("french");
    let commentary = writer
        .add_audio_input_pcm(48_000.0, 1, 16)
        .expect("commentary");

    assert!(writer.add_input_group(&[], None).is_err());
    assert!(writer.add_input_group(&[english, english], None).is_err());
    assert!(writer.add_input_group(&[english], Some(french)).is_err());
    writer
        .add_input_group(&[english, french], Some(english))
        .expect("valid group");
    assert!(writer.add_input_group(&[french, commentary], None).is_err());

    expect_invalid_state(writer.set_input_marks_output_track_as_enabled(french, true));
    writer
        .set_input_marks_output_track_as_enabled(commentary, true)
        .expect("an ungrouped input");
}

#[test]
fn input_groups_need_a_file_type_that_supports_them() {
    let (_dir, writer) = writer("groups-caf", FileType::Caf, "caf");
    let first = writer.add_audio_input_pcm(48_000.0, 1, 16).expect("first");
    assert!(writer.add_input_group(&[first], Some(first)).is_err());
}

#[test]
fn adaptors_attach_once_to_matching_inputs() {
    let (_dir, writer) = writer("adaptors", FileType::Mov, "mov");
    let audio = writer
        .add_audio_input_pcm(48_000.0, 1, 16)
        .expect("audio input");
    let video = writer
        .add_input(&MediaType::Video, Some(&video_settings()), None, false)
        .expect("video input");
    let captions = writer
        .add_caption_input(&MediaType::ClosedCaption, false)
        .expect("caption input");
    expect_invalid_argument(writer.add_caption_input(&MediaType::Subtitle, false));
    let subtitles = writer
        .add_input(&MediaType::Subtitle, None, None, false)
        .expect("subtitle input");

    expect_invalid_argument(writer.attach_caption_adaptor(subtitles));
    expect_invalid_argument(writer.attach_pixel_buffer_adaptor(audio, None));
    expect_invalid_argument(writer.attach_metadata_adaptor(audio));
    expect_invalid_argument(writer.attach_metadata_adaptor(video));
    writer
        .attach_pixel_buffer_adaptor(
            video,
            Some(&PropertyValue::Dictionary(
                [
                    (
                        "PixelFormatType".into(),
                        PropertyValue::Unsigned(u64::from(BGRA)),
                    ),
                    ("Width".into(), PropertyValue::Integer(64)),
                    ("Height".into(), PropertyValue::Integer(64)),
                ]
                .into(),
            )),
        )
        .expect("first adaptor");
    expect_invalid_state(writer.attach_pixel_buffer_adaptor(video, None));
    expect_invalid_state(writer.attach_caption_adaptor(video));
    expect_invalid_state(writer.attach_caption_adaptor(captions));

    writer.start_session(CMTime::new(0, 30)).expect("start");
    assert!(writer.pixel_buffer_pool(video).expect("pool").is_some());
}

#[test]
fn caption_objects_are_validated() {
    let range = CMTimeRange::new(CMTime::new(0, 30), CMTime::new(30, 30));
    let renderer = CaptionRenderer::new().expect("renderer");

    let mixed_units = CaptionRegion::new(
        None,
        CaptionPoint::default(),
        CaptionSize::new(
            CaptionDimension::new(10.0, CaptionUnitsType::Cells),
            CaptionDimension::new(10.0, CaptionUnitsType::Percent),
        ),
        CaptionRegionScroll::None,
        CaptionRegionDisplayAlignment::Before,
        CaptionRegionWritingMode::LeftToRightAndTopToBottom,
    );
    expect_invalid_argument(
        renderer.set_captions(&[Caption::new("mixed", range).with_region(mixed_units)]),
    );
    expect_invalid_argument(
        renderer.set_captions(&[Caption::new("ab", range).with_ruby(1..5, CaptionRuby::new("x"))]),
    );
    renderer
        .set_captions(&[Caption::new("ab", range).with_ruby(0..2, CaptionRuby::new("x"))])
        .expect("ruby inside the text");

    let settings = CaptionConversionSettings::new("tx3g");
    expect_invalid_argument(CaptionFormatConformer::new(&settings));
    expect_invalid_argument(CaptionConversionValidator::new(&[], range, &settings));
    CaptionFormatConformer::new(&CaptionConversionSettings::cea608()).expect("CEA-608 conformer");
}

#[test]
fn caption_groups_must_contain_their_captions() {
    let (_dir, writer) = writer("caption-groups", FileType::Mov, "mov");
    let captions = writer
        .add_caption_input(&MediaType::ClosedCaption, false)
        .expect("caption input");
    writer.start_session(CMTime::new(0, 30)).expect("start");

    let range = CMTimeRange::new(CMTime::new(0, 30), CMTime::new(30, 30));
    let outside = Caption::new(
        "late",
        CMTimeRange::new(CMTime::new(60, 30), CMTime::new(30, 30)),
    );
    expect_invalid_argument(
        writer.append_caption_group(captions, &CaptionGroup::new(vec![outside], range)),
    );
}

#[test]
fn audio_mix_volume_times_must_be_numeric() {
    let parameters = MutableAudioMixInputParameters::new().expect("parameters");
    expect_invalid_argument(parameters.set_volume(0.5, CMTime::INVALID));
    expect_invalid_argument(parameters.set_volume_ramp(
        0.0,
        1.0,
        CMTimeRange::new(CMTime::INVALID, CMTime::new(1, 1)),
    ));
    parameters
        .set_volume(0.5, CMTime::new(0, 1))
        .expect("numeric volume time");
    parameters
        .set_volume_ramp(
            0.0,
            1.0,
            CMTimeRange::new(CMTime::new(0, 1), CMTime::new(1, 1)),
        )
        .expect("numeric ramp");
}

#[test]
fn export_audio_mix_excludes_alternate_track_handling() {
    let dir = scratch("export-mix");
    let source = dir.join("source.m4a");
    {
        let writer = Writer::create(&source, FileType::M4a).expect("source writer");
        let input = writer
            .add_audio_input_pcm(48_000.0, 1, 16)
            .expect("audio input");
        writer.start_session(CMTime::new(0, 48_000)).expect("start");
        writer
            .append_audio_pcm(input, &[0_u8; 9_600], 4_800, CMTime::new(0, 48_000))
            .expect("append");
        writer.finish().expect("finish");
    }
    let session = ExportSession::new(&source, ExportPreset::AppleM4A).expect("export session");
    let mix = MutableAudioMix::new().expect("audio mix");

    session
        .set_audio_track_group_handling(TrackGroupOutputHandling::PreserveAlternateTracks)
        .expect("preserve alternate tracks");
    expect_invalid_state(session.set_audio_mix(Some(&mix)));
    session
        .set_audio_track_group_handling(TrackGroupOutputHandling::None)
        .expect("reset handling");
    session.set_audio_mix(Some(&mix)).expect("audio mix");
    expect_invalid_state(
        session.set_audio_track_group_handling(TrackGroupOutputHandling::PreserveAlternateTracks),
    );
}

#[test]
fn flush_segment_requires_a_segment_delegate() {
    let (_dir, writer) = writer("flush", FileType::Mp4, "mp4");
    writer
        .add_video_input_pixel_buffer(64, 64, BGRA)
        .expect("video input");
    writer
        .set_preferred_output_segment_interval(INDEFINITE)
        .expect("indefinite interval");
    if writer.start_session(CMTime::new(0, 30)).is_ok() {
        expect_invalid_state(writer.flush_segment());
    }
}
