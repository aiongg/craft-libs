//! Hostile inputs must produce `Err` (or a best-effort `Ok`), never a panic.

use craft_audio::{AudioError, decode, probe};
use proptest::prelude::*;

fn caf(chunks: &[(&[u8; 4], i64, Vec<u8>)]) -> Vec<u8> {
    let mut out = b"caff\0\x01\0\0".to_vec();
    for (kind, size, body) in chunks {
        out.extend_from_slice(*kind);
        out.extend_from_slice(&size.to_be_bytes());
        out.extend_from_slice(body);
    }
    out
}

fn caf_desc(format: &[u8; 4], bytes_per_packet: u32, frames_per_packet: u32, channels: u32, bits: u32) -> (&'static [u8; 4], i64, Vec<u8>) {
    let mut d = 48_000f64.to_be_bytes().to_vec();
    d.extend_from_slice(format);
    for v in [0, bytes_per_packet, frames_per_packet, channels, bits] {
        d.extend_from_slice(&v.to_be_bytes());
    }
    (b"desc", 32, d)
}

fn packet_table(packets: i64, frames: i64, entries: &[u8]) -> Vec<u8> {
    [&packets.to_be_bytes()[..], &frames.to_be_bytes(), &[0; 8], entries].concat()
}

fn atom(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
    [&(8 + body.len() as u32).to_be_bytes()[..], kind, body].concat()
}

/// A mono 16-bit, 48 kHz audio sample entry of `version`, its version fields and codec atoms.
fn sample_entry(kind: &[u8; 4], version: u8, fields: &[u8], codec: &[u8]) -> Vec<u8> {
    let base = [0, 0, 0, 0, 0, 0, 0, 1, 0, version, 0, 0, 0, 0, 0, 0, 0, 1, 0, 16, 0, 0, 0, 0];
    atom(kind, &[&base[..], &(48_000u32 << 16).to_be_bytes(), fields, codec].concat())
}

/// An MP4 of `frames` 2-byte samples described by `entry`; `swap` may replace any version-0 atom.
fn mp4(frames: u32, entry: &[u8], swap: impl Fn(&[u8; 4], Vec<u8>) -> Vec<u8>) -> Vec<u8> {
    let full = |kind: &[u8; 4], fields: &[u32]| {
        let body: Vec<u8> = [0].iter().chain(fields).flat_map(|v: &u32| v.to_be_bytes()).collect();
        swap(kind, atom(kind, &body))
    };
    let moov = |offset: u32| {
        let stsd = atom(b"stsd", &[&[0, 0, 0, 0, 0, 0, 0, 1][..], entry].concat());
        let stbl =
            [stsd, full(b"stts", &[1, frames, 1]), full(b"stsc", &[1, 1, frames, 1]), full(b"stsz", &[2, frames]), full(b"stco", &[1, offset])];
        let minf = [full(b"smhd", &[0]), atom(b"stbl", &stbl.concat())].concat();
        let mdia = [full(b"mdhd", &[0, 0, 48_000, frames, 0]), full(b"hdlr", &[0, u32::from_be_bytes(*b"soun"), 0, 0, 0, 0]), atom(b"minf", &minf)];
        let trak = [full(b"tkhd", &[0; 20]), atom(b"mdia", &mdia.concat())].concat();
        atom(b"moov", &[full(b"mvhd", &[[0, 0, 48_000, frames, 1 << 16].as_slice(), &[0; 18], &[2]].concat()), atom(b"trak", &trak)].concat())
    };
    let ftyp = atom(b"ftyp", b"M4A \0\0\0\0M4A isom");
    let offset = (ftyp.len() + moov(0).len() + 8) as u32;
    let pcm: Vec<u8> = (0..frames as i16).flat_map(|i| (i * 40).to_le_bytes()).collect();
    [ftyp, moov(offset), atom(b"mdat", &pcm)].concat()
}

/// A false MPEG sync prefix must not let malformed CAF or MP4 size fields bypass the container preflight.
#[test]
fn a_false_sync_does_not_bypass_container_checks() {
    let bad_caf = caf(&[
        caf_desc(b"aac ", 0, 1024, 2, 0),
        (b"pakt", 32, packet_table(1, 1024, &[0x82, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0])),
        (b"data", -1, vec![0; 64]),
    ]);
    assert!(matches!(probe(&bad_caf, None), Err(AudioError::Malformed(m)) if m.contains("larger than the file")));
    let f = [vec![0xFF, 0xE0, 0x00], bad_caf].concat();
    assert!(matches!(probe(&f, None), Err(AudioError::Malformed(m)) if m.contains("larger than the file")));
    assert!(matches!(decode(&f, None), Err(AudioError::Malformed(m)) if m.contains("larger than the file")));

    let pcm = sample_entry(b"sowt", 0, &[], &[]);
    let bad_mp4 = mp4(480, &pcm, |kind, mut a| {
        if kind == b"stsz" {
            a[12..16].copy_from_slice(&u32::MAX.to_be_bytes());
        }
        a
    });
    assert!(probe(&bad_mp4, None).is_err());
    let f = [vec![0xFF, 0xE0, 0x00], bad_mp4].concat();
    assert!(matches!(probe(&f, None), Err(AudioError::Malformed(m)) if m.contains("larger than the file")));
    assert!(matches!(decode(&f, None), Err(AudioError::Malformed(m)) if m.contains("larger than the file")));
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 256, ..ProptestConfig::default() })]


    #[test]
    fn random_bodies_behind_valid_headers_never_panic(body in proptest::collection::vec(any::<u8>(), 0..1024), which in 0usize..4) {
        let mut f = match which {
            0 => b"RIFF\xff\xff\xff\xffWAVE".to_vec(),
            1 => b"RF64\xff\xff\xff\xffWAVE".to_vec(),
            2 => b"FORM\xff\xff\xff\xffAIFC".to_vec(),
            _ => b"FORM\xff\xff\xff\xffAIFF".to_vec(),
        };
        f.extend_from_slice(&body);
        let _ = probe(&f, None);
        let _ = decode(&f, None);
    }

}
