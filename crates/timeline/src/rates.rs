//! Frame rates, sample rates and pixel aspect ratios, with the presets the settings dialogs and
//! menus offer (Premiere's sequence settings, After Effects' composition settings, Pro Tools'
//! session rates).

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use crate::time::Tick;

/// A frame rate as an exact rational, `num / den` frames a second. Rates come from files, so
/// every frame computation goes through [`sane`](Self::sane) and a zero or negative rate counts
/// as the default (29.97).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct FrameRate {
    pub num: i64,
    pub den: i64,
}

impl Default for FrameRate {
    fn default() -> Self {
        Self::FPS_29_97
    }
}

impl FrameRate {
    pub const FPS_23_976: FrameRate = FrameRate { num: 24000, den: 1001 };
    pub const FPS_24: FrameRate = FrameRate { num: 24, den: 1 };
    pub const FPS_25: FrameRate = FrameRate { num: 25, den: 1 };
    pub const FPS_29_97: FrameRate = FrameRate { num: 30000, den: 1001 };
    pub const FPS_30: FrameRate = FrameRate { num: 30, den: 1 };
    pub const FPS_48: FrameRate = FrameRate { num: 48, den: 1 };
    pub const FPS_50: FrameRate = FrameRate { num: 50, den: 1 };
    pub const FPS_59_94: FrameRate = FrameRate { num: 60000, den: 1001 };
    pub const FPS_60: FrameRate = FrameRate { num: 60, den: 1 };
    pub const FPS_119_88: FrameRate = FrameRate { num: 120000, den: 1001 };
    pub const FPS_120: FrameRate = FrameRate { num: 120, den: 1 };

    /// The rates a frame-rate menu offers, in Premiere's order.
    pub const COMMON: [FrameRate; 11] = [
        Self::FPS_23_976,
        Self::FPS_24,
        Self::FPS_25,
        Self::FPS_29_97,
        Self::FPS_30,
        Self::FPS_48,
        Self::FPS_50,
        Self::FPS_59_94,
        Self::FPS_60,
        Self::FPS_119_88,
        Self::FPS_120,
    ];

    /// `num / den` reduced by their greatest common divisor.
    // filmcraft: crates/time/src/lib.rs:198 (i128 division by the gcd, at least 1).
    pub fn new(num: i64, den: i64) -> Self {
        todo!()
    }

    /// The [`COMMON`](Self::COMMON) rate within 0.005 fps of `fps` (a container's average rate),
    /// else `round(fps × 1000) / 1000`.
    // filmcraft: crates/time/src/lib.rs:204.
    pub fn from_f64(fps: f64) -> Self {
        todo!()
    }

    /// This rate when both parts are positive, else the default.
    pub fn sane(self) -> FrameRate {
        if self.num > 0 && self.den > 0 { self } else { FrameRate::default() }
    }

    pub fn as_f64(self) -> f64 {
        self.num as f64 / self.den as f64
    }

    /// One frame's exact duration (floored only for exotic rates); saturates at the tick range.
    // filmcraft: crates/time/src/lib.rs:225 (`TICKS_PER_SECOND * den / num` in i128).
    pub fn frame_duration(self) -> Tick {
        todo!()
    }

    /// The index of the frame holding `t` (floor), saturated to the frame range.
    // filmcraft: crates/time/src/lib.rs:231 (`frame_index`: `t * num / (TICKS_PER_SECOND * den)`
    // with `div_euclid` in i128).
    pub fn frame_at(self, t: Tick) -> i64 {
        todo!()
    }

    /// The first tick of frame `f`, by ceiling division, so `frame_at(tick_of(f)) == f` holds
    /// for every rate with at least one tick per frame (37.516 fps included: flooring drifts each
    /// boundary back a tick and frame stepping stalls).
    // filmcraft: crates/time/src/lib.rs:244 (`frame_boundary`).
    pub fn tick_of(self, f: i64) -> Tick {
        todo!()
    }

    /// `t` moved down to its frame's first tick.
    // filmcraft: crates/time/src/lib.rs:249.
    pub fn snap(self, t: Tick) -> Tick {
        todo!()
    }

    /// `t` moved to the nearest frame boundary (the earlier one at the exact middle); both
    /// boundaries are compared in i128 before saturating.
    // filmcraft: crates/time/src/lib.rs:255.
    pub fn snap_nearest(self, t: Tick) -> Tick {
        todo!()
    }

    /// Frames counted per timecode second: 30 for 29.97, 24 for 23.976 (`num / den` rounded up,
    /// at least 1).
    // filmcraft: crates/time/src/lib.rs:267.
    pub fn timecode_base(self) -> i64 {
        todo!()
    }

    /// An NTSC (x/1001) rate.
    pub fn is_ntsc(self) -> bool {
        self.den == 1001
    }

    /// Whether drop-frame counting applies: NTSC rates with a base that is a multiple of 30
    /// (29.97, 59.94, 119.88).
    // filmcraft: crates/time/src/lib.rs:278.
    pub fn supports_drop_frame(self) -> bool {
        todo!()
    }

    /// The menu label without the unit: `24`, `23.976`, `29.97` (three decimals, trailing zeros
    /// and the point trimmed). The unit ("fps") is the menu's word.
    // filmcraft: crates/time/src/lib.rs:282.
    pub fn label(self) -> String {
        todo!()
    }
}

/// An audio sample rate in hertz.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct SampleRate(pub u32);

impl Default for SampleRate {
    fn default() -> Self {
        Self::HZ_48000
    }
}

impl SampleRate {
    pub const HZ_44100: SampleRate = SampleRate(44_100);
    pub const HZ_48000: SampleRate = SampleRate(48_000);

    /// The rates a session's sample-rate menu offers (SoundCraft `crates/time/src/lib.rs:46`).
    /// A video app offers [`VIDEO`](Self::VIDEO).
    pub const COMMON: [SampleRate; 8] = [
        SampleRate(44_100),
        SampleRate(48_000),
        SampleRate(88_200),
        SampleRate(96_000),
        SampleRate(176_400),
        SampleRate(192_000),
        SampleRate(352_800),
        SampleRate(384_000),
    ];

    /// The sequence sample rates FilmCraft offers (`panels/sequence_settings.rs:59`).
    pub const VIDEO: [SampleRate; 5] = [SampleRate(32_000), SampleRate(44_100), SampleRate(48_000), SampleRate(88_200), SampleRate(96_000)];

    pub fn hz(self) -> u32 {
        self.0
    }

    /// The time of sample `n` (exact for every rate in [`COMMON`](Self::COMMON)).
    pub fn tick_of(self, n: i64) -> Tick {
        Tick::from_units(n, i64::from(self.0))
    }

    /// The sample holding `t` (floor).
    pub fn sample_at(self, t: Tick) -> i64 {
        t.to_units_floor(i64::from(self.0))
    }
}

/// A pixel aspect ratio, `num / den` (1/1 is square pixels).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct PixelAspect {
    pub num: u32,
    pub den: u32,
}

impl Default for PixelAspect {
    fn default() -> Self {
        Self::SQUARE
    }
}

impl PixelAspect {
    pub const SQUARE: PixelAspect = PixelAspect { num: 1, den: 1 };

    /// The ratio, 1.0 for a zero denominator.
    pub fn as_f64(self) -> f64 {
        if self.den == 0 { 1.0 } else { f64::from(self.num) / f64::from(self.den) }
    }

    /// The ratio as a menu shows it after its name: `1.0` for square pixels, else four decimals
    /// (`0.9091`). FilmCraft's labels are "Square Pixels (1.0)" and "Custom (0.9091)"; the words
    /// are the menu's.
    // filmcraft: crates/ui-egui/src/panels/sequence_settings.rs:204 `par_label`.
    pub fn ratio_label(self) -> String {
        todo!()
    }
}
