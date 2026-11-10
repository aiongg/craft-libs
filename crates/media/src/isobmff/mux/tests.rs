//! Muxer tests.

use super::*;

use crate::isobmff::codec::{AvcConfig, PcmConfig};
use std::io::Cursor;

fn avc_entry() -> SampleEntry {
    SampleEntry::avc(AvcConfig::new(vec![vec![0x67, 0x42, 0, 0x1E]], vec![vec![0x68, 0xCE]], 4), 64, 48)
}

#[test]
fn language_pack() {
    assert_eq!(pack_language("und"), 0x55C4);
    assert_eq!(pack_language("eng"), 0x15C7);
    assert_eq!(pack_language("EN"), 0x55C4);
}

#[test]
fn simple_roundtrip_and_faststart() {
    for fast in [false, true] {
        let mut w = Mp4Writer::new(Cursor::new(Vec::new()), WriterOptions::new(Brand::Mp4)).unwrap();
        let v = w.add_track(TrackConfig::new(avc_entry(), 12800)).unwrap();
        let pcm = PcmConfig { bits: 16, float: false, big_endian: false, signed: true, channels: 2, sample_rate: 48000.0 };
        let a = w.add_track(TrackConfig::new(SampleEntry::pcm(pcm), 48000)).unwrap();
        for i in 0..10u8 {
            let data = vec![i; 100 + i as usize];
            w.write_sample(v, WriteSample { data: &data, duration: 512, composition_offset: if i % 2 == 0 { 1024 } else { 0 }, is_sync: i % 5 == 0 })
                .unwrap();
            w.write_sample(a, WriteSample { data: &[i; 400], duration: 0, composition_offset: 0, is_sync: true }).unwrap();
        }
        let out = if fast { w.finish_faststart() } else { w.finish() }.unwrap().into_inner();
        let f = crate::isobmff::open(out.as_slice()).unwrap();
        assert_eq!(f.tracks.len(), 2);
        let t = &f.tracks[0];
        assert_eq!(t.samples.len(), 10);
        for (i, s) in t.samples.iter().enumerate() {
            assert_eq!(s.size as usize, 100 + i);
            assert_eq!(s.dts, 512 * i as i64);
            assert_eq!(s.is_sync, i % 5 == 0);
            assert_eq!(f.read_sample(out.as_slice(), 0, i).unwrap(), vec![i as u8; 100 + i]);
        }
        let at = &f.tracks[1];
        assert!(at.pcm_chunked);
        assert_eq!(at.samples.len(), 10);
        assert_eq!(at.samples[3].size, 400);
        assert_eq!(at.samples[3].duration, 100);
        assert_eq!(f.read_sample(out.as_slice(), 1, 3).unwrap(), vec![3u8; 400]);
        if fast {
            assert!(f.moov_offset < t.samples[0].offset);
        }
    }
}

#[test]
fn set_sample_entry_keeps_track_layout() {
    let mut w = Mp4Writer::new(Cursor::new(Vec::new()), WriterOptions::new(Brand::Mov)).unwrap();
    let pcm = PcmConfig { bits: 16, float: false, big_endian: false, signed: true, channels: 2, sample_rate: 48000.0 };
    let a = w.add_track(TrackConfig::new(SampleEntry::pcm(pcm), 48000)).unwrap();
    let f = w.add_track(TrackConfig::new(SampleEntry::flac(&[0; 34]), 48000)).unwrap();
    assert!(w.set_sample_entry(a, SampleEntry::pcm(PcmConfig { big_endian: true, ..pcm })).is_ok());
    assert!(w.set_sample_entry(a, SampleEntry::pcm(PcmConfig { bits: 24, ..pcm })).is_err());
    assert!(w.set_sample_entry(a, avc_entry()).is_err());
    let v = w.add_track(TrackConfig::new(avc_entry(), 90_000)).unwrap();
    assert!(w.set_sample_entry(v, avc_entry()).is_ok());
    assert!(w.set_sample_entry(v, SampleEntry::jpeg(64, 64)).is_err());
    assert!(w.set_sample_entry(f, SampleEntry::flac(&[1; 34])).is_ok());
    assert!(w.set_sample_entry(f, SampleEntry::pcm(pcm)).is_err());
    assert!(matches!(w.set_sample_entry(5, avc_entry()), Err(Error::NoSuchTrack(5))));
}

#[test]
fn fragmented_roundtrip() {
    let mut w = FragmentedWriter::new(Vec::new(), WriterOptions::new(Brand::Mp4));
    let v = w.add_track(TrackConfig::new(avc_entry(), 1000)).unwrap();
    for i in 0..6u8 {
        w.write_sample(v, WriteSample { data: &[i; 10], duration: 40, composition_offset: -(i as i32 % 2), is_sync: i % 3 == 0 }).unwrap();
        if i % 3 == 2 {
            w.flush_fragment().unwrap();
        }
    }
    let out = w.finish().unwrap();
    let f = crate::isobmff::open(out.as_slice()).unwrap();
    assert!(f.fragmented);
    assert_eq!(f.fragment_count, 2);
    let t = &f.tracks[0];
    assert_eq!(t.samples.len(), 6);
    for (i, s) in t.samples.iter().enumerate() {
        assert_eq!(s.dts, 40 * i as i64);
        assert_eq!(s.pts, s.dts - (i as i64 % 2));
        assert_eq!(s.is_sync, i % 3 == 0);
        assert_eq!(f.read_sample(out.as_slice(), 0, i).unwrap(), vec![i as u8; 10]);
    }
    assert_eq!(t.sync_sample_before(5), 3);
}
