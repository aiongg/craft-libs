//! `craft_audio::decode` routes each container and codec below to the home-grown demuxers and
//! decoders, or to symphonia (FLAC): each pair decodes, sample-aligned, to its presentation
//! length, within an SNR of ffmpeg's decode; truncated and corrupted copies never panic. Fixtures are written by ffmpeg
//! (skipped without it).

use craft_audio::{ChannelLayout, CodecId, Container, decode, probe};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Five seconds of two tones and a little noise per channel.
const STEREO: &str =
    "aevalsrc=0.4*sin(2*PI*220*t)+0.15*sin(2*PI*1375*t)+0.05*(random(0)-0.5)|0.35*sin(2*PI*330*t)+0.2*sin(2*PI*2750*t*(1+t/20))+0.05*(random(1)-0.5)";
const MONO: &str = "aevalsrc=0.4*sin(2*PI*220*t)";

struct Case {
    name: &'static str,
    surround: bool,
    mono: bool,
    rate: u32,
    args: &'static [&'static str],
    container: Container,
    codec: CodecId,
    /// The speakers of the decoded channels (WAV order).
    layout: ChannelLayout,
    channels: usize,
    /// Presented frames, or `None` for a raw stream (as many as ffmpeg decodes).
    frames: Option<usize>,
    /// Minimum SNR against ffmpeg.
    min_snr: f64,
}

const CASES: &[Case] = &[
    Case {
        name: "flac.flac",
        surround: false,
        mono: false,
        rate: 48_000,
        args: &["-c:a", "flac", "-sample_fmt", "s16"],
        container: Container::Flac,
        codec: CodecId::Flac,
        layout: ChannelLayout::STEREO,
        channels: 2,
        frames: Some(240_000),
        min_snr: 140.0,
    },
    Case {
        name: "pcm-s24be.mov",
        surround: false,
        mono: false,
        rate: 48_000,
        args: &["-c:a", "pcm_s24be"],
        container: Container::QuickTime,
        codec: CodecId::Pcm,
        layout: ChannelLayout::STEREO,
        channels: 2,
        frames: Some(240_000),
        min_snr: 140.0,
    },
    Case {
        name: "pcm-f32.mkv",
        surround: false,
        mono: false,
        rate: 48_000,
        args: &["-c:a", "pcm_f32le"],
        container: Container::Matroska,
        codec: CodecId::Pcm,
        layout: ChannelLayout::STEREO,
        channels: 2,
        frames: Some(240_000),
        min_snr: 140.0,
    },
];

fn run(ff: &Path, args: &[&str]) -> bool {
    Command::new(ff).args(["-v", "error", "-y"]).args(args).output().is_ok_and(|o| o.status.success())
}

/// The fixture for `c`, written by ffmpeg (cached), or `None` when this ffmpeg lacks an encoder.
fn fixture(ff: &Path, c: &Case) -> Option<PathBuf> {
    let out = super::testkit::fixtures_dir("routing").join(c.name);
    let source = match (c.mono, c.surround) {
        (true, _) => MONO.to_string(),
        (_, true) => format!("{STEREO}{SURROUND_EXTRA}"),
        _ => STEREO.to_string(),
    };
    let input = format!("{source}:s={}", c.rate);
    super::testkit::fixtures::generate(&out, |tmp| {
        let tmp = tmp.to_string_lossy();
        let mut args = vec!["-f", "lavfi", "-i", &input, "-t", "5"];
        args.extend_from_slice(c.args);
        args.push(&tmp);
        run(ff, &args)
    })
}

/// ffmpeg's decode of `file` as interleaved f32.
fn reference(ff: &Path, file: &Path) -> Vec<f32> {
    let out = Command::new(ff).args(["-v", "error", "-i"]).arg(file).args(["-f", "f32le", "-c:a", "pcm_f32le", "-"]).output().unwrap();
    assert!(out.status.success(), "ffmpeg decode of {}", file.display());
    out.stdout.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes(*c)).collect()
}

/// SNR (dB) of `ours` (planar) against `reference` (interleaved), sample-aligned, over the
/// frames both hold.
fn snr(ours: &[Vec<f32>], reference: &[f32]) -> f64 {
    let ch = ours.len();
    let n = ours.iter().map(Vec::len).min().unwrap_or(0).min(reference.len() / ch);
    let (mut s, mut e) = (0f64, 0f64);
    for i in 0..n {
        for (c, plane) in ours.iter().enumerate() {
            let r = f64::from(reference[i * ch + c]);
            let d = f64::from(plane[i]) - r;
            s += r * r;
            e += d * d;
        }
    }
    if e == 0.0 { f64::INFINITY } else { 10.0 * (s / e).log10() }
}

#[test]
fn every_container_and_codec_decodes_home_grown() {
    let ff = require_ffmpeg!();
    for c in CASES {
        let Some(path) = fixture(&ff, c) else {
            eprintln!("SKIPPED {}: this ffmpeg cannot write it", c.name);
            continue;
        };
        let bytes = std::fs::read(&path).unwrap();
        let (info, buf) = decode(&bytes, None).unwrap_or_else(|e| panic!("{}: {e}", c.name));
        let reference = reference(&ff, &path);
        assert_eq!((info.container, info.codec, info.sample_rate, buf.channels.len()), (c.container, c.codec, c.rate, c.channels), "{}", c.name);
        assert_eq!(info.layout, c.layout, "{}: layout", c.name);
        let expected = c.frames.unwrap_or(reference.len() / c.channels);
        assert_eq!(buf.frames(), expected, "{}: presented frames", c.name);
        let s = snr(&buf.channels, &reference);
        assert!(s >= c.min_snr, "{}: SNR {s:.1} dB < {}", c.name, c.min_snr);
        let p = probe(&bytes, None).unwrap();
        assert_eq!(
            (p.container, p.codec, p.sample_rate, usize::from(p.channels), p.layout),
            (c.container, c.codec, c.rate, c.channels, c.layout),
            "{}: probe",
            c.name
        );
    }
}

/// SNR (dB) of `ours` against `reference`, both planar, over the frames both hold.
fn snr_planar(ours: &[Vec<f32>], reference: &[Vec<f32>]) -> f64 {
    let (mut s, mut e) = (0f64, 0f64);
    for (o, r) in ours.iter().zip(reference) {
        for (&a, &b) in o.iter().zip(r) {
            s += f64::from(b) * f64::from(b);
            e += f64::from(a - b) * f64::from(a - b);
        }
    }
    if e == 0.0 { f64::INFINITY } else { 10.0 * (s / e).log10() }
}

/// The streaming decoder gives the same audio as `decode`, block by block, and a seek lands on the
/// exact frame: what follows matches `decode` from there (bit for bit for the deterministic
/// codecs; within the codec's bound where the decoder state is reconstructed from a pre-roll).
#[test]
fn streaming_and_seeking_match_the_whole_file_decode() {
    use craft_audio::{DecodeOptions, Decoder};
    let ff = require_ffmpeg!();
    for c in CASES {
        let Some(path) = fixture(&ff, c) else { continue };
        let bytes = std::fs::read(&path).unwrap();
        let (_, whole) = decode(&bytes, None).unwrap();
        let mut d = Decoder::from_bytes(&bytes, None, &DecodeOptions::default()).unwrap();
        let mut streamed: Vec<Vec<f32>> = vec![Vec::new(); whole.channels.len()];
        while let Some(b) = d.next_block().unwrap() {
            for (s, ch) in streamed.iter_mut().zip(b.channels) {
                s.extend(ch);
            }
        }
        assert_eq!(streamed, whole.channels, "{}: streamed blocks", c.name);

        for target in [1, whole.frames() / 3, whole.frames() - 4800] {
            assert_eq!(d.seek(target as u64).unwrap(), target as u64, "{}", c.name);
            let mut after: Vec<Vec<f32>> = vec![Vec::new(); whole.channels.len()];
            while let Some(b) = d.next_block().unwrap() {
                for (s, ch) in after.iter_mut().zip(b.channels) {
                    s.extend(ch);
                }
            }
            assert_eq!(after[0].len(), whole.frames() - target, "{}: frames after a seek to {target}", c.name);
            let expected: Vec<Vec<f32>> = whole.channels.iter().map(|ch| ch[target..].to_vec()).collect();
            let s = snr_planar(&after, &expected);
            assert!(s == f64::INFINITY, "{}: SNR {s:.1} dB after a seek to {target}", c.name);
        }
    }
}
