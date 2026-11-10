//! Sample entries (`stsd` children) and codec configuration records: parsing and serialization.

use crate::CodecId;
use crate::bitstream::BitReader;
use crate::isobmff::bytes::{BoxBuf, Cur, FourCc, boxes};
use crate::isobmff::error::{Error, Result};

/// One sample description (an entry of `stsd`).
#[derive(Clone, Debug, PartialEq)]
pub struct SampleEntry {
    /// Sample entry format (`avc1`, `mp4a`, `apch`, …).
    pub format: FourCc,
    /// Index into the `dref` of the track's `dinf` box (1-based; usually 1).
    pub data_reference_index: u16,
    #[allow(missing_docs)]
    pub codec: CodecConfig,
    /// Present for visual sample entries.
    pub video: Option<VideoParams>,
    /// Present for audio sample entries.
    pub audio: Option<AudioParams>,
    /// `btrt` box.
    pub bitrate: Option<BitRate>,
}

/// `btrt` box.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BitRate {
    /// Decoding buffer size in bytes.
    pub buffer_size: u32,
    /// Maximum bitrate in bits per second.
    pub max_bitrate: u32,
    /// Average bitrate in bits per second.
    pub avg_bitrate: u32,
}

/// Codec configuration decoded from the sample entry.
#[derive(Clone, Debug, PartialEq)]
pub enum CodecConfig {
    #[allow(missing_docs)]
    Avc(AvcConfig),
    #[allow(missing_docs)]
    Hevc(HevcConfig),
    #[allow(missing_docs)]
    Av1(Av1Config),
    #[allow(missing_docs)]
    Vp9(VpcConfig),
    /// Apple ProRes; fourcc is `apch`/`apcn`/`apcs`/`apco`/`ap4h`/`ap4x`.
    ProRes {
        #[allow(missing_docs)]
        fourcc: FourCc,
    },
    /// Motion JPEG (`jpeg`, `mjpa`, `mjpb`).
    Jpeg {
        #[allow(missing_docs)]
        fourcc: FourCc,
    },
    /// Avid DNxHD/DNxHR (`AVdn`, `AVdh`).
    Dnx {
        #[allow(missing_docs)]
        fourcc: FourCc,
    },
    /// Advanced Professional Video (`apv1` + `apvC`).
    Apv(ApvConfig),
    #[allow(missing_docs)]
    Aac(AacConfig),
    /// MPEG-1/2 audio (esds object type 0x69/0x6B, or QuickTime `.mp3` / `.mp2`). Usually layer
    /// III, but these name no layer: ffmpeg stores MP2 as `mp4a` + 0x6B. Each frame header has it,
    /// and [`crate::isobmff::Track::codec_params`] reports the layer the first frame names.
    Mp3,
    #[allow(missing_docs)]
    Pcm(PcmConfig),
    /// ALAC magic cookie (ALACSpecificConfig, 24 bytes, optionally followed by a channel layout).
    Alac {
        #[allow(missing_docs)]
        cookie: Vec<u8>,
    },
    #[allow(missing_docs)]
    Opus(OpusConfig),
    #[allow(missing_docs)]
    Flac(FlacConfig),
    /// `dac3` payload.
    Ac3 {
        #[allow(missing_docs)]
        dac3: Vec<u8>,
    },
    /// `dec3` payload.
    Eac3 {
        #[allow(missing_docs)]
        dec3: Vec<u8>,
    },
    #[allow(missing_docs)]
    Timecode(TimecodeConfig),
    /// Anything else; `raw` is the full sample entry payload (after the 8-byte box header).
    Unknown {
        #[allow(missing_docs)]
        fourcc: FourCc,
        #[allow(missing_docs)]
        raw: Vec<u8>,
    },
}

impl CodecConfig {
    /// The codec this configuration is for. `Mp3` covers every MPEG audio layer: these entries
    /// name none, and each frame header has it (ffmpeg stores MP2 as `mp4a` + 0x6B);
    /// [`crate::isobmff::Track::codec_params`] reads it from the first frame.
    pub fn codec_id(&self) -> CodecId {
        match self {
            CodecConfig::Avc(_) => CodecId::H264,
            CodecConfig::Hevc(_) => CodecId::Hevc,
            CodecConfig::Av1(_) => CodecId::Av1,
            CodecConfig::Vp9(_) => CodecId::Vp9,
            CodecConfig::ProRes { .. } => CodecId::ProRes,
            CodecConfig::Jpeg { .. } => CodecId::Mjpeg,
            CodecConfig::Aac(_) => CodecId::Aac,
            CodecConfig::Mp3 => CodecId::Mp3,
            CodecConfig::Pcm(_) => CodecId::Pcm,
            CodecConfig::Alac { .. } => CodecId::Alac,
            CodecConfig::Opus(_) => CodecId::Opus,
            CodecConfig::Flac(_) => CodecId::Flac,
            CodecConfig::Ac3 { .. } => CodecId::Ac3,
            CodecConfig::Eac3 { .. } => CodecId::Eac3,
            CodecConfig::Dnx { .. } => CodecId::Dnxhd,
            CodecConfig::Apv(_) => CodecId::Apv,
            CodecConfig::Timecode(_) | CodecConfig::Unknown { .. } => CodecId::Unknown,
        }
    }

    /// The codec configuration in Matroska's `CodecPrivate` form (see [`crate::CodecParams`]):
    /// the `OpusHead` built from `dOps`, `fLaC` + the `dfLa` metadata blocks, the other records
    /// as stored.
    pub fn extradata(&self) -> Vec<u8> {
        match self {
            CodecConfig::Avc(c) => c.to_bytes(),
            CodecConfig::Hevc(c) => c.to_bytes(),
            CodecConfig::Av1(c) => c.to_bytes(),
            CodecConfig::Vp9(c) => c.to_bytes(),
            CodecConfig::Apv(c) => c.record_bytes(),
            CodecConfig::Aac(a) => a.asc.clone(),
            CodecConfig::Alac { cookie } => cookie.clone(),
            CodecConfig::Opus(o) => o.opus_head(),
            CodecConfig::Flac(f) => [b"fLaC".as_slice(), &f.metadata_blocks].concat(),
            CodecConfig::Ac3 { dac3 } => dac3.clone(),
            CodecConfig::Eac3 { dec3 } => dec3.clone(),
            _ => Vec::new(),
        }
    }

    /// A short human-readable codec name.
    pub fn name(&self) -> &'static str {
        match self {
            CodecConfig::Avc(_) => "h264",
            CodecConfig::Hevc(_) => "hevc",
            CodecConfig::Av1(_) => "av1",
            CodecConfig::Vp9(_) => "vp9",
            CodecConfig::ProRes { .. } => "prores",
            CodecConfig::Jpeg { .. } => "mjpeg",
            CodecConfig::Dnx { .. } => "dnxhd",
            CodecConfig::Apv(_) => "apv",
            CodecConfig::Aac(_) => "aac",
            CodecConfig::Mp3 => "mp3",
            CodecConfig::Pcm(_) => "pcm",
            CodecConfig::Alac { .. } => "alac",
            CodecConfig::Opus(_) => "opus",
            CodecConfig::Flac(_) => "flac",
            CodecConfig::Ac3 { .. } => "ac3",
            CodecConfig::Eac3 { .. } => "eac3",
            CodecConfig::Timecode(_) => "timecode",
            CodecConfig::Unknown { .. } => "unknown",
        }
    }
}

mod audio;
mod parse;
mod video;
mod write;

pub use audio::{AacConfig, AudioParams, FlacConfig, OpusConfig, PcmConfig, TimecodeConfig, parse_asc};
pub(crate) use parse::parse_sample_entry;
pub use video::{
    ApvConfig, Av1Config, AvcConfig, CleanAperture, ColorInfo, FieldInfo, HevcConfig, HevcNalArray, MasteringDisplay, VideoParams, VpcConfig,
};

#[cfg(test)]
mod params_tests;
#[cfg(test)]
mod tests;
