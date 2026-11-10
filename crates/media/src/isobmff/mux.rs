//! Muxers: progressive MP4/MOV ([`Mp4Writer`], optional faststart) and fragmented MP4 ([`FragmentedWriter`]).

use crate::isobmff::bytes::BoxBuf;
use crate::isobmff::codec::{CodecConfig, SampleEntry, TimecodeConfig};
use crate::isobmff::demux::Edit;
use crate::isobmff::error::{Error, Result};
use std::io::{Read, Seek, SeekFrom, Write};

/// Output flavor: determines brands and some sample-entry encodings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Brand {
    /// ISO MP4: `isom` major brand, compatible `isom iso2 avc1 mp41 mp42`.
    Mp4,
    /// QuickTime MOV: `qt  ` major brand.
    Mov,
}

/// Movie-level writer options.
#[derive(Clone, Debug)]
pub struct WriterOptions {
    /// File type brand written to `ftyp`.
    pub brand: Brand,
    /// Movie timescale (`mvhd`); default 1000.
    pub movie_timescale: u32,
    /// Text metadata written to `moov/udta` (keys like `©nam`, `©ART`, `©cmt`, `©too`).
    pub metadata: Vec<(String, String)>,
}

impl WriterOptions {
    /// Options for `brand` with a 1000 ticks-per-second movie timescale and no metadata.
    pub fn new(brand: Brand) -> Self {
        WriterOptions { brand, movie_timescale: 1000, metadata: Vec::new() }
    }
}

/// Per-track configuration.
#[derive(Clone, Debug)]
pub struct TrackConfig {
    /// Sample entry (codec configuration) for the track's `stsd` box.
    pub entry: SampleEntry,
    /// Media timescale. For PCM it must equal the sample rate (one tick per audio frame).
    pub timescale: u32,
    /// ISO 639-2/T language (`und` by default).
    pub language: String,
    /// Explicit edit list (segment durations in the movie timescale). Takes precedence over `media_start`.
    pub edits: Vec<Edit>,
    /// Convenience: create one edit starting at this media time (e.g. AAC encoder delay / priming,
    /// or the first pts of a B-frame stream) spanning the rest of the media.
    pub media_start: Option<i64>,
    /// `hdlr` name; a sensible default is used when `None`.
    pub handler_name: Option<String>,
}

impl TrackConfig {
    /// Configuration for `entry` and a media `timescale` in ticks per second, with language `und`, no edits and no handler name.
    pub fn new(entry: SampleEntry, timescale: u32) -> Self {
        TrackConfig { entry, timescale, language: "und".into(), edits: Vec::new(), media_start: None, handler_name: None }
    }
}

/// A sample handed to the muxers.
#[derive(Clone, Copy, Debug)]
pub struct WriteSample<'a> {
    /// Encoded sample payload, in the format the track's sample entry declares.
    pub data: &'a [u8],
    /// Duration in the media timescale. Ignored for PCM (each frame lasts one tick).
    pub duration: u32,
    /// pts − dts in the media timescale.
    pub composition_offset: i32,
    /// True for a sync sample (random access point, such as a key frame).
    pub is_sync: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Handler {
    Video,
    Audio,
    Timecode,
    Other,
}

fn handler_of(e: &SampleEntry) -> Handler {
    if matches!(e.codec, CodecConfig::Timecode(_)) {
        Handler::Timecode
    } else if e.video.is_some() {
        Handler::Video
    } else if e.audio.is_some() {
        Handler::Audio
    } else {
        Handler::Other
    }
}

/// The `CodecConfig` variant, and the fourcc of variants that cover several codecs.
fn same_codec(a: &CodecConfig, b: &CodecConfig) -> bool {
    use CodecConfig::*;
    match (a, b) {
        (ProRes { fourcc: x }, ProRes { fourcc: y })
        | (Jpeg { fourcc: x }, Jpeg { fourcc: y })
        | (Dnx { fourcc: x }, Dnx { fourcc: y })
        | (Unknown { fourcc: x, .. }, Unknown { fourcc: y, .. }) => x == y,
        _ => std::mem::discriminant(a) == std::mem::discriminant(b),
    }
}

/// Accumulated sample tables for one track.
#[derive(Clone, Debug, Default)]
struct Tables {
    stts: Vec<(u32, u32)>,
    ctts: Vec<(u32, i32)>,
    sizes: Vec<u32>,
    /// Constant sample size (PCM) — then `sizes` is unused.
    const_size: Option<u32>,
    sample_count: u64,
    /// 1-based sync sample numbers.
    sync: Vec<u32>,
    all_sync: bool,
    /// (offset relative to file start before shifting, sample count)
    chunks: Vec<(u64, u32)>,
    media_duration: u64,
    /// End of the last sample on the composition timeline (decode time + composition offset +
    /// duration, maximum over samples).
    comp_end: u64,
}

impl Tables {
    fn new() -> Self {
        Tables { all_sync: true, ..Default::default() }
    }
    fn push_run<T: PartialEq + Copy>(runs: &mut Vec<(u32, T)>, mut count: u64, v: T) {
        if let Some(last) = runs.last_mut()
            && last.1 == v
        {
            let n = count.min(u64::from(u32::MAX - last.0));
            last.0 += n as u32;
            count -= n;
        }
        while count > 0 {
            let n = count.min(u64::from(u32::MAX));
            runs.push((n as u32, v));
            count -= n;
        }
    }

    /// Records `count` samples written at `offset`, `sample_bytes` each, extending the last
    /// chunk when the write follows it. A chunk holds at most `max` samples.
    fn push_chunks(&mut self, mut offset: u64, mut count: u64, sample_bytes: u64, contiguous: bool, max: u32) {
        if contiguous && let Some(c) = self.chunks.last_mut() {
            let n = count.min(u64::from(max.saturating_sub(c.1)));
            c.1 += n as u32;
            count -= n;
            offset += n * sample_bytes;
        }
        while count > 0 {
            let n = count.min(u64::from(max));
            self.chunks.push((offset, n as u32));
            count -= n;
            offset += n * sample_bytes;
        }
    }
}

/// `stsz` counts samples (PCM: frames) in 32 bits.
fn too_many_samples() -> Error {
    Error::Mux("more than 2^32 - 1 samples in one track".into())
}

/// PCM chunk size limit, ffmpeg's: readers that take each PCM chunk as one sample (this crate's
/// demuxer among them) get samples of bounded size.
const PCM_CHUNK_BYTES: u64 = 1 << 20;

/// Frames per PCM chunk: at most [`PCM_CHUNK_BYTES`], at least one frame.
fn pcm_chunk_frames(frame_bytes: u32) -> u32 {
    (PCM_CHUNK_BYTES / u64::from(frame_bytes.max(1))).max(1) as u32
}

struct WTrack {
    cfg: TrackConfig,
    id: u32,
    handler: Handler,
    pcm_frame_bytes: Option<u32>,
    t: Tables,
    /// Track id of a linked timecode track (`tref/tmcd`).
    tmcd_ref: Option<u32>,
    /// For timecode tracks: (video track index, start frame).
    tmcd_source: Option<(usize, u32)>,
}

fn pack_language(lang: &str) -> u16 {
    let b = lang.as_bytes();
    if b.len() == 3 && b.iter().all(|c| c.is_ascii_lowercase()) {
        (((b[0] - 0x60) as u16) << 10) | (((b[1] - 0x60) as u16) << 5) | ((b[2] - 0x60) as u16)
    } else {
        0x55C4 // "und"
    }
}

fn rescale(v: u64, from: u32, to: u32) -> u64 {
    if from == 0 {
        return 0;
    }
    (v as u128 * to as u128 / from as u128).min(u64::MAX as u128) as u64
}

fn validate_track(entry: &SampleEntry, timescale: u32) -> Result<Option<u32>> {
    if timescale == 0 {
        return Err(Error::Mux("timescale must be non-zero".into()));
    }
    if let CodecConfig::Pcm(p) = &entry.codec {
        if p.bits == 0 || p.channels == 0 {
            return Err(Error::Mux("PCM needs bits and channels".into()));
        }
        if (p.sample_rate - timescale as f64).abs() > 0.5 {
            return Err(Error::Mux("PCM track timescale must equal the sample rate".into()));
        }
        return Ok(Some(p.bytes_per_frame()));
    }
    Ok(None)
}

mod boxes;
mod fragmented;
mod progressive;

pub use fragmented::FragmentedWriter;
pub use progressive::Mp4Writer;

#[cfg(test)]
mod tests;
