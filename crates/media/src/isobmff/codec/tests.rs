//! Round-trip tests for sample entry parsing and serialization.

use super::*;

fn roundtrip(e: &SampleEntry, qt: bool, handler: &[u8; 4]) -> SampleEntry {
    let mut b = BoxBuf::new();
    e.write(&mut b, qt).unwrap();
    let bx = crate::isobmff::bytes::boxes(&b.buf).next().unwrap().unwrap();
    parse_sample_entry(bx.kind, bx.payload, FourCc(*handler), qt).unwrap()
}

#[test]
fn avcc_roundtrip() {
    let cfg = AvcConfig::new(vec![vec![0x67, 0x64, 0x00, 0x1F, 0xAC]], vec![vec![0x68, 0xEB]], 4);
    assert_eq!(cfg.profile, 0x64);
    assert_eq!(AvcConfig::parse(&cfg.to_bytes()).unwrap(), cfg);
    let mut e = SampleEntry::avc(cfg, 1920, 1080);
    e.video.as_mut().unwrap().color = Some(ColorInfo::Nclx { primaries: 1, transfer: 1, matrix: 1, full_range: false });
    e.video.as_mut().unwrap().pixel_aspect = Some((4, 3));
    e.video.as_mut().unwrap().field_info = Some(FieldInfo { fields: 2, detail: 9 });
    e.video.as_mut().unwrap().clean_aperture =
        Some(CleanAperture { width: (1920, 1), height: (1080, 1), horiz_offset: (-2, 1), vert_offset: (0, 1) });
    e.video.as_mut().unwrap().mastering_display = Some(MasteringDisplay::bt2020(1000.0, 0.0001));
    e.video.as_mut().unwrap().content_light = Some((1000, 400));
    e.bitrate = Some(BitRate { buffer_size: 1, max_bitrate: 2, avg_bitrate: 3 });
    assert_eq!(roundtrip(&e, false, b"vide"), e);
    assert_eq!(MasteringDisplay::bt2020(1000.0, 0.0001).max_nits(), 1000.0);
}

#[test]
fn hvcc_roundtrip() {
    let cfg = HevcConfig {
        general_profile_idc: 1,
        general_profile_compatibility_flags: 0x6000_0000,
        general_constraint_indicator_flags: 0x9000_0000_0000,
        general_level_idc: 93,
        chroma_format_idc: 1,
        bit_depth_luma: 8,
        bit_depth_chroma: 8,
        length_size: 4,
        num_temporal_layers: 1,
        temporal_id_nested: true,
        arrays: vec![
            HevcNalArray { completeness: true, nal_type: 32, nalus: vec![vec![0x40, 1]] },
            HevcNalArray { completeness: true, nal_type: 33, nalus: vec![vec![0x42, 1, 2]] },
            HevcNalArray { completeness: true, nal_type: 34, nalus: vec![vec![0x44, 1]] },
        ],
        ..Default::default()
    };
    let back = HevcConfig::parse(&cfg.to_bytes()).unwrap();
    assert_eq!(back, cfg);
    assert_eq!(back.sps(), vec![&[0x42u8, 1, 2][..]]);
}

#[test]
fn av1_vp9_opus_roundtrip() {
    let a = Av1Config { seq_profile: 1, seq_level_idx_0: 8, high_bitdepth: true, config_obus: vec![1, 2], ..Default::default() };
    assert_eq!(Av1Config::parse(&a.to_bytes()).unwrap(), a);
    let v = VpcConfig {
        profile: 2,
        level: 31,
        bit_depth: 10,
        chroma_subsampling: 1,
        full_range: true,
        colour_primaries: 9,
        transfer_characteristics: 16,
        matrix_coefficients: 9,
        codec_init: vec![],
    };
    assert_eq!(VpcConfig::parse(&v.to_bytes()).unwrap(), v);
    let o = OpusConfig {
        output_channels: 2,
        pre_skip: 312,
        input_sample_rate: 48000,
        output_gain: 0,
        channel_mapping_family: 0,
        channel_mapping: vec![],
    };
    assert_eq!(OpusConfig::parse(&o.to_bytes()).unwrap(), o);
}

#[test]
fn aac_channel_configuration_is_an_index() {
    // 48 kHz AAC LC: configuration → channels (Table 1.19 with Amd 4; #922)
    let channels = |c: u16| AacConfig::from_asc(((2 << 11) | (3 << 7) | (c << 3)).to_be_bytes().to_vec()).channels();
    assert_eq!([0, 1, 6, 7, 8, 11, 12, 13, 14, 15].map(channels), [0, 1, 6, 8, 0, 7, 8, 24, 8, 0]);
}

#[test]
fn aac_esds_roundtrip() {
    let e = SampleEntry::aac(vec![0x12, 0x10], 2, 44100);
    match &e.codec {
        CodecConfig::Aac(a) => {
            assert_eq!((a.object_type, a.sample_rate, a.channel_config), (2, 44100, 2));
        }
        _ => panic!(),
    }
    assert_eq!(roundtrip(&e, false, b"soun"), e);
    assert_eq!(roundtrip(&e, true, b"soun"), e);
}

/// QuickTime's `.mp2` entry (what ffmpeg writes for MP2 in MOV) is MPEG audio, as `.mp3` is,
/// even under a handler that does not say "sound" (#800).
#[test]
fn quicktime_mpeg_audio_entries() {
    for fourcc in [*b".mp3", *b".mp2"] {
        let e = SampleEntry::audio(FourCc(fourcc), CodecConfig::Mp3, 2, 48000.0, 16);
        for handler in [b"soun", b"\0\0\0\0"] {
            let back = roundtrip(&e, true, handler);
            assert_eq!(back.codec, CodecConfig::Mp3, "{fourcc:?} {handler:?}");
            assert_eq!(back.audio.map(|a| (a.channels, a.sample_rate)), Some((2, 48000.0)), "{fourcc:?} {handler:?}");
        }
    }
}

fn qt_v2_lpcm(channels: u32, bits: u32) -> Vec<u8> {
    let mut p = vec![0; 6];
    let u16b = |p: &mut Vec<u8>, v: u16| p.extend(v.to_be_bytes());
    let u32b = |p: &mut Vec<u8>, v: u32| p.extend(v.to_be_bytes());
    u16b(&mut p, 1);
    u16b(&mut p, 2);
    u16b(&mut p, 0);
    u32b(&mut p, 0);
    u16b(&mut p, 2);
    u16b(&mut p, 16);
    u16b(&mut p, 0);
    u16b(&mut p, 0);
    u32b(&mut p, 65536);
    u32b(&mut p, 72);
    p.extend(48000.0f64.to_be_bytes());
    u32b(&mut p, channels);
    u32b(&mut p, 0);
    u32b(&mut p, bits);
    u32b(&mut p, 0);
    u32b(&mut p, 0);
    u32b(&mut p, 1);
    p
}

#[test]
fn pcm_frame_size_saturates_on_a_hostile_channel_count() {
    assert_eq!(PcmConfig { bits: 16, channels: 2, sample_rate: 48000.0, ..Default::default() }.bytes_per_frame(), 4);
    assert_eq!(PcmConfig { bits: 24, channels: 0, ..Default::default() }.bytes_per_frame(), 3);
    assert_eq!(PcmConfig { bits: 16, channels: u32::MAX, ..Default::default() }.bytes_per_frame(), u32::MAX);
    let ok = parse_sample_entry(FourCc(*b"lpcm"), &qt_v2_lpcm(2, 16), FourCc(*b"soun"), true).unwrap();
    assert!(matches!(ok.codec, CodecConfig::Pcm(p) if p.bytes_per_frame() == 4));
}

#[test]
fn qt_v2_pcm_rejects_channels_and_bits_past_16_bits() {
    let entry = |channels, bits| parse_sample_entry(FourCc(*b"lpcm"), &qt_v2_lpcm(channels, bits), FourCc(*b"soun"), true);
    assert!(matches!(entry(u32::MAX, 16), Err(Error::Invalid(_))));
    assert!(matches!(entry(65_536, 16), Err(Error::Invalid(_))));
    assert!(entry(65_535, 16).is_ok());
    // 0x0001_0010 would read as 16-bit PCM, and 0x0001_0000 would fall back to the sample size.
    assert!(matches!(entry(2, 0x0001_0010), Err(Error::Invalid(_))));
    assert!(matches!(entry(2, 0x0001_0000), Err(Error::Invalid(_))));
    let ok = entry(2, 24).unwrap();
    assert!(matches!(ok.codec, CodecConfig::Pcm(p) if p.bits == 24));
}

#[test]
fn pcm_entries() {
    for (bits, float, be, qt) in [
        (16, false, false, true),
        (16, false, true, true),
        (24, false, false, true),
        (32, true, false, true),
        (24, false, true, false),
        (32, true, false, false),
    ] {
        let p = PcmConfig { bits, float, big_endian: be, signed: true, channels: 2, sample_rate: 48000.0 };
        let e = SampleEntry::pcm(p);
        let back = roundtrip(&e, qt, b"soun");
        assert_eq!(back.codec, CodecConfig::Pcm(p), "{bits} {float} {be} {qt}");
    }
}

#[test]
fn mov_pcm_keeps_high_rates_and_signedness() {
    let pcm = |bits, signed, sample_rate| PcmConfig { bits, float: false, big_endian: true, signed, channels: 2, sample_rate };
    for (p, format) in [
        (pcm(16, true, 48000.0), b"twos"),
        (pcm(16, true, 96000.0), b"lpcm"),
        (pcm(16, false, 48000.0), b"lpcm"),
        (pcm(8, false, 48000.0), b"raw "),
        (pcm(8, true, 48000.0), b"twos"),
        (pcm(8, false, 192000.0), b"lpcm"),
    ] {
        let mut b = BoxBuf::new();
        SampleEntry::pcm(p).write(&mut b, true).unwrap();
        let bx = crate::isobmff::bytes::boxes(&b.buf).next().unwrap().unwrap();
        assert_eq!(&bx.kind.0, format, "{p:?}");
        let back = roundtrip(&SampleEntry::pcm(p), true, b"soun");
        assert_eq!(back.codec, CodecConfig::Pcm(p));
        assert_eq!(back.audio.unwrap().sample_rate, p.sample_rate);
    }
}

#[test]
fn timecode_and_prores() {
    let tc = TimecodeConfig { flags: 1, timescale: 30000, frame_duration: 1001, frames_per_second: 30, start_frame: None, name: Some("reel".into()) };
    let e = SampleEntry {
        format: FourCc(*b"tmcd"),
        data_reference_index: 1,
        codec: CodecConfig::Timecode(tc.clone()),
        video: None,
        audio: None,
        bitrate: None,
    };
    assert_eq!(roundtrip(&e, true, b"tmcd"), e);
    assert_eq!(tc.format_frame(107892), "01:00:00;00");
    let nd = TimecodeConfig { flags: 0, frames_per_second: 24, ..Default::default() };
    assert_eq!(nd.format_frame(86400), "01:00:00:00");
    let p = SampleEntry::prores(FourCc(*b"apch"), 1920, 1080);
    assert_eq!(roundtrip(&p, true, b"vide"), p);
}

#[test]
fn asc_parse() {
    assert_eq!(parse_asc(&[0x11, 0x90]), Some((2, 48000, 2)));
    assert_eq!(parse_asc(&[]), None);
}
