//! Decode path for everything that isn't native PCM WAV/AIFF, backed by `symphonia`.

use std::borrow::Cow;
use std::io::{Cursor, Seek, SeekFrom};
use std::panic::{AssertUnwindSafe, catch_unwind};

use symphonia::core::codecs::audio::AudioDecoder;
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::codecs::audio::well_known::CODEC_ID_ALAC;
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::units::{TimeBase, Timestamp};

use crate::pcm::{MAX_DECODE_SAMPLES, be_u32};
use crate::{AudioBuffer, AudioError, AudioInfo, ChannelLayout, CodecId, Container, Result};

/// The most frames a packet may declare: decoders allocate a packet's frames up front, and ALAC
/// takes its frame length from the file (encoders use 4096).
const MAX_PACKET_FRAMES: u64 = 1 << 20;

/// Turn a symphonia error into the audio layer's error, keeping "unsupported" and "too large"
/// distinct from a malformed file.
fn map_err(e: SymError) -> AudioError {
    match e {
        SymError::Unsupported(m) => AudioError::Unsupported(m.to_string()),
        SymError::LimitError(m) => AudioError::TooLarge(m.to_string()),
        other => AudioError::Malformed(other.to_string()),
    }
}

/// Run `f`, turning a panic inside the third-party decoder into an error.
fn guarded<T>(f: impl FnOnce() -> Result<T>) -> Result<T> {
    catch_unwind(AssertUnwindSafe(f)).unwrap_or_else(|_| Err(AudioError::Malformed("decoder failed on malformed input".into())))
}

/// What [`open`] read from the file for what symphonia's FLAC and Ogg readers leave out.
enum Preread {
    /// Not a FLAC file or an Ogg file.
    Nothing,
    /// A FLAC file's layout.
    Flac(ChannelLayout),
    /// An Ogg file's audio streams in file order: each one's serial, which is symphonia's track ID,
    /// and its FLAC channel mask. Symphonia orders the tracks by serial, and the tags its reader
    /// collects do not say which track they are from.
    Ogg(Vec<(u32, Option<u32>)>),
}

/// What [`open`] found: a reader, or a stream that holds no audio frames and has no reader.
enum Opened {
    Reader(Box<dyn FormatReader>, Preread),
    Empty(AudioInfo),
}

/// Decoder options for every compressed format. Gapless playback used to be a
/// `FormatOptions` flag in symphonia 0.5; in 0.6 the decoder option (on by default) trims the
/// encoder delay and padding that the demuxer reports on each packet.
fn decoder_options() -> AudioDecoderOptions {
    AudioDecoderOptions::default().gapless(true)
}

mod map;
mod precheck;
use map::{audio_params, audio_tracks, codec_id_of, info_of};
use precheck::{Precheck, SIZE_CHECKED_PROBE_FORMATS, precheck, skip_id3v2};

/// Skip leading ID3v2 tags, run the container size preflight, then probe with symphonia; a zero-frame FLAC found by
/// the preflight comes back as [`Opened::Empty`] without probing. A size-checked container is refused when probing
/// selects a different reader or one that was never checked, so incidental payload magic cannot bypass those limits.
fn open(bytes: Cow<'_, [u8]>, detected: Option<Container>) -> Result<Opened> {
    let start = skip_id3v2(&bytes);
    if start > 0 && start >= bytes.len() {
        return Err(AudioError::Malformed("ID3v2 tag extends past the end of the file".into()));
    }
    let (checked, flac_layout) = match precheck(&bytes, start, detected)? {
        Precheck::Empty(info) => return Ok(Opened::Empty(info)),
        Precheck::Flac(expected, layout) => (Precheck::Checked(expected), layout),
        checked => (checked, None),
    };
    let body = bytes.get(start..).unwrap_or_default();
    let ogg_streams = craft_media::ogg::sniff(body).then(|| super::flac::ogg_audio_streams(body));
    // symphonia reads from an owned stream: a borrowed buffer is copied once, a buffer read from
    // a shared source is moved in.
    let mut mss = MediaSourceStream::new(Box::new(Cursor::new(bytes.into_owned())), MediaSourceStreamOptions::default());
    // Seek (rather than pre-positioning the cursor) so the stream's absolute position is `start`
    // too: container offsets such as MP4's `stco` are absolute file offsets.
    mss.seek(SeekFrom::Start(u64::try_from(start).unwrap_or(u64::MAX)))?;
    let reader: Box<dyn FormatReader> =
        symphonia::default::get_probe().probe(&Hint::new(), mss, FormatOptions::default(), MetadataOptions::default()).map_err(map_err)?;
    let probed = reader.format_info().short_name;
    match checked {
        // Only containers whose readers allocate from sizes in the file need the check. A stream
        // the probe reads with another reader (MP3, Ogg, AIFF, ...) has no such size fields, so a
        // marker found in its audio data must not reject it.
        Precheck::Checked(expected) if SIZE_CHECKED_PROBE_FORMATS.contains(&probed) && probed != expected => {
            return Err(AudioError::Malformed(format!("the probed container ({probed}) is not the one whose size checks ran")));
        }
        Precheck::Unchecked if SIZE_CHECKED_PROBE_FORMATS.contains(&probed) => {
            return Err(AudioError::Malformed(format!("the probed container ({probed}) was not size-checked")));
        }
        _ => {}
    }
    let preread = match (flac_layout.filter(|_| probed == "flac"), ogg_streams.filter(|_| probed == "ogg")) {
        (Some(layout), _) => Preread::Flac(layout),
        (None, Some(streams)) => Preread::Ogg(streams),
        _ => Preread::Nothing,
    };
    Ok(Opened::Reader(reader, preread))
}

/// A demux error ends the decode; it is only an error when nothing was decoded yet.
fn demux_failure(frames: u64, e: SymError) -> Result<()> {
    if frames == 0 {
        return Err(map_err(e));
    }
    log::warn!("stopping decode after demux error: {e}");
    Ok(())
}

/// Note a corrupt packet (the `errors`-th); a stream that never decodes is malformed.
fn corrupt_packet(errors: usize, frames: u64, msg: &str) -> Result<()> {
    log::debug!("concealing corrupt packet: {msg}");
    if errors > 1000 && frames == 0 {
        return Err(AudioError::Malformed(format!("no decodable packets: {msg}")));
    }
    Ok(())
}

/// A file decoded by symphonia, block by block.
pub(crate) struct Symph {
    /// `None` for a stream with no audio frames (a zero-frame FLAC).
    reader: Option<Box<dyn FormatReader>>,
    decoder: Option<Box<dyn AudioDecoder>>,
    track_id: u32,
    time_base: Option<TimeBase>,
    info: AudioInfo,
    frames_known: bool,
    /// Frames decoded so far (for the "nothing decoded" checks).
    decoded: u64,
    errors: usize,
    /// Frames of the last decoded packet: the length of the silence that replaces a packet that
    /// fails to decode when the demuxer gives no duration for it, so the audio after it keeps its
    /// timing (0 before the first packet decodes).
    last_frames: usize,
    /// Frames still to drop after a seek landed before its target.
    discard: u64,
    ended: bool,
    interleaved: Vec<f32>,
}

impl Symph {
    /// Open audio track `track` (the first when `None`) of the file in `bytes`.
    pub(crate) fn open(bytes: Cow<'_, [u8]>, detected: Option<Container>, track: Option<usize>) -> Result<Symph> {
        guarded(move || {
            let (reader, preread) = match open(bytes, detected)? {
                Opened::Reader(reader, preread) => (reader, preread),
                Opened::Empty(info) => {
                    // a stream with no audio still has one track, as WAV and AIFF do
                    if let Some(n) = track.filter(|&n| n > 0) {
                        return Err(AudioError::Unsupported(format!("no audio track {n}: the file has one")));
                    }
                    return Ok(Symph {
                        reader: None,
                        decoder: None,
                        track_id: 0,
                        time_base: None,
                        info,
                        frames_known: true,
                        decoded: 0,
                        errors: 0,
                        last_frames: 0,
                        discard: 0,
                        ended: true,
                        interleaved: Vec::new(),
                    });
                }
            };
            let ogg_streams = if let Preread::Ogg(streams) = &preread { streams.as_slice() } else { &[] };
            let (track_id, mut info) = info_of(reader.as_ref(), detected, ogg_streams, track)?;
            match &preread {
                Preread::Flac(layout) if layout.channels() == info.channels => info.layout = *layout,
                Preread::Ogg(streams) if info.codec == CodecId::Flac => {
                    let mask = streams.iter().find(|(serial, _)| *serial == track_id).and_then(|&(_, mask)| mask);
                    info.layout = super::flac::layout_from(mask, info.channels);
                }
                _ => {}
            }
            let t = reader.tracks().iter().find(|t| t.id == track_id).ok_or_else(|| AudioError::Malformed("track vanished".into()))?;
            let params = audio_params(t).cloned().ok_or_else(|| AudioError::Malformed("track vanished".into()))?;
            let alac = params.extra_data.as_deref().filter(|_| params.codec == CODEC_ID_ALAC).and_then(|d| be_u32(d, 0));
            if params.max_frames_per_packet.into_iter().chain(alac.map(u64::from)).any(|n| n > MAX_PACKET_FRAMES) {
                return Err(AudioError::Malformed("the file declares packets far longer than any codec uses".into()));
            }
            let decoder = symphonia::default::get_codecs().make_audio_decoder(&params, &decoder_options()).map_err(map_err)?;
            let frames_known = info.frames > 0 && info.channels > 0;
            Ok(Symph {
                time_base: t.time_base,
                reader: Some(reader),
                decoder: Some(decoder),
                track_id,
                info,
                frames_known,
                decoded: 0,
                errors: 0,
                last_frames: 0,
                discard: 0,
                ended: false,
                interleaved: Vec::new(),
            })
        })
    }

    pub(crate) fn info(&self) -> &AudioInfo {
        &self.info
    }

    pub(crate) fn frames_known(&self) -> bool {
        self.frames_known
    }

    /// The BWF metadata of the WAV around the stream (symphonia does not read `bext`).
    pub(crate) fn set_bwf(&mut self, bwf: Option<crate::BwfInfo>) {
        self.info.bwf = bwf;
    }

    /// The next block of decoded audio, or `None` at the end.
    pub(crate) fn next_block(&mut self) -> Result<Option<AudioBuffer>> {
        guarded(|| self.next_block_inner())
    }

    fn next_block_inner(&mut self) -> Result<Option<AudioBuffer>> {
        let (Some(reader), Some(decoder)) = (self.reader.as_mut(), self.decoder.as_mut()) else { return Ok(None) };
        loop {
            if self.ended {
                if self.info.channels == 0 {
                    return Err(AudioError::Malformed("stream contains no audio".into()));
                }
                return Ok(None);
            }
            let packet = match reader.next_packet() {
                Ok(Some(packet)) => packet,
                Ok(None) | Err(SymError::ResetRequired) => {
                    self.ended = true;
                    continue;
                }
                Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    self.ended = true;
                    continue;
                }
                Err(e) => {
                    demux_failure(self.decoded, e)?;
                    self.ended = true;
                    continue;
                }
            };
            if packet.track_id != self.track_id {
                continue;
            }
            let block = match decoder.decode(&packet) {
                Ok(decoded) => {
                    // The interleaved copy below is laid out by planes: use their count so a
                    // decoder whose channel spec disagrees with its planes cannot trip the copy's
                    // length assertion.
                    let ch = decoded.num_planes();
                    if ch == 0 {
                        continue;
                    }
                    let n = decoded.frames();
                    let needed = n
                        .checked_mul(ch)
                        .filter(|&t| t as u64 <= MAX_DECODE_SAMPLES)
                        .ok_or_else(|| AudioError::TooLarge(format!("a decoded block exceeds the {MAX_DECODE_SAMPLES}-sample decode limit")))?;
                    if self.interleaved.len() < needed {
                        self.interleaved
                            .try_reserve(needed - self.interleaved.len())
                            .map_err(|_| AudioError::TooLarge("cannot allocate decoded audio".into()))?;
                        self.interleaved.resize(needed, 0.0);
                    }
                    decoded.copy_to_slice_interleaved(&mut self.interleaved[..needed]);
                    let mut planar = crate::pcm::planar(ch, n)?;
                    for frame in self.interleaved[..needed].chunks_exact(ch) {
                        for (dst, &s) in planar.iter_mut().zip(frame) {
                            dst.push(if s.is_finite() { s } else { 0.0 });
                        }
                    }
                    // The decoded stream is authoritative for the channel count.
                    if usize::from(self.info.channels) != ch {
                        self.info.channels = u16::try_from(ch).map_err(|_| AudioError::Unsupported("too many channels".into()))?;
                        self.info.layout = ChannelLayout::unordered(crate::layout::count(ch));
                    }
                    self.decoded = self.decoded.saturating_add(n as u64);
                    self.last_frames = n;
                    planar
                }
                Err(SymError::DecodeError(m)) => {
                    self.errors += 1;
                    self.info.damaged += 1;
                    corrupt_packet(self.errors, self.decoded, m)?;
                    // the packet's own duration when the demuxer gives one (Vorbis packets vary in
                    // length; it leaves out priming), otherwise the last packet's: a damaged first
                    // packet is concealed only when its duration is known
                    let rate = u64::from(self.info.sample_rate.max(1));
                    let (numer, denom) = self.time_base.map_or((1, rate), |tb| (u64::from(tb.numer.get()), u64::from(tb.denom.get())));
                    let own = u128::from(packet.dur.get()) * u128::from(numer) * u128::from(rate) / u128::from(denom);
                    let frames = match usize::try_from(own) {
                        Ok(n) if n > 0 && n as u64 <= MAX_PACKET_FRAMES => n,
                        _ => self.last_frames,
                    };
                    if frames == 0 || self.info.channels == 0 {
                        continue;
                    }
                    crate::pcm::silence(usize::from(self.info.channels), frames)?
                }
                Err(SymError::IoError(_)) => {
                    self.ended = true;
                    continue;
                }
                Err(e) => return Err(map_err(e)),
            };
            let mut block = AudioBuffer { sample_rate: self.info.sample_rate, channels: block };
            if self.discard > 0 {
                let drop = (self.discard as usize).min(block.frames());
                for c in &mut block.channels {
                    c.drain(..drop);
                }
                self.discard -= drop as u64;
                if block.frames() == 0 {
                    continue;
                }
            }
            return Ok(Some(block));
        }
    }

    /// Continue from frame `frame`: an accurate seek lands at or before it, and the frames up to
    /// it are dropped.
    pub(crate) fn seek(&mut self, frame: u64) -> Result<u64> {
        // At or past a known end: nothing is left to decode.
        if self.frames_known && frame >= self.info.frames {
            self.ended = true;
            self.discard = 0;
            return Ok(self.info.frames);
        }
        guarded(|| {
            let (Some(reader), Some(decoder)) = (self.reader.as_mut(), self.decoder.as_mut()) else { return Ok(0) };
            let rate = u64::from(self.info.sample_rate.max(1));
            // Timestamps count `numer / denom` seconds; without a time base they count frames.
            let (numer, denom) = self.time_base.map_or((1, rate), |tb| (u64::from(tb.numer.get()), u64::from(tb.denom.get())));
            let to_ts = |f: u64| u128::from(f) * u128::from(denom) / (u128::from(numer) * u128::from(rate));
            let ts = i64::try_from(to_ts(frame)).unwrap_or(i64::MAX);
            let seeked = match reader.seek(SeekMode::Accurate, SeekTo::Timestamp { ts: Timestamp::new(ts), track_id: self.track_id }) {
                Ok(s) => s,
                // Past the end of a stream whose length is unknown: the stream is over.
                Err(SymError::SeekError(_)) => {
                    self.ended = true;
                    return Ok(frame);
                }
                Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                    self.ended = true;
                    return Ok(frame);
                }
                Err(e) => return Err(map_err(e)),
            };
            decoder.reset();
            // the packet before the seek says nothing about the length of one lost after it
            self.last_frames = 0;
            let actual = u64::try_from(seeked.actual_ts.get()).unwrap_or(0);
            let actual = u64::try_from(u128::from(actual) * u128::from(numer) * u128::from(rate) / u128::from(denom)).unwrap_or(u64::MAX);
            self.discard = frame.saturating_sub(actual);
            self.ended = false;
            Ok(frame)
        })
    }
}

/// The codec parameters of each audio track symphonia finds, for a track list.
pub(crate) fn tracks(bytes: Cow<'_, [u8]>, detected: Option<Container>) -> Result<Vec<(CodecId, u32, u16)>> {
    guarded(move || {
        let reader = match open(bytes, detected)? {
            Opened::Reader(reader, _) => reader,
            Opened::Empty(info) => return Ok(vec![(info.codec, info.sample_rate, info.channels)]),
        };
        Ok(audio_tracks(reader.as_ref())
            .filter_map(audio_params)
            .map(|p| (codec_id_of(p.codec), p.sample_rate.unwrap_or(0), p.channels.as_ref().map_or(0, |c| u16::try_from(c.count()).unwrap_or(0))))
            .collect())
    })
}

#[cfg(test)]
mod tests {
    use super::super::id3v2_len;
    use super::map::{container_of, sample_format_of};
    use super::precheck::{FirstMarker, MAX_AUDIO_SCORE_ATTEMPTS, ScoreBudget, audio_sync_at, first_marker_with_budget};
    use super::*;
    use crate::SampleFormat;
    use symphonia::core::codecs::audio::well_known::CODEC_ID_PCM_ALAW;
    use symphonia::core::codecs::audio::well_known::{
        CODEC_ID_AAC, CODEC_ID_ADPCM_MS, CODEC_ID_ALAC, CODEC_ID_FLAC, CODEC_ID_MP3, CODEC_ID_PCM_F32LE, CODEC_ID_PCM_F64LE, CODEC_ID_PCM_S16LE,
        CODEC_ID_VORBIS,
    };
    use symphonia::core::codecs::audio::{AudioCodecId, AudioCodecParameters};

    /// Build codec parameters for a test, optionally declaring a bit depth.
    fn params(codec: AudioCodecId, bits_per_sample: Option<u32>) -> AudioCodecParameters {
        let mut params = AudioCodecParameters::new();
        params.for_codec(codec);
        if let Some(bits) = bits_per_sample {
            params.with_bits_per_sample(bits);
        }
        params
    }

    /// symphonia's codec ids map to ours, PCM by its id range.
    #[test]
    fn codecs_map_to_codec_ids() {
        assert_eq!(codec_id_of(CODEC_ID_FLAC), CodecId::Flac);
        assert_eq!(codec_id_of(CODEC_ID_MP3), CodecId::Mp3);
        assert_eq!(codec_id_of(CODEC_ID_ALAC), CodecId::Alac);
        assert_eq!(codec_id_of(CODEC_ID_VORBIS), CodecId::Vorbis);
        assert_eq!(codec_id_of(CODEC_ID_PCM_S16LE), CodecId::Pcm);
        assert_eq!(codec_id_of(CODEC_ID_PCM_F64LE), CodecId::Pcm);
        assert_eq!(codec_id_of(CODEC_ID_ADPCM_MS), CodecId::Adpcm);
        assert_eq!(codec_id_of(CODEC_ID_PCM_ALAW), CodecId::ALaw);
    }

    /// The reader names the container; the detection refines MP4 into QuickTime and Matroska into WebM.
    #[test]
    fn readers_map_to_containers() {
        assert_eq!(container_of("isomp4", None), Container::Mp4);
        assert_eq!(container_of("isomp4", Some(Container::QuickTime)), Container::QuickTime);
        assert_eq!(container_of("matroska", Some(Container::WebM)), Container::WebM);
        assert_eq!(container_of("mp3", None), Container::MpegAudio);
        assert_eq!(container_of("wave", Some(Container::Wav)), Container::Wav);
    }

    /// PCM and FLAC keep their declared bit depth; lossy codecs report `Compressed` regardless of their decoded
    /// width.
    #[test]
    fn sample_formats_keep_their_depth_and_lossy_marker() {
        assert_eq!(sample_format_of(&params(CODEC_ID_PCM_S16LE, Some(16))), SampleFormat::Int16);
        assert_eq!(sample_format_of(&params(CODEC_ID_FLAC, Some(24))), SampleFormat::Int24);
        assert_eq!(sample_format_of(&params(CODEC_ID_FLAC, Some(8))), SampleFormat::Int8);
        assert_eq!(sample_format_of(&params(CODEC_ID_FLAC, Some(32))), SampleFormat::Int32);
        assert_eq!(sample_format_of(&params(CODEC_ID_PCM_F32LE, Some(32))), SampleFormat::Float32);
        assert_eq!(sample_format_of(&params(CODEC_ID_PCM_F64LE, Some(64))), SampleFormat::Float64);
        // Lossy codecs report no fixed bit depth, whatever their decoded sample format is.
        assert_eq!(sample_format_of(&params(CODEC_ID_AAC, Some(16))), SampleFormat::Compressed);
        assert_eq!(sample_format_of(&params(CODEC_ID_ADPCM_MS, Some(4))), SampleFormat::Compressed);
        assert_eq!(sample_format_of(&params(CODEC_ID_ALAC, None)), SampleFormat::Compressed);
    }

    /// Decoder options keep gapless trimming enabled so the encoder delay and padding reported by the demuxer are
    /// removed.
    #[test]
    fn gapless_decoding_stays_enabled() {
        assert!(decoder_options().gapless);
    }

    /// A complete MPEG frame header is a candidate; loose or unregistered sync pairs are rejected before scoring,
    /// while a registered but truncated header is rejected by the reader's score after consuming a budget slot.
    #[test]
    fn only_the_readers_own_audio_markers_are_candidates() {
        /// Whether the preflight would score `b` as an audio candidate at offset 0.
        fn candidate(b: &[u8]) -> bool {
            let mut stream = None;
            let mut budget = ScoreBudget::new();
            matches!(audio_sync_at(b, 0, &mut stream, &mut budget), Ok(true))
        }
        let mut mp3_frame = vec![0u8; 417];
        mp3_frame[..4].copy_from_slice(&[0xFF, 0xFB, 0x90, 0x00]); // MPEG-1 Layer III, 128 kbps, 44.1 kHz
        assert!(candidate(&[&mp3_frame[..], &mp3_frame].concat()));
        for false_sync in [vec![0xFF, 0xE0, 0x00], vec![0xFF, 0xF0, 0x00], vec![0xFF, 0xFB, 0xF0, 0x00]] {
            assert!(!candidate(&false_sync), "{false_sync:?} is not a probe candidate");
        }
    }

    /// A prefix made of false MPEG sync markers must not ask the readers to score every byte
    /// position: the per-scan budget caps the expensive work. A cheap container marker reached
    /// before the next audio candidate is still found, but once the cap is spent an audio
    /// candidate stops the scan so later payload magic cannot be preflight-authoritative.
    #[test]
    fn a_prefix_crowded_with_sync_markers_scores_only_the_budget() {
        let mut crowded = vec![0u8; 1 << 16];
        for pair in crowded.as_chunks_mut::<2>().0 {
            pair.copy_from_slice(&[0xFF, 0xE2]);
        }
        let mut budget = ScoreBudget::new();
        assert!(matches!(first_marker_with_budget(&crowded, 0, &mut budget), Ok(None)));
        assert_eq!(budget.used, MAX_AUDIO_SCORE_ATTEMPTS, "no candidate after the cap is scored");

        // At the cap boundary, with no later audio candidate, the cheap marker is still found.
        let mut at_cap: Vec<u8> = (0..MAX_AUDIO_SCORE_ATTEMPTS).flat_map(|_| [0xFFu8, 0xE2]).collect();
        at_cap.extend_from_slice(b"fLaC");
        let mut budget = ScoreBudget::new();
        assert!(
            matches!(first_marker_with_budget(&at_cap, 0, &mut budget), Ok(Some(FirstMarker::Flac(_)))),
            "a container marker reached before the next audio candidate is still found"
        );
        assert_eq!(budget.used, MAX_AUDIO_SCORE_ATTEMPTS);

        // One more false candidate between the cap and the magic makes the scan stop unchecked:
        // the same magic could instead sit in a valid audio payload.
        let mut after_candidate = at_cap[..2 * MAX_AUDIO_SCORE_ATTEMPTS].to_vec();
        after_candidate.extend_from_slice(&[0xFF, 0xE2]);
        after_candidate.extend_from_slice(b"fLaC");
        let mut budget = ScoreBudget::new();
        assert!(matches!(first_marker_with_budget(&after_candidate, 0, &mut budget), Ok(None)), "an audio candidate after the cap stops the scan");
        assert_eq!(budget.used, MAX_AUDIO_SCORE_ATTEMPTS);
    }

    /// ID3v2 sizes are read in their 7-bit syncsafe form; consecutive tags are skipped by their declared sizes, and
    /// the v2.3 reserved footer bit does not extend a tag.
    #[test]
    fn leading_id3_tags_are_skipped_by_their_declared_size() {
        let tag = [&b"ID3\x04\0\0\0\0\0\x05"[..], b"hello"].concat();
        assert_eq!(skip_id3v2(&[&tag[..], &tag[..]].concat()), 30);
        assert_eq!(skip_id3v2(b"fLaC"), 0);
        // The 10-byte footer flag exists only in v2.4; the same bit in v2.3 is reserved.
        assert_eq!(id3v2_len(b"ID3\x03\0\x10\0\0\0\0"), Some(10));
        assert_eq!(id3v2_len(b"ID3\x04\0\x10\0\0\0\0"), Some(20));
    }
}
