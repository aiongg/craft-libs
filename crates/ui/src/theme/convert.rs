//! Token values to egui values. Every conversion the bridge makes is here.

use craft_theme::{Elevation, FontSlot, Rgba, TypeStyle};
use egui::{Color32, Context, CornerRadius, FontFamily, FontId, Margin, Shadow, Stroke};

use crate::fonts;

/// A token value with a direct egui counterpart.
pub trait ToEgui {
    type Egui;
    fn to_egui(self) -> Self::Egui;
}

impl ToEgui for Rgba {
    type Egui = Color32;
    fn to_egui(self) -> Color32 {
        let [r, g, b, a] = self.0;
        Color32::from_rgba_unmultiplied(r, g, b, a)
    }
}

/// A black shadow at the elevation's alpha.
impl ToEgui for Elevation {
    type Egui = Shadow;
    fn to_egui(self) -> Shadow {
        let [x, y] = self.offset;
        Shadow {
            offset: [to_i8(x), to_i8(y)],
            blur: to_u8(self.blur),
            spread: to_u8(self.spread),
            color: Color32::from_black_alpha(to_u8(self.alpha * 255.0)),
        }
    }
}

/// A radius in points, the same at every corner. [`craft_theme::Radius::Full`] becomes 255, which
/// egui clamps to half the shape's height.
pub fn corner_radius(px: f32) -> CornerRadius {
    CornerRadius::same(to_u8(px))
}

/// The same margin on every side.
pub fn margin(px: f32) -> Margin {
    Margin::same(to_i8(px))
}

/// A line `width` points wide.
pub fn stroke(width: f32, color: Rgba) -> Stroke {
    Stroke::new(width, color.to_egui())
}

fn to_u8(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

fn to_i8(v: f32) -> i8 {
    v.round().clamp(-128.0, 127.0) as i8
}

/// The egui family each [`FontSlot`] draws with.
///
/// The UI slot is egui's proportional family and the mono slot its monospace one. The semibold
/// slot is the [`fonts::SEMIBOLD`] family when the app's font stack has it, else the
/// proportional family, so text never lands in a family with no faces.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FontFamilies {
    pub ui: FontFamily,
    pub ui_semibold: FontFamily,
    pub mono: FontFamily,
}

impl Default for FontFamilies {
    fn default() -> Self {
        Self { ui: FontFamily::Proportional, ui_semibold: FontFamily::Proportional, mono: FontFamily::Monospace }
    }
}

impl FontFamilies {
    /// The families for the font stack last installed in `ctx` with [`crate::fonts`].
    pub fn installed(ctx: &Context) -> Self {
        let families = fonts::installed_families(ctx);
        let semibold = families.iter().flat_map(|f| f.iter()).find(|f| matches!(f, FontFamily::Name(name) if &**name == fonts::SEMIBOLD));
        match semibold {
            Some(semibold) => Self { ui_semibold: semibold.clone(), ..Self::default() },
            None => Self::default(),
        }
    }

    pub fn family(&self, slot: FontSlot) -> &FontFamily {
        match slot {
            FontSlot::Ui => &self.ui,
            FontSlot::UiSemibold => &self.ui_semibold,
            FontSlot::Mono => &self.mono,
        }
    }

    /// The font of a type style. egui's `FontId` has no line height: rows take the face's own,
    /// and components that need the token's read [`TypeStyle::line_height`].
    pub fn font_id(&self, style: TypeStyle) -> FontId {
        FontId::new(style.size, self.family(style.font).clone())
    }
}
