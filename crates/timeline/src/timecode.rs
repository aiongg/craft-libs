//! Time as text: SMPTE timecode (drop and non-drop), frames, feet + frames, audio samples and
//! seconds, formatted and parsed the way Premiere's timecode fields do.
//!
//! Timecode is only a display of [`Tick`]s. The words around the values (the display names in a
//! menu, the error notices) are the UI's: [`TimeDisplay::label`] gives the English name for
//! `craft_ui::words!` tables.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use crate::rates::{FrameRate, SampleRate};
use crate::time::Tick;

/// How time is shown (Premiere: Timecode, Feet + Frames 16 mm / 35 mm, Frames, Audio Samples;
/// After Effects' frame numbers; seconds for scripts).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TimeDisplay {
    /// `HH:MM:SS:FF`, `HH;MM;SS;FF` for drop-frame; three frame digits from 100 fps.
    #[default]
    Timecode,
    /// The frame number, counted from [`TimeFormat::frame_offset`] (After Effects' Start
    /// Numbering Frames At).
    Frames,
    /// `feet+frames` at 40 frames a foot.
    Feet16,
    /// `feet+frames` at 16 frames a foot.
    Feet35,
    /// `HH:MM:SS:sssss`, the samples past the second.
    AudioSamples,
    /// Seconds with three decimals.
    Seconds,
}

impl TimeDisplay {
    /// The order of Premiere's Time Display menu.
    pub const ALL: [TimeDisplay; 6] =
        [TimeDisplay::Timecode, TimeDisplay::Feet35, TimeDisplay::Feet16, TimeDisplay::Frames, TimeDisplay::AudioSamples, TimeDisplay::Seconds];

    /// The English menu name ("Feet + Frames 35mm"), the key of its word.
    // filmcraft: crates/time/src/lib.rs:344.
    pub fn label(self) -> &'static str {
        todo!()
    }
}

/// Everything a time display needs: the rate, drop-frame, the display, the sample rate (for
/// [`TimeDisplay::AudioSamples`]) and the first frame's number.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TimeFormat {
    pub rate: FrameRate,
    pub drop_frame: bool,
    pub display: TimeDisplay,
    pub sample_rate: SampleRate,
    /// Added to the frame number in [`TimeDisplay::Frames`] (0 or 1 in After Effects).
    pub frame_offset: i64,
}

impl TimeFormat {
    /// Timecode at `rate`, non-drop.
    pub fn timecode(rate: FrameRate) -> Self {
        Self { rate, ..Self::default() }
    }

    pub fn drop_frame(mut self, on: bool) -> Self {
        self.drop_frame = on;
        self
    }

    pub fn display(mut self, display: TimeDisplay) -> Self {
        self.display = display;
        self
    }
}

/// SMPTE fields of a frame count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TimecodeFields {
    pub negative: bool,
    pub hours: i64,
    pub minutes: i64,
    pub seconds: i64,
    pub frames: i64,
}

/// Why typed time was refused. The UI shows it as a notice through its own words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimecodeError {
    /// Nothing typed.
    Empty,
    /// Not a time in the field's display (`"1:x"`); carries the input.
    NotATime(String),
}

impl std::fmt::Display for TimecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimecodeError::Empty => f.write_str("empty timecode"),
            TimecodeError::NotATime(s) => write!(f, "not a timecode: `{s}`"),
        }
    }
}

impl std::error::Error for TimecodeError {}

/// The SMPTE fields of `frame` at `rate`. Drop-frame (when asked and the rate supports it) skips
/// `base / 15` labels at the start of each minute except every tenth: 2 at 29.97, 4 at 59.94.
// filmcraft: crates/time/src/lib.rs:363 `frames_to_fields` (and `df_params` above it).
pub fn frames_to_fields(frame: i64, rate: FrameRate, drop_frame: bool) -> TimecodeFields {
    todo!()
}

/// The frame count of SMPTE fields; the inverse of [`frames_to_fields`], in i128 so nominal
/// drop-frame labels past `i64` still give a frame that fits.
// filmcraft: crates/time/src/lib.rs:386 `fields_to_frames`.
pub fn fields_to_frames(fields: TimecodeFields, rate: FrameRate, drop_frame: bool) -> i64 {
    todo!()
}

/// `t` as text in `format`: timecode `00:00:10:00` (`;` separators for drop-frame, three frame
/// digits from a 100 fps base, a leading `-` for negative times); frames `250` (plus the
/// offset); feet + frames `15+10` (16 frames a foot for 35 mm, 40 for 16 mm); samples
/// `00:00:10:00000` (five digits of samples past the second); seconds `10.000` (`-0.000` shown
/// as `0.000`).
// filmcraft: crates/time/src/lib.rs:399 `format_timecode_frames`, `:408` `format_time`.
pub fn format_time(t: Tick, format: &TimeFormat) -> String {
    todo!()
}

/// Typed time in `format` as a tick, the way Premiere's fields read it:
/// - `01:02:03:04`, `01;02;03;04`, `1.2.3.4` (any of `:;.,` separates fields);
/// - bare digits are right-aligned as `HHMMSSFF` (`1000` is 00:00:10:00);
/// - a leading `+` or `-` is relative to `current` (`+15` is 15 frames later, `-1.00` one
///   second earlier);
/// - in [`TimeDisplay::Frames`] a bare number is a frame number (less the offset); in
///   [`TimeDisplay::Seconds`] a decimal number of seconds; feet + frames reads `ft+ff`.
///
/// The result is the first tick of the frame.
// filmcraft: crates/time/src/lib.rs:450 `parse_timecode`; soundcraft: crates/time/src/format.rs:90
// `parse_position` (the other displays).
pub fn parse_time(input: &str, format: &TimeFormat, current: Tick) -> Result<Tick, TimecodeError> {
    todo!()
}
