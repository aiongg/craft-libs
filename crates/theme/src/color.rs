//! sRGB, OKLab and OKLCH colour with conversions and gamut mapping.
//!
//! Conversions use Björn Ottosson's OKLab matrices in `f64`, so an sRGB colour survives a round
//! trip through OKLCH unchanged.

/// An sRGB colour with 8-bit channels and unmultiplied alpha.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Rgba(pub [u8; 4]);

/// A colour in OKLab: lightness `l` in 0..=1, opponent axes `a` (green-red) and `b` (blue-yellow).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Oklab {
    /// Perceptual lightness, 0 (black) to 1 (white).
    pub l: f64,
    /// Green-red axis.
    pub a: f64,
    /// Blue-yellow axis.
    pub b: f64,
}

/// A colour in OKLCH: OKLab in polar form. Used for building ramps and moving lightness.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Oklch {
    /// Perceptual lightness, 0 (black) to 1 (white).
    pub l: f64,
    /// Chroma, 0 for greys; sRGB tops out near 0.32.
    pub c: f64,
    /// Hue angle in degrees, 0..360.
    pub h: f64,
}

/// Linear-light sRGB, each channel 0..=1 when in gamut.
type Linear = [f64; 3];

/// Slack when testing whether a linear channel is inside 0..=1.
const GAMUT_EPS: f64 = 1e-6;

impl Rgba {
    /// Opaque black.
    pub const BLACK: Rgba = Rgba::rgb(0, 0, 0);
    /// Opaque white.
    pub const WHITE: Rgba = Rgba::rgb(255, 255, 255);
    /// Fully transparent.
    pub const TRANSPARENT: Rgba = Rgba([0, 0, 0, 0]);

    /// An opaque colour from channels.
    pub const fn rgb(r: u8, g: u8, b: u8) -> Rgba {
        Rgba([r, g, b, 255])
    }

    /// Parses `#RRGGBB` or `#RRGGBBAA` (case-insensitive, `#` required).
    pub fn from_hex(s: &str) -> Option<Rgba> {
        let digits = s.strip_prefix('#')?;
        if !digits.is_ascii() || !(digits.len() == 6 || digits.len() == 8) {
            return None;
        }
        let mut out = [255u8; 4];
        for (i, slot) in out.iter_mut().enumerate().take(digits.len() / 2) {
            *slot = u8::from_str_radix(digits.get(i * 2..i * 2 + 2)?, 16).ok()?;
        }
        Some(Rgba(out))
    }

    /// Lowercase `#rrggbb`, or `#rrggbbaa` when not opaque.
    pub fn to_hex(self) -> String {
        let [r, g, b, a] = self.0;
        if a == 255 { format!("#{r:02x}{g:02x}{b:02x}") } else { format!("#{r:02x}{g:02x}{b:02x}{a:02x}") }
    }

    /// The alpha channel, 0..=255.
    pub const fn a(self) -> u8 {
        self.0[3]
    }

    /// Linear-light channels (alpha ignored).
    pub fn to_linear(self) -> [f64; 3] {
        [srgb_to_linear(self.0[0]), srgb_to_linear(self.0[1]), srgb_to_linear(self.0[2])]
    }

    /// This colour in OKLab (alpha ignored).
    pub fn to_oklab(self) -> Oklab {
        Oklab::from_linear(self.to_linear())
    }

    /// This colour in OKLCH (alpha ignored).
    pub fn to_oklch(self) -> Oklch {
        self.to_oklab().to_oklch()
    }

    /// The same colour with a different alpha.
    pub const fn with_alpha(self, a: u8) -> Rgba {
        Rgba([self.0[0], self.0[1], self.0[2], a])
    }
}

/// egui's colours are premultiplied, so a translucent one comes back to its straight channels only
/// as closely as 8 bits allow, and a fully transparent one is transparent black.
#[cfg(feature = "ecolor")]
impl From<ecolor::Color32> for Rgba {
    fn from(c: ecolor::Color32) -> Rgba {
        Rgba(c.to_srgba_unmultiplied())
    }
}

/// Decodes one sRGB channel to linear light.
pub fn srgb_to_linear(v: u8) -> f64 {
    let c = f64::from(v) / 255.0;
    if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
}

/// Encodes linear light as an sRGB value in 0..=1 (not clamped).
pub fn linear_to_srgb(v: f64) -> f64 {
    if v <= 0.003_130_8 { v * 12.92 } else { 1.055 * v.powf(1.0 / 2.4) - 0.055 }
}

fn to_u8(v: f64) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

fn in_gamut(lin: Linear) -> bool {
    lin.iter().all(|&c| (-GAMUT_EPS..=1.0 + GAMUT_EPS).contains(&c))
}

impl Oklab {
    /// From linear-light sRGB.
    pub fn from_linear([r, g, b]: [f64; 3]) -> Oklab {
        let l = (0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b).cbrt();
        let m = (0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b).cbrt();
        let s = (0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b).cbrt();
        Oklab {
            l: 0.210_454_255_3 * l + 0.793_617_785_0 * m - 0.004_072_046_8 * s,
            a: 1.977_998_495_1 * l - 2.428_592_205_0 * m + 0.450_593_709_9 * s,
            b: 0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766_0 * s,
        }
    }

    /// To linear-light sRGB; channels may fall outside 0..=1 for out-of-gamut colours.
    pub fn to_linear(self) -> [f64; 3] {
        let l = (self.l + 0.396_337_777_4 * self.a + 0.215_803_757_3 * self.b).powi(3);
        let m = (self.l - 0.105_561_345_8 * self.a - 0.063_854_172_8 * self.b).powi(3);
        let s = (self.l - 0.089_484_177_5 * self.a - 1.291_485_548_0 * self.b).powi(3);
        [
            4.076_741_662_1 * l - 3.307_711_591_3 * m + 0.230_969_929_2 * s,
            -1.268_438_004_6 * l + 2.609_757_401_1 * m - 0.341_319_396_5 * s,
            -0.004_196_086_3 * l - 0.703_418_614_7 * m + 1.707_614_701_0 * s,
        ]
    }

    /// Polar form. Hue is 0 for exact greys.
    pub fn to_oklch(self) -> Oklch {
        let c = self.a.hypot(self.b);
        let h = if c < 1e-9 { 0.0 } else { self.b.atan2(self.a).to_degrees().rem_euclid(360.0) };
        Oklch { l: self.l, c, h }
    }
}

impl Oklch {
    /// This colour in OKLab.
    pub fn to_oklab(self) -> Oklab {
        let (sin, cos) = self.h.to_radians().sin_cos();
        Oklab { l: self.l, a: self.c * cos, b: self.c * sin }
    }

    /// Whether the colour is inside the sRGB gamut.
    pub fn in_gamut(self) -> bool {
        in_gamut(self.to_oklab().to_linear())
    }

    /// Maps into sRGB by reducing chroma at fixed lightness and hue (binary search for the
    /// largest in-gamut chroma, as in CSS Color 4 without its final clip). Lightness is clamped
    /// to 0..=1 first, so the result is always in gamut.
    pub fn gamut_map(self) -> Oklch {
        let l = self.l.clamp(0.0, 1.0);
        let mut out = Oklch { l, c: self.c.max(0.0), h: self.h };
        if out.in_gamut() {
            return out;
        }
        let (mut lo, mut hi) = (0.0, out.c);
        for _ in 0..40 {
            let mid = 0.5 * (lo + hi);
            if (Oklch { c: mid, ..out }).in_gamut() {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        out.c = lo;
        out
    }

    /// Gamut-maps and encodes as opaque sRGB.
    pub fn to_rgba(self) -> Rgba {
        let lin = self.gamut_map().to_oklab().to_linear();
        Rgba::rgb(to_u8(linear_to_srgb(lin[0])), to_u8(linear_to_srgb(lin[1])), to_u8(linear_to_srgb(lin[2])))
    }
}

impl From<Rgba> for Oklch {
    fn from(c: Rgba) -> Oklch {
        c.to_oklch()
    }
}
