//! Tests for codec parameter conversions.

use super::*;

/// `dOps` is big-endian; the `OpusHead` it becomes is little-endian, with the mapping table
/// only for families other than 0.
#[test]
fn dops_becomes_an_opus_head() {
    let o = OpusConfig {
        output_channels: 6,
        pre_skip: 312,
        input_sample_rate: 48_000,
        output_gain: -256,
        channel_mapping_family: 1,
        channel_mapping: vec![4, 2, 0, 4, 1, 2, 3, 5],
    };
    let h = o.opus_head();
    assert_eq!(&h[..8], b"OpusHead");
    assert_eq!(&h[8..19], &[1, 6, 0x38, 0x01, 0x80, 0xBB, 0, 0, 0x00, 0xFF, 1]);
    assert_eq!(&h[19..], &o.channel_mapping[..]);
    let stereo = OpusConfig { output_channels: 2, channel_mapping_family: 0, channel_mapping: vec![9, 9], ..o };
    assert_eq!(stereo.opus_head().len(), 19);
    assert_eq!(CodecConfig::Opus(stereo).codec_id(), CodecId::Opus);
}

#[test]
fn apv_extradata_is_the_bare_record() {
    let cfg = ApvConfig {
        profile_idc: 33,
        level_idc: 60,
        frame_width: 1920,
        frame_height: 1080,
        chroma_format_idc: 2,
        bit_depth_minus8: 2,
        ..Default::default()
    };
    let record = cfg.record_bytes();
    assert_eq!(record[0], 1, "configurationVersion");
    assert_eq!(cfg.to_bytes(), [&[0u8; 4][..], &record].concat());
    // Matroska's V_APV CodecPrivate carries the record as is; an MP4 track must match it
    let parsed = ApvConfig::parse(&cfg.to_bytes()).unwrap();
    assert_eq!(parsed.raw_record, record);
    assert_eq!(CodecConfig::Apv(parsed.clone()).extradata(), record);
    assert_eq!(parsed.to_bytes(), cfg.to_bytes());
    assert_eq!(CodecConfig::Apv(cfg).extradata(), record);
}
