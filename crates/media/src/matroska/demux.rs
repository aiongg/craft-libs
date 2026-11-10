//! Matroska / WebM demuxer: EBML header, Segment walk, cluster scanning, packet reading, seeking.

use crate::matroska::error::{Error, Result};
use crate::matroska::meta::{Attachment, ClusterInfo, CuePoint, Edition, SeekEntry, SegmentInfo, Tag};
use crate::matroska::track::Track;

const WIN: usize = 64 * 1024;
/// Upper bound for level-1 metadata elements read into memory (Info, Tracks, Cues, Tags…).
const MAX_META: u64 = 256 << 20;
/// Cap on the frames indexed from one file, all tracks together.
const MAX_FRAMES: usize = 1 << 26;
/// Bytes of file each indexed frame needs beyond the first `MIN_FRAME_ALLOWANCE` frames. An
/// indexed frame takes about 76 bytes of memory (its [`Sample`] and keyframe entry) and fixed
/// lacing can describe one in no bytes at all, so without this a file could make the index
/// about 76 times its own size. Real frames carry a block header or lace size and their data,
/// well over 8 bytes each.
const FILE_BYTES_PER_FRAME: u64 = 8;
/// Frames any file may index, however short: about 80 MiB of index.
const MIN_FRAME_ALLOWANCE: usize = 1 << 20;
/// Cap on the TrackEntries of a file. Each kept entry needs only 5 bytes of file but becomes a
/// [`Track`] of several hundred bytes.
pub(super) const MAX_TRACKS: usize = 1 << 16;

/// What is left of the file-wide limits while clusters are scanned.
struct FrameBudget {
    /// Frames (what the file's length allows, at most `MAX_FRAMES`, minus those already indexed).
    frames: usize,
    /// Bytes of file not yet claimed by a frame. Each frame claims its stored size and at least
    /// one byte, because fixed-size lacing describes up to 256 empty frames in a 4-byte block.
    bytes: u64,
}

impl FrameBudget {
    fn new(file_len: u64) -> Self {
        let by_len = usize::try_from(file_len / FILE_BYTES_PER_FRAME).unwrap_or(usize::MAX);
        Self { frames: by_len.clamp(MIN_FRAME_ALLOWANCE, MAX_FRAMES), bytes: file_len }
    }

    fn take(&mut self, frames: &[(u64, u32)]) -> Result<()> {
        self.frames = self.frames.checked_sub(frames.len()).ok_or(Error::TooLarge("frame count"))?;
        let bytes = frames.iter().map(|&(_, n)| u64::from(n.max(1))).sum::<u64>();
        self.bytes = self.bytes.checked_sub(bytes).ok_or(Error::TooLarge("frame count"))?;
        Ok(())
    }
}

/// Options for [`open_with`] / [`Demuxer::with_options`].
#[derive(Clone, Debug)]
pub struct OpenOptions {
    /// Scan every cluster at open to build per-track sample tables and keyframe indices
    /// (reads only element and block headers, never frame payloads). Default `true`.
    ///
    /// With `false`, open stops at the first Cluster and uses the SeekHead to find Cues, Tags,
    /// Chapters… placed after the clusters; [`Demuxer::seek`] then uses Cues (or builds the index
    /// on first use if there are none).
    pub index: bool,
    /// Verify CRC-32 elements of level-1 elements (and clusters while indexing). Mismatches are
    /// recorded in [`MkvFile::crc_errors`]; parsing continues. Default `false`.
    pub verify_crc: bool,
}

impl Default for OpenOptions {
    fn default() -> Self {
        Self { index: true, verify_crc: false }
    }
}

/// A parsed Matroska / WebM file (first Segment).
#[derive(Clone, Debug)]
pub struct MkvFile {
    /// EBML `DocType`: `matroska` or `webm`.
    pub doc_type: String,
    /// EBML `DocTypeVersion`.
    pub doc_type_version: u64,
    /// EBML `DocTypeReadVersion`: the lowest reader version that can parse the file.
    pub doc_type_read_version: u64,
    /// Absolute offset of the Segment's data (base for SeekHead and Cues positions).
    pub segment_data_start: u64,
    /// Absolute end of the Segment (file end for unknown-size / live segments).
    pub segment_end: u64,
    /// Segment size field was "unknown" (live / streamed output).
    pub live: bool,
    /// Segment `Info` element.
    pub info: SegmentInfo,
    /// Parsed `Tracks` entries, in file order.
    pub tracks: Vec<Track>,
    /// `SeekHead` entries.
    pub seek_head: Vec<SeekEntry>,
    /// Cue points, sorted by time.
    pub cues: Vec<CuePoint>,
    /// Clusters in file order (complete only when [`MkvFile::indexed`]).
    pub clusters: Vec<ClusterInfo>,
    /// Chapter editions from `Chapters`.
    pub chapters: Vec<Edition>,
    /// `Tags` elements.
    pub tags: Vec<Tag>,
    /// Attached files from `Attachments`.
    pub attachments: Vec<Attachment>,
    /// Absolute offset of the first Cluster.
    pub first_cluster: Option<u64>,
    /// Sample tables have been built by scanning all clusters.
    pub indexed: bool,
    /// Number of CRC-32 elements verified (with `verify_crc`).
    pub crc_checked: u32,
    /// Absolute offsets of elements whose CRC-32 did not match.
    pub crc_errors: Vec<u64>,
    /// Non-fatal problems met while parsing (damage, resyncs, truncation).
    pub warnings: Vec<String>,
}

/// One demuxed frame.
#[derive(Clone, Debug, PartialEq)]
pub struct Packet {
    /// Index into [`MkvFile::tracks`].
    pub track: usize,
    /// Matroska `TrackNumber`.
    pub track_number: u64,
    /// Presentation timestamp in timestamp ticks (the track timebase, [`Track::timebase`]).
    pub pts: i64,
    /// Presentation timestamp in nanoseconds.
    pub pts_ns: i64,
    /// Duration in ticks (0 if unknown).
    pub duration: u64,
    /// Duration in nanoseconds (0 if unknown).
    pub duration_ns: u64,
    #[allow(missing_docs)]
    pub keyframe: bool,
    #[allow(missing_docs)]
    pub discardable: bool,
    #[allow(missing_docs)]
    pub invisible: bool,
    /// `DiscardPadding` (ns) from the enclosing BlockGroup.
    pub discard_padding_ns: Option<i64>,
    /// Absolute offset of the stored frame data.
    pub offset: u64,
    /// Index of the frame within a laced block (0 when not laced). Laced frames after the first
    /// only have exact timestamps when the block or track carries a duration; otherwise they
    /// repeat the block timestamp and `duration` is 0.
    pub lace: u16,
    /// Frame bytes with header stripping undone. Frames of tracks with other content encodings
    /// (zlib, encryption) are returned as stored; see [`Track::frames_readable`].
    pub data: Vec<u8>,
}

/// Result of [`Demuxer::seek`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeekPoint {
    /// Index into [`MkvFile::tracks`].
    pub track: usize,
    /// Keyframe timestamp in ticks and ns.
    pub pts: i64,
    /// Keyframe timestamp in nanoseconds.
    pub pts_ns: i64,
    /// Sample index of the keyframe when the file is indexed.
    pub sample: Option<usize>,
    /// Absolute offset of the Cluster holding the keyframe.
    pub cluster_offset: u64,
    /// Absolute offset of the Block holding the keyframe.
    pub block_offset: u64,
}

/// A keyframe index entry ([`Demuxer::keyframe_index`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Keyframe {
    /// Index into the track's sample table.
    pub sample: usize,
    /// Keyframe timestamp in ticks of the track timebase.
    pub pts: i64,
    /// Keyframe timestamp in nanoseconds.
    pub pts_ns: i64,
    /// Absolute offset of the Block holding the keyframe.
    pub block_offset: u64,
}

mod block;
mod io;
mod open;
mod stream;

pub use open::{open, open_with};
pub use stream::Demuxer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_indexed_scale_with_the_file_length() {
        assert_eq!(FrameBudget::new(1000).frames, MIN_FRAME_ALLOWANCE);
        // 64 MiB of file is enough bytes for 2^26 empty frames, but not enough index memory.
        let mut b = FrameBudget::new(64 << 20);
        assert_eq!(b.frames, 8 << 20);
        let lace = [(0u64, 0u32); 256];
        let taken = (0..1 << 18).take_while(|_| b.take(&lace).is_ok()).count();
        assert_eq!(taken * lace.len(), 8 << 20);
        assert_eq!(FrameBudget::new(u64::MAX).frames, MAX_FRAMES);
    }
}
