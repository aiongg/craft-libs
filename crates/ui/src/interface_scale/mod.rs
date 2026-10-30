//! The interface scale: how many device pixels one egui point covers.
//!
//! The setting is egui's zoom factor, `pixels_per_point = native * zoom_factor`. `Auto` follows
//! the display's native scale; a percent is absolute, so 100 % is one device pixel per point on
//! every display. It is independent of Size and Text size, which change point values.

use egui::Context;

mod fit;
mod options;

pub use fit::Fit;
pub use options::InterfaceScaleLabels;

/// The interface scale a user chose. Stored as the string `"auto"` or `"75"` .. `"300"`
/// (see [`InterfaceScale::code`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum InterfaceScale {
    /// Follows the display's native scale, fractional scales included.
    #[default]
    Auto,
    /// An absolute percent: 100 is one device pixel per point. Built by
    /// [`InterfaceScale::percent`], so it is always within `MIN..=MAX`.
    Percent(Pct),
}

/// A percent within [`InterfaceScale::MIN`]..=[`InterfaceScale::MAX`]; only
/// [`InterfaceScale::percent`] makes one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Pct(u16);

impl Pct {
    /// The percent, e.g. 125.
    pub fn get(self) -> u16 {
        self.0
    }
}

/// A zoom change smaller than this is not written, so `apply` is cheap every frame.
const ZOOM_EPSILON: f32 = 1e-3;

impl InterfaceScale {
    /// Smallest accepted percent.
    pub const MIN: u16 = 50;
    /// Largest accepted percent.
    pub const MAX: u16 = 300;
    /// The percents a picker offers.
    pub const STEPS: [u16; 16] = [75, 80, 85, 90, 95, 100, 105, 110, 115, 120, 125, 150, 175, 200, 250, 300];

    /// A fixed percent, clamped to `MIN..=MAX`.
    pub fn percent(p: u16) -> InterfaceScale {
        InterfaceScale::Percent(Pct(p.clamp(Self::MIN, Self::MAX)))
    }

    /// The stored string: `"auto"` or the percent, e.g. `"125"`.
    pub fn code(self) -> String {
        match self {
            InterfaceScale::Auto => "auto".to_owned(),
            InterfaceScale::Percent(p) => p.get().to_string(),
        }
    }

    /// Reads a stored string. A number is kept as given (clamped to `MIN..=MAX`), including values
    /// off [`STEPS`](Self::STEPS); anything else is `None`.
    pub fn from_code(code: &str) -> Option<InterfaceScale> {
        if code == "auto" {
            return Some(InterfaceScale::Auto);
        }
        code.parse::<u16>().ok().map(InterfaceScale::percent)
    }

    /// Pixels per point the setting asks for: `Auto` is `native`, `Percent(p)` is `p / 100`.
    /// An unknown `native` (before egui's first frame) is 1.
    pub fn pixels_per_point(self, native: Option<f32>) -> f32 {
        match self {
            InterfaceScale::Auto => native.unwrap_or(1.0),
            InterfaceScale::Percent(p) => f32::from(p.get()) / 100.0,
        }
    }

    /// The egui zoom factor that gives [`pixels_per_point`](Self::pixels_per_point): the wanted
    /// value divided by `native` (1 when unknown).
    pub fn zoom_factor(self, native: Option<f32>) -> f32 {
        self.pixels_per_point(native) / native.unwrap_or(1.0)
    }

    /// Sets the context's zoom factor when it differs from the wanted one by more than 1e-3.
    /// Call every frame: the native scale can change when the window moves to another display,
    /// and is unknown on the first frame.
    pub fn apply(self, ctx: &Context) {
        self.sync(ctx);
    }

    /// `apply`, reporting whether it wrote the zoom factor.
    pub(crate) fn sync(self, ctx: &Context) -> bool {
        let wanted = self.zoom_factor(ctx.native_pixels_per_point());
        if (ctx.zoom_factor() - wanted).abs() > ZOOM_EPSILON {
            ctx.set_zoom_factor(wanted);
            true
        } else {
            false
        }
    }

    /// Turns egui's Ctrl +/- zoom off, so the shortcuts can zoom the canvas. Apps call it once;
    /// the gallery does not.
    pub fn disable_keyboard_zoom(ctx: &Context) {
        ctx.options_mut(|o| o.zoom_with_keyboard = false);
    }

    /// Converts a zoom factor relative to the native scale (DesignCraft's `ui_scale`, 1.0 is
    /// native) into a setting: 1.0 is `Auto`, anything else the nearest step of
    /// `factor * native * 100`. A non-finite factor is `Auto`.
    pub fn from_relative(factor: f32, native: Option<f32>) -> InterfaceScale {
        if !factor.is_finite() || (factor - 1.0).abs() <= ZOOM_EPSILON {
            return InterfaceScale::Auto;
        }
        let wanted = factor * native.unwrap_or(1.0) * 100.0;
        let nearest =
            Self::STEPS.iter().copied().min_by(|a, b| (f32::from(*a) - wanted).abs().total_cmp(&(f32::from(*b) - wanted).abs())).unwrap_or(100);
        InterfaceScale::percent(nearest)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for InterfaceScale {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.code())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for InterfaceScale {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let code = <std::borrow::Cow<'de, str>>::deserialize(deserializer)?;
        InterfaceScale::from_code(&code).ok_or_else(|| serde::de::Error::custom(format!("not an interface scale: {code:?}")))
    }
}

#[cfg(test)]
mod tests;
