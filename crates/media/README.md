# craft-media

Media containers and bit-level I/O for the Craft apps: an MSB-first bit reader and writer, and a
clean-room ISO BMFF (MP4) and QuickTime (MOV) demuxer and muxer written from the public
specifications. No `unsafe`, no panics on malformed input, and it builds for
`wasm32-unknown-unknown`. The only dependency is `thiserror`.

## Features

| Feature | Module | Enables |
|---|---|---|
| `bitstream` | `craft_media::bitstream` | none |
| `isobmff` | `craft_media::isobmff` | `bitstream`, `thiserror` |

`default = ["isobmff"]`, so both modules build. A crate that needs only bit I/O (a codec) uses:

```toml
craft-media = { git = "https://github.com/storytold/craft-libs", rev = "<commit>", default-features = false, features = ["bitstream"] }
```

## Boundaries

- In scope: bit-level reading and writing, and container demuxing and muxing (boxes, sample
  tables, sample entries and codec configuration records).
- Out of scope for now: codecs (H.264, HEVC, ProRes, AAC and others). They stay in the apps.
  Matroska, Ogg and other containers can join later as their own features.

## Origin and API

The code comes from FilmCraft's `filmcraft-bitstream` and `filmcraft-isobmff` crates at
[storytold/filmcraft@6c3948a](https://github.com/storytold/filmcraft/commit/6c3948a36c80e4996d924766ba7157eec4fca85e).
Every public item keeps its name, signature and behaviour for valid input, so FilmCraft can turn
both crates into re-exports (`pub use craft_media::isobmff::*;`). The differences:

- `bitstream::BitReader::read_ue_plus` (from DeckCraft's copy) is new.
- `BitReader::peek`, `BitReader::read_bits` and `BitWriter::write_bits` accept widths above 32
  without a debug assertion: `peek` and `read_bits` return the low 32 bits of the field (use
  `read_bits_u64` for wide fields), and `write_bits` writes leading zero bits, at most 64 bits in
  all.
- `bitstream::length_prefixed_nals` returns `BitError::Eof` for a zero `length_size` on non-empty
  data and cannot overflow on a hostile length prefix.
- `isobmff::PcmConfig::bytes_per_frame` saturates at `u32::MAX` for a hostile channel count.

## `bitstream`

- `BitReader`: MSB-first reader with Exp-Golomb (`ue(v)`, `se(v)`, `te(v)`), as used by
  H.264/HEVC/AV1 headers.
- `BitWriter`: the matching writer, for encoders and header rewriting.
- `unescape_rbsp` / `escape_rbsp`: remove or insert H.264/HEVC emulation-prevention bytes.
- `annexb_nals` / `length_prefixed_nals`: split Annex-B byte streams and AVCC/HVCC samples into
  NAL units.
- `crc32`: CRC-32 (IEEE), as used by Matroska and PNG.

## `isobmff`

### Demuxing

```rust
use craft_media::isobmff::{open, TrackKind};

let file = std::fs::File::open("clip.mov")?;
let mp4 = open(&file)?;                        // any ByteSource: &[u8], Vec<u8>, Arc<[u8]>, File, ...
let v = mp4.track_of_kind(TrackKind::Video).unwrap();
let track = &mp4.tracks[v];
let i = track.sample_at_presentation_time(1000).unwrap();   // track timescale, edit-adjusted
let key = track.sync_sample_before(i);          // decode from here
let bytes = mp4.read_sample(&file, v, key)?;    // raw sample (AVCC/HVCC length-prefixed NALs, ...)
```

- `ByteSource` (`len`, `read_at(offset, buf)`) is the input abstraction. Implementations exist for `[u8]`,
  `Vec<u8>`, `&T`, `Arc<T>`, `Box<T>`, and `std::fs::File` on non-wasm targets.
- `open` reads only top-level box headers, `ftyp`, `moov` and every `moof`. It never reads `mdat`.
  The one exception is the first sample of a `tmcd` track, which it reads (4 bytes) to get the timecode
  start frame.
- `Mp4File`: brands, `is_quicktime`, movie `timescale`/`duration`, `tracks`, `metadata`, `fragmented`,
  `fragment_count`.
- `Track`: `id`, `kind` (Video/Audio/Timecode/Subtitle/Other), `handler`, `timescale`, `duration`,
  `language`, `tkhd` fields (size, matrix, volume, enabled), `entries: Vec<SampleEntry>`, `samples`,
  `edits`, `edit_offset`, `references` (`tref`), `sdtp`, `sample_groups` (`sbgp`+`sgpd`, raw), `pcm_chunked`.
  Helpers: `codec()`, `video()`, `audio()`, `presentation_pts(i)`, `sample_at_pts(pts)`,
  `sample_at_presentation_time(t)`, `sync_sample_before(i)`, `sync_samples()`.
- `Sample`: `offset`, `size`, `dts`, `pts` (dts + ctts), `duration`, `is_sync` (all samples are sync when
  `stss` is absent), `description_index`. All times are raw media time in the track timescale.
- **Edit lists:** `edits` holds the `elst` entries unchanged. `edit_offset` is
  `leading empty edits (rescaled to media timescale) − first non-empty edit's media_time`, so
  `presentation time = pts + edit_offset`. This matches ffprobe's timestamps for the common cases
  (B-frame delay, AAC priming, empty-edit offsets). Multi-segment edits, dwells and rate changes are exposed
  raw for the caller to interpret.
- **Uncompressed PCM** (`lpcm`, `sowt`, `twos`, `in24`, `in32`, `fl32`, `fl64`, `raw `, `ipcm`, `fpcm`) is
  merged to one `Sample` per chunk (`pcm_chunked = true`), with `duration` in frames. This avoids one
  table entry per audio frame.

#### Parsed boxes
`ftyp`, `moov/mvhd`, `trak/tkhd`, `edts/elst` (v0/v1), `tref`, `mdia/mdhd` (v0/v1, ISO and Mac language
codes), `hdlr` (ISO and Pascal names), `minf/stbl`: `stsd`, `stts`, `ctts` (v0/v1, read as signed), `stss`,
`stsc`, `stsz`/`stz2` (4/8/16-bit), `stco`/`co64`, `sdtp`, `sbgp`/`sgpd`. Also `mvex/trex`, `moof/traf/tfhd/tfdt/trun`
(every `tfhd` base-offset mode and multiple `moof`s), and `udta` metadata (QuickTime `©xxx` atoms,
`meta/ilst`, and `meta/keys` (mdta)). `uuid`, `free`, `skip`, `wide`, `sidx` and `mfra` are skipped.
Boxes with 64-bit sizes and with size 0 (runs to the end of the file) are supported.

#### Sample entries → `CodecConfig`
| Format | Config |
|---|---|
| `avc1`/`avc3` + `avcC` | `Avc(AvcConfig)`: profile, level, length size, SPS/PPS, extension bytes |
| `hvc1`/`hev1` + `hvcC` | `Hevc(HevcConfig)`: all header fields plus NAL arrays (`vps()/sps()/pps()`) |
| `av01` + `av1C` | `Av1(Av1Config)` |
| `vp09` + `vpcC` | `Vp9(VpcConfig)` |
| `apch apcn apcs apco ap4h ap4x` | `ProRes { fourcc }` |
| `jpeg mjpa mjpb` | `Jpeg { fourcc }` |
| `AVdn AVdh` | `Dnx { fourcc }` |
| `mp4a` + `esds` (in `wave` too) | `Aac(AacConfig)`: ASC bytes + decoded object type/rate/channels; `Mp3` for OTI 0x69/0x6B |
| `.mp3`/`.mp2` | `Mp3`: MPEG-1/2 audio of any layer (each frame header names it) |
| PCM fourccs | `Pcm(PcmConfig)`: bits, float, endianness (`enda`, lpcm flags, `pcmC`), channels, rate |
| `alac` | `Alac { cookie }` |
| `Opus` + `dOps` | `Opus(OpusConfig)` |
| `fLaC` + `dfLa` | `Flac(FlacConfig)` (STREAMINFO decoded) |
| `ac-3`/`ec-3` | `Ac3 { dac3 }` / `Eac3 { dec3 }` |
| `tmcd` | `Timecode(TimecodeConfig)`: flags (drop frame), timescale, frame duration, fps, start frame, reel name; `format_frame()` |
| anything else | `Unknown { fourcc, raw }` |

Video entries also parse `colr` (`nclx`/`nclc`/ICC), `pasp`, `clap`, `fiel`, `gama` and `btrt` into
`VideoParams`. Audio entries parse QuickTime sound description v0/v1/v2 into `AudioParams`.

#### Robustness
The parser does not panic on malformed input; every error comes back as an `Error`. Table counts are
checked against the bytes in their box before anything is allocated. A constant-size `stsz` count is
clamped to what fits in the file. `trun` counts are checked, and each track is capped at 2²⁶ samples.
`moov`/`moof` are capped at 1 GiB. Compressed movie headers (`cmov`) are rejected with
`Error::Unsupported`.

### Muxing

```rust
use craft_media::isobmff::*;
let mut w = Mp4Writer::new(std::io::Cursor::new(Vec::new()), WriterOptions::new(Brand::Mp4))?;
let v = w.add_track(TrackConfig::new(SampleEntry::avc(avcc, 1920, 1080), 90_000))?;
let mut acfg = TrackConfig::new(SampleEntry::aac(asc, 2, 48_000), 48_000);
acfg.media_start = Some(1024);                 // AAC encoder delay → edit list
let a = w.add_track(acfg)?;
w.write_sample(v, WriteSample { data: &au, duration: 3000, composition_offset: 6000, is_sync: true })?;
// ... interleave samples in decode order ...
let bytes = w.finish_faststart()?.into_inner(); // or finish() for moov-at-end
```

- `Mp4Writer<W: Write + Seek>` streams samples into a single `mdat`. `finish()` appends `moov`.
  `finish_faststart()` (needs `W: Read + Write + Seek`) moves the media data forward in place and writes
  `moov` before it. If `mdat` goes past 4 GiB the header becomes a large-size header, and chunk offsets
  switch to `co64` when needed.
- Brands: `Brand::Mp4` writes `isom` with `isom iso2 avc1 mp41 mp42`. `Brand::Mov` writes `qt  `, uses
  QuickTime `hdlr`/Pascal names, and sets `wide` as the placeholder box.
- Tables written: `stts`, `ctts` (v1 when an offset is negative), `stss` (omitted when every sample is
  sync), `stsc`, `stsz` (constant or table), `stco`/`co64`. Consecutive samples of one track share a chunk.
- Sample entries come from `SampleEntry` (`avc`, `hevc`, `prores`, `jpeg`, `aac`, `pcm`, or any demuxed
  entry). Remuxing copies codec config, `colr`, `pasp`, `clap`, `fiel`, `gama` and `btrt`.
  PCM is written as `sowt`/`twos` (16-bit) or `raw ` (8-bit) or sound description v2 `lpcm` in MOV, and as
  `ipcm`/`fpcm` + `pcmC` in MP4. A PCM `write_sample` call can hold any whole number of frames, and the
  track timescale must equal the sample rate.
- Edit lists: set `TrackConfig::edits` explicitly, or use `media_start` to get a single edit.
- Timecode: `add_timecode_track(video, TimecodeConfig, start_frame)` adds a `tmcd` track (with `gmhd`)
  and a `tref/tmcd` link from the video track.
- `WriterOptions::metadata` writes `©xxx` text atoms to `udta`.
- `FragmentedWriter<W: Write>` writes fMP4. The init segment is `ftyp` + `moov` with `mvex/trex`. Each
  `flush_fragment()` then writes `moof` (`mfhd`, `traf` with default-base-is-moof `tfhd`, `tfdt` v1,
  `trun` with per-sample duration, size, flags and composition offset) followed by `mdat`.

### Limitations
- `presentation time = pts + edit_offset` only applies the leading empty edits and the first media
  edit. Multi-segment edit lists, dwell edits and non-unity rates are exposed raw. ffmpeg handles an edit
  that starts mid-frame differently: it snaps to the frame.
- Other data references (external `alis`/`url ` files), encrypted entries (`encv`/`enca`, `sinf`,
  `senc`), `cmov`, and QuickTime reference movies are not supported.
- `sidx`/`mfra` are ignored, because the demuxer scans every top-level box. `stsd` entries with a
  QuickTime colour table are not decoded.
- Subtitle and text tracks (`tx3g`, `c608`, `wvtt`) and `mp4v` show up as `Unknown`.
- Muxer: one `mdat`, no chunk-size or duration limits (the caller controls interleaving), no `sdtp`,
  `sgpd`/`sbgp` or `cslg` output, ISO-style `meta/ilst` metadata is not written, and the fragmented
  writer has no PCM, `sidx` or `mfra`.

## Tests

- Unit tests cover the bit reader and writer, the box reader, hand-built `stbl` edge cases, codec
  configuration round trips and allocation guards.
- `tests/roundtrip.rs`: property tests that write random multi-track files (progressive and
  fragmented) and read them back.
- `tests/oracle_demux.rs` and `tests/oracle_mux.rs` compare the demuxer and muxer with ffprobe and
  ffmpeg on generated fixtures in `target/fixtures/isobmff/` (`$CRAFT_MEDIA_FIXTURES_DIR`
  overrides it). Fixtures are never committed.
- `tests/fuzz.rs`: deterministic mutation fuzzing; nothing may panic.

ffmpeg and ffprobe are test oracles only, never linked. They are looked up in
`$CRAFT_MEDIA_FFMPEG` / `$CRAFT_MEDIA_FFPROBE`, then `PATH`, then common install directories. When
a tool is missing, the oracle tests print `SKIPPED` and pass; `CRAFT_MEDIA_REQUIRE_ORACLES=1` turns
a skip into a failure. Generate every fixture ahead of time with
`cargo test -p craft-media --test oracle_demux -- --ignored generate_fixtures`.
