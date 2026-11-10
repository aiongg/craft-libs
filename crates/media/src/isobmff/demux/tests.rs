//! Demuxer tests.

use super::metadata::lossy_len;
use super::stbl::SttsCursor;
use super::*;

#[test]
fn language_codes() {
    // "und" = 0x55C4
    assert_eq!(parse_language(0x55C4), "und");
    assert_eq!(parse_language(0), "eng");
    assert_eq!(parse_language(0x15C7), "eng");
}

#[test]
fn stts_cursor() {
    let runs = [(2, 10), (0, 99), (3, 5)];
    let mut c = SttsCursor { runs: &runs, run: 0, used: 0 };
    assert_eq!(c.next_value(), 10);
    assert_eq!(c.take(3), 10 + 5 + 5);
    assert_eq!(c.next_value(), 5);
    assert_eq!(c.next_value(), 0);
}

#[test]
fn codec_params_follow_the_sample_description() {
    use crate::isobmff::codec::SampleEntry;
    use crate::isobmff::mux::{Brand, Mp4Writer, TrackConfig, WriteSample, WriterOptions};
    let mut w = Mp4Writer::new(std::io::Cursor::new(Vec::new()), WriterOptions::new(Brand::Mp4)).unwrap();
    let a = w.add_track(TrackConfig::new(SampleEntry::aac(vec![0x12, 0x10], 2, 48_000), 48_000)).unwrap();
    for _ in 0..2 {
        w.write_sample(a, WriteSample { data: &[0; 8], duration: 1024, composition_offset: 0, is_sync: true }).unwrap();
    }
    let out = w.finish().unwrap().into_inner();
    let mut t = open(out.as_slice()).unwrap().tracks.remove(0);
    assert!(!t.has_mixed_descriptions());
    assert_eq!(t.codec_params(), t.codec_params_for(1));
    // entry 1 unused, every sample on entry 2 (a 44.1 kHz mono AudioSpecificConfig)
    t.entries.push(SampleEntry::aac(vec![0x12, 0x08], 1, 44_100));
    t.samples.iter_mut().for_each(|s| s.description_index = 2);
    let p = t.codec_params();
    assert_eq!((p.extradata.as_slice(), p.sample_rate, p.channels), (&[0x12, 0x08][..], Some(44_100), Some(1)));
    assert_eq!(t.codec_params_for(1).extradata, [0x12, 0x10]);
    assert_eq!(t.codec_params_for(0).codec, crate::CodecId::Unknown);
    assert_eq!(t.codec_params_for(3).codec, crate::CodecId::Unknown);
    assert!(!t.has_mixed_descriptions());
    t.samples[1].description_index = 1;
    assert!(t.has_mixed_descriptions());
    t.samples[0].description_index = 0; // malformed: falls back to the first entry
    assert_eq!(t.codec_params().extradata, [0x12, 0x10]);
}

#[test]
fn mpeg_audio_layer_comes_from_the_first_frame() {
    use crate::CodecId;
    use crate::isobmff::codec::SampleEntry;
    use crate::isobmff::mux::{Brand, Mp4Writer, TrackConfig, WriteSample, WriterOptions};
    // MPEG-1 frame headers: layer I, II, III; then no frame header at all
    let cases: [(&[u8; 4], Brand, [u8; 2], CodecId); 5] = [
        (b"mp4a", Brand::Mp4, [0xFF, 0xFF], CodecId::Mp1),
        (b"mp4a", Brand::Mp4, [0xFF, 0xFD], CodecId::Mp2),
        (b"mp4a", Brand::Mp4, [0xFF, 0xFB], CodecId::Mp3),
        (b"mp4a", Brand::Mp4, [0, 0], CodecId::Mp3),
        (b".mp2", Brand::Mov, [0, 0], CodecId::Mp2),
    ];
    for (format, brand, header, want) in cases {
        let mut w = Mp4Writer::new(std::io::Cursor::new(Vec::new()), WriterOptions::new(brand)).unwrap();
        let entry = SampleEntry::audio(FourCc(*format), CodecConfig::Mp3, 2, 48_000.0, 16);
        let a = w.add_track(TrackConfig::new(entry, 48_000)).unwrap();
        let frame = [&header[..], &[0x94, 0x00, 0, 0, 0, 0]].concat();
        w.write_sample(a, WriteSample { data: &frame, duration: 1152, composition_offset: 0, is_sync: true }).unwrap();
        let out = w.finish().unwrap().into_inner();
        let t = &open(out.as_slice()).unwrap().tracks[0];
        assert_eq!(t.codec(), Some(&CodecConfig::Mp3));
        assert_eq!(t.codec_params().codec, want, "{format:?} {header:02X?}");
    }
}

#[test]
fn rejects_no_moov_and_cmov() {
    let mut b = crate::isobmff::bytes::BoxBuf::new();
    b.leaf(b"free", &[0; 4]);
    assert!(matches!(open(b.buf.clone()), Err(Error::NoMoov)));
    let m = b.start(b"moov");
    b.leaf(b"cmov", &[]);
    b.end(m);
    assert!(matches!(open(b.buf), Err(Error::Unsupported(_))));
}

#[test]
fn metadata_qt_and_ilst() {
    let mut b = crate::isobmff::bytes::BoxBuf::new();
    let m = b.start(&[0xA9, b'n', b'a', b'm']);
    b.u16(2);
    b.u16(0);
    b.bytes(b"hi");
    b.end(m);
    let meta = b.start_full(b"meta", 0, 0);
    b.leaf(b"hdlr", &[0; 24]);
    let il = b.start(b"ilst");
    let it = b.start(&[0xA9, b'A', b'R', b'T']);
    let d = b.start(b"data");
    b.u32(1);
    b.u32(0);
    b.bytes(b"me");
    b.end(d);
    b.end(it);
    b.end(il);
    b.end(meta);
    let mut md = Metadata::default();
    parse_udta(&b.buf, &mut md, &mut { MAX_METADATA_BYTES });
    assert_eq!(md.title(), Some("hi"));
    assert_eq!(md.get("©ART"), Some("me"));
}

use crate::isobmff::bytes::BoxBuf;

fn full(b: &mut BoxBuf, kind: &[u8; 4], v: u8, f: impl FnOnce(&mut BoxBuf)) {
    let m = b.start_full(kind, v, 0);
    f(b);
    b.end(m);
}

/// Hand-built file: uuid/wide/skip boxes, stz2 sizes, co64, ctts v1 with negative offsets,
/// stss, sdtp, sbgp/sgpd, and an edit list with a leading empty edit.
fn handmade(stsz_override: Option<(u32, u32)>) -> Vec<u8> {
    let mut b = BoxBuf::new();
    let m = b.start(b"ftyp");
    b.bytes(b"isom");
    b.u32(0);
    b.bytes(b"isom");
    b.end(m);
    let m = b.start(b"uuid");
    b.bytes(&[0xAB; 16]);
    b.bytes(b"payload");
    b.end(m);
    b.leaf(b"wide", &[]);
    b.leaf(b"skip", &[1, 2, 3]);
    // mdat with 64-bit size: 4 samples of 3,1,2,4 bytes.
    let data_start = b.len() as u64 + 16;
    b.u32(1);
    b.bytes(b"mdat");
    b.u64(16 + 10);
    b.bytes(&[1, 1, 1, 2, 3, 3, 4, 4, 4, 4]);
    let moov = b.start(b"moov");
    full(&mut b, b"mvhd", 0, |b| {
        b.u32(0);
        b.u32(0);
        b.u32(1000);
        b.u32(500);
        b.zeros(80);
    });
    let trak = b.start(b"trak");
    full(&mut b, b"tkhd", 0, |b| {
        b.u32(0);
        b.u32(0);
        b.u32(7);
        b.u32(0);
        b.u32(500);
        b.zeros(52);
        b.u32(320 << 16);
        b.u32(240 << 16);
    });
    let edts = b.start(b"edts");
    full(&mut b, b"elst", 1, |b| {
        b.u32(2);
        b.u64(100);
        b.i64(-1);
        b.i32(0x10000);
        b.u64(400);
        b.i64(20);
        b.i32(0x10000);
    });
    b.end(edts);
    let mdia = b.start(b"mdia");
    full(&mut b, b"mdhd", 0, |b| {
        b.u32(0);
        b.u32(0);
        b.u32(100);
        b.u32(40);
        b.u16(0x15C7);
        b.u16(0);
    });
    full(&mut b, b"hdlr", 0, |b| {
        b.u32(0);
        b.bytes(b"vide");
        b.zeros(12);
        b.bytes(b"Video\0");
    });
    let minf = b.start(b"minf");
    let stbl = b.start(b"stbl");
    full(&mut b, b"stsd", 0, |b| {
        b.u32(1);
        crate::isobmff::codec::SampleEntry::jpeg(320, 240).write(b, false).unwrap();
    });
    full(&mut b, b"stts", 0, |b| {
        b.u32(1);
        b.u32(4);
        b.u32(10);
    });
    full(&mut b, b"ctts", 1, |b| {
        b.u32(2);
        b.u32(2);
        b.i32(-5);
        b.u32(2);
        b.i32(5);
    });
    full(&mut b, b"stss", 0, |b| {
        b.u32(2);
        b.u32(3);
        b.u32(1);
    });
    full(&mut b, b"stsc", 0, |b| {
        b.u32(2);
        b.u32(1);
        b.u32(3);
        b.u32(1);
        b.u32(2);
        b.u32(1);
        b.u32(1);
    });
    match stsz_override {
        Some((size, count)) => full(&mut b, b"stsz", 0, |b| {
            b.u32(size);
            b.u32(count);
        }),
        None => full(&mut b, b"stz2", 0, |b| {
            b.u32(4);
            b.u32(4);
            b.u8(0x31);
            b.u8(0x24);
        }),
    }
    full(&mut b, b"co64", 0, |b| {
        b.u32(2);
        b.u64(data_start);
        b.u64(data_start + 6);
    });
    full(&mut b, b"sdtp", 0, |b| b.bytes(&[0x20, 0x10, 0x10, 0x10]));
    full(&mut b, b"sbgp", 0, |b| {
        b.bytes(b"roll");
        b.u32(1);
        b.u32(4);
        b.u32(1);
    });
    full(&mut b, b"sgpd", 1, |b| {
        b.bytes(b"roll");
        b.u32(2);
        b.u32(1);
        b.i16(-1);
    });
    b.end(stbl);
    b.end(minf);
    b.end(mdia);
    b.end(trak);
    b.end(moov);
    // Trailing box extending to EOF (size 0).
    b.u32(0);
    b.bytes(b"free");
    b.bytes(&[9; 5]);
    b.buf
}

#[test]
fn handmade_sample_table() {
    let d = handmade(None);
    let f = open(d.as_slice()).unwrap();
    assert_eq!(f.major_brand, *b"isom");
    let t = &f.tracks[0];
    assert_eq!((t.id, t.kind, t.timescale, t.language.as_str()), (7, TrackKind::Video, 100, "eng"));
    assert_eq!((t.width, t.height), (320, 240));
    assert_eq!(t.handler_name, "Video");
    let sizes: Vec<u32> = t.samples.iter().map(|s| s.size).collect();
    assert_eq!(sizes, [3, 1, 2, 4]);
    let offs: Vec<u64> = t.samples.iter().map(|s| s.offset - t.samples[0].offset).collect();
    assert_eq!(offs, [0, 3, 4, 6]);
    let pts: Vec<i64> = t.samples.iter().map(|s| s.pts).collect();
    assert_eq!(pts, [-5, 5, 25, 35]);
    let sync: Vec<bool> = t.samples.iter().map(|s| s.is_sync).collect();
    assert_eq!(sync, [true, false, true, false]);
    for (i, want) in [vec![1u8, 1, 1], vec![2], vec![3, 3], vec![4; 4]].iter().enumerate() {
        assert_eq!(&f.read_sample(d.as_slice(), 0, i).unwrap(), want);
    }
    // Empty edit of 100 movie ticks = 10 media ticks, then media starts at 20.
    assert_eq!(t.edits.len(), 2);
    assert_eq!(t.edit_offset, 10 - 20);
    assert_eq!(t.presentation_pts(2), Some(15));
    assert_eq!(t.sdtp.len(), 4);
    assert_eq!(t.sample_groups[0].grouping_type, *b"roll");
    assert_eq!(t.sample_groups[0].descriptions, vec![vec![0xFF, 0xFF]]);
    assert!(f.read_sample(d.as_slice(), 0, 4).is_err());
    assert!(f.read_sample(d.as_slice(), 1, 0).is_err());
}

#[test]
fn constant_size_samples_share_one_file_budget() {
    // One chunk of u32::MAX constant-size samples makes a track build one sample per file
    // byte. Repeating the track must not repeat that allocation.
    let mut d = handmade(Some((1, u32::MAX)));
    let p = d.windows(4).position(|w| w == b"stsc").unwrap();
    d[p + 16..p + 20].copy_from_slice(&u32::MAX.to_be_bytes());
    let at = d.windows(4).position(|w| w == b"trak").unwrap() - 4;
    let size = u32::from_be_bytes(d[at..at + 4].try_into().unwrap()) as usize;
    let trak = d[at..at + size].to_vec();
    for _ in 0..3 {
        d.splice(at..at, trak.iter().copied());
    }
    let moov = d.windows(4).position(|w| w == b"moov").unwrap() - 4;
    let moov_size = u32::from_be_bytes(d[moov..moov + 4].try_into().unwrap()) + 3 * size as u32;
    d[moov..moov + 4].copy_from_slice(&moov_size.to_be_bytes());
    let f = open(d.as_slice()).unwrap();
    assert_eq!(f.tracks.len(), 4);
    assert!(f.tracks[0].samples.len() > d.len() / 2);
    assert!(f.tracks.iter().map(|t| t.samples.len()).sum::<usize>() <= d.len());
}

#[test]
fn oversized_trailing_box_ends_the_scan() {
    let mut d = handmade(None);
    d.truncate(d.len() - 13);
    d.extend_from_slice(&1u32.to_be_bytes());
    d.extend_from_slice(b"free");
    d.extend_from_slice(&u64::MAX.to_be_bytes());
    d.extend_from_slice(&[0; 16]);
    assert_eq!(open(d.as_slice()).unwrap().tracks[0].samples.len(), 4);
}

#[test]
fn one_long_key_named_by_many_items_stays_within_the_metadata_budget() {
    let key = vec![b'k'; 64 * 1024];
    let mut b = BoxBuf::new();
    b.leaf(b"hdlr", &[0; 24]);
    let keys = b.start_full(b"keys", 0, 0);
    b.u32(1);
    b.u32(8 + key.len() as u32);
    b.bytes(b"mdta");
    b.bytes(&key);
    b.end(keys);
    let il = b.start(b"ilst");
    for _ in 0..1000 {
        let it = b.start(&1u32.to_be_bytes());
        let d = b.start(b"data");
        b.u32(1);
        b.u32(0);
        b.bytes(b"v");
        b.end(d);
        b.end(it);
    }
    b.end(il);
    let mut payload = vec![0; 4];
    payload.extend_from_slice(&b.buf);
    let mut md = Metadata::default();
    parse_meta(&payload, &mut md, &mut { MAX_METADATA_BYTES });
    assert!(!md.entries.is_empty() && md.entries.len() < 1000, "{}", md.entries.len());
    assert!(md.entries.iter().map(|(k, v)| k.len() + v.len()).sum::<usize>() <= MAX_METADATA_BYTES);
    assert_eq!(md.entries[0].0.len(), key.len());
}

#[test]
fn metadata_is_charged_at_its_stored_length() {
    assert_eq!(lossy_len(b"plain"), 5);
    for bytes in [&b"a\xFFb"[..], b"\xFF\xFE\xFD", b"\xE2\x82", "\u{a9}nam".as_bytes()] {
        assert_eq!(lossy_len(bytes), String::from_utf8_lossy(bytes).len(), "{bytes:?}");
    }
    // Invalid UTF-8 triples in size when converted; the budget sees the converted size.
    let mut b = BoxBuf::new();
    let m = b.start(&[0xA9, b'n', b'a', b'm']);
    b.u16(1000);
    b.u16(0);
    b.bytes(&[0xFF; 1000]);
    b.end(m);
    let mut md = Metadata::default();
    let mut left = 3000 + METADATA_ENTRY_COST + 5;
    parse_udta(&b.buf, &mut md, &mut left);
    assert_eq!(md.entries.len(), 1);
    assert_eq!(md.entries[0].0.len() + md.entries[0].1.len(), 3000 + 5 - left);
}

#[test]
fn a_declared_sample_count_past_the_chunk_map_reserves_only_what_it_maps() {
    // stsz claims u32::MAX samples; the two chunks place four.
    let d = handmade(Some((1, u32::MAX)));
    let f = open(d.as_slice()).unwrap();
    assert_eq!(f.tracks[0].samples.len(), 4);
    assert!(f.tracks[0].samples.capacity() < 64, "{}", f.tracks[0].samples.capacity());
}

#[test]
fn absurd_counts_are_bounded() {
    // Constant-size stsz claiming 4 billion samples is clamped to what fits in the file.
    let d = handmade(Some((1, u32::MAX)));
    let f = open(d.as_slice()).unwrap();
    assert!(f.tracks[0].samples.len() <= d.len());
    // A table claiming more entries than its box holds fails cleanly.
    let mut d = handmade(None);
    let p = d.windows(4).position(|w| w == b"stts").unwrap();
    d[p + 8..p + 12].copy_from_slice(&u32::MAX.to_be_bytes());
    assert!(matches!(open(d.as_slice()), Err(Error::Truncated(_))));
    // Box larger than its parent.
    let mut d = handmade(None);
    let p = d.windows(4).position(|w| w == b"stsc").unwrap();
    d[p - 4..p].copy_from_slice(&0x7FFF_FFFFu32.to_be_bytes());
    assert!(open(d.as_slice()).is_err());
}
/// `data` followed by zeros up to `len` bytes.
struct Padded(Vec<u8>, u64);

impl ByteSource for Padded {
    fn len(&self) -> u64 {
        self.1
    }
    fn read_at(&self, offset: u64, buf: &mut [u8]) -> std::io::Result<()> {
        buf.fill(0);
        let start = (offset as usize).min(self.0.len());
        let end = (start + buf.len()).min(self.0.len());
        buf[..end - start].copy_from_slice(&self.0[start..end]);
        Ok(())
    }
}

#[test]
fn samples_scale_with_the_file_length() {
    assert_eq!(SampleBudget::new(1000).samples, MIN_SAMPLE_ALLOWANCE);
    assert_eq!(SampleBudget::new(64 << 20).samples, 16 << 20);
    assert_eq!(SampleBudget::new(u64::MAX).samples, MAX_SAMPLES);
    // A constant-size stsz of one-byte samples claims as many as a 64 MiB file has bytes.
    let d = handmade(Some((1, u32::MAX)));
    assert!(matches!(open(Padded(d, 64 << 20)), Err(Error::TooLarge(_))));
}

#[test]
fn a_pcm_chunk_past_4_gib_reads_as_bounded_samples() {
    use crate::isobmff::{Brand, Mp4Writer, PcmConfig, TrackConfig, WriteSample, WriterOptions};
    let pcm = PcmConfig { bits: 8, float: false, big_endian: false, signed: false, channels: 1, sample_rate: 48000.0 };
    let mut w = Mp4Writer::new(std::io::Cursor::new(Vec::new()), WriterOptions::new(Brand::Mov)).unwrap();
    let t = w.add_track(TrackConfig::new(SampleEntry::pcm(pcm), 48000)).unwrap();
    w.write_sample(t, WriteSample { data: &[0; 4], duration: 0, composition_offset: 0, is_sync: true }).unwrap();
    let mut d = w.finish_faststart().unwrap().into_inner();
    // One chunk of u32::MAX one-byte frames, as a writer without chunk limits makes for a
    // 4 GiB recording.
    let frames = u32::MAX.to_be_bytes();
    for (kind, at) in [(b"stts", 12), (b"stsz", 12), (b"stsc", 16)] {
        let p = d.windows(4).position(|w| w == kind).unwrap();
        d[p + at..p + at + 4].copy_from_slice(&frames);
    }
    // In a file too small for it, the claim is clamped to the file.
    let f = open(d.as_slice()).unwrap();
    assert!(f.tracks[0].samples.iter().map(|s| u64::from(s.size)).sum::<u64>() <= d.len() as u64);
    let len = d.len() as u64 + u64::from(u32::MAX);
    let f = open(Padded(d, len)).unwrap();
    let s = &f.tracks[0].samples;
    assert_eq!(s.len(), 256);
    assert!(s.iter().all(|s| u64::from(s.size) <= PCM_MAX_SAMPLE_BYTES));
    assert_eq!(s.iter().map(|s| u64::from(s.size)).sum::<u64>(), u64::from(u32::MAX));
    assert_eq!(s.iter().map(|s| u64::from(s.duration)).sum::<u64>(), u64::from(u32::MAX));
    assert!(s.windows(2).all(|w| w[1].offset == w[0].offset + u64::from(w[0].size) && w[1].dts == w[0].dts + i64::from(w[0].duration)));
}
