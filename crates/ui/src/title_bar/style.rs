//! [`TitleBarStyle`]: the title bar's sizes and colours from the `title_bar` tokens.

use craft_theme::{Rgba, TypeRole};
use egui::Color32;

use crate::theme::{ToEgui, UiTokens};

/// The sizes and colours the bar, its title and its caption buttons paint with, from
/// [`TitleBarStyle::from_tokens`]. Painting code reads colours from here and nowhere else.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TitleBarStyle {
    /// Height of the bar: `title_bar.height`, the size preset's `bar.title`.
    pub height: f32,
    /// Least height with Adwaita caption buttons: `title_bar.height_adwaita`.
    pub height_adwaita: f32,
    /// End margin of a side without caption buttons: `title_bar.pad`.
    pub pad: f32,
    /// Least room between the title and what is beside it: `title_bar.title_gap`.
    pub title_gap: f32,
    /// Least gap that still shows an elided title: `title_bar.title_min_gap`.
    pub title_min_gap: f32,
    /// The title's type role: `title_bar.title_role`.
    pub title_role: TypeRole,
    /// Width of the resize zones along the window edges: `title_bar.edge`.
    pub edge: f32,
    /// Side of the resize zones at the window corners: [`CORNER_RESIZE`].
    pub corner: f32,
    /// The bar's background, and Breeze's glyph on its hovered or pressed disc: `title_bar.bar`.
    pub bar: Color32,
    /// The title: `title_bar.title`.
    pub title: Color32,
    /// The caption glyphs in the active window: `title_bar.glyph`.
    pub glyph: Color32,
    /// The caption glyphs in an inactive window: `title_bar.glyph_inactive`.
    pub glyph_inactive: Color32,
    /// The fill under the pointer: `title_bar.hover`.
    pub hover: Color32,
    /// The fill while a button is held down: `title_bar.pressed`.
    pub pressed: Color32,
    /// The hairline along the bar's bottom edge: `title_bar.separator`.
    pub separator: Color32,
    /// Windows' Close button under the pointer; held down it is 90 % of this.
    pub windows_close: Color32,
    /// The glyph on Windows' Close button.
    pub windows_close_glyph: Color32,
    /// Breeze's disc under a pressed button: the bar's colour 70 % of the way to
    /// [`glyph`](Self::glyph), with the bar's colour as the glyph on it. Derived, so it follows the
    /// theme's colours and keeps the glyph's 4.5:1 against the disc in high contrast.
    pub breeze_pressed: Color32,
    /// Breeze's Close button under the pointer.
    pub breeze_close_hover: Color32,
    /// The glyph on Breeze's hovered Close button.
    pub breeze_close_hover_glyph: Color32,
    /// Breeze's Close button held down.
    pub breeze_close_pressed: Color32,
    /// The glyph on Breeze's pressed Close button.
    pub breeze_close_pressed_glyph: Color32,
}

/// How far Breeze's pressed disc is from the bar toward the glyph colour.
const BREEZE_PRESSED_MIX: f32 = 0.7;

/// The bar height with libadwaita's buttons.
const ADWAITA_HEIGHT: f32 = 47.0;
/// The room kept between the title and the buttons before the title is cut.
const TITLE_MIN_GAP: f32 = 80.0;
/// Side of the resize zones at the window corners, larger than any corner radius.
pub const CORNER_RESIZE: f32 = 12.0;
/// The Windows Close button's hover fill.
const WINDOWS_CLOSE: Rgba = Rgba::rgb(0xC4, 0x2B, 0x1C);
/// The glyph on the Windows Close button's hover fill.
const WINDOWS_CLOSE_GLYPH: Rgba = Rgba::WHITE;
/// Breeze's Close button under the pointer.
const BREEZE_CLOSE_HOVER: Rgba = Rgba::rgb(0xFF, 0x98, 0xA2);
/// The glyph on Breeze's hovered Close button.
const BREEZE_CLOSE_HOVER_GLYPH: Rgba = Rgba::rgb(0x23, 0x26, 0x29);
/// Breeze's Close button held down.
const BREEZE_CLOSE_PRESSED: Rgba = Rgba::rgb(0x6D, 0x22, 0x29);
/// The glyph on Breeze's pressed Close button.
const BREEZE_CLOSE_PRESSED_GLYPH: Rgba = Rgba::WHITE;

/// `fg` laid over the opaque `bg`.
fn over(fg: Rgba, bg: Rgba) -> Rgba {
    let a = f64::from(fg.a()) / 255.0;
    let channel = |i: usize| (f64::from(fg.0[i]) * a + f64::from(bg.0[i]) * (1.0 - a)).round() as u8;
    Rgba::rgb(channel(0), channel(1), channel(2))
}

impl TitleBarStyle {
    /// The style for the installed theme.
    pub fn from_tokens(tokens: &UiTokens) -> Self {
        let (c, space) = (&tokens.color, &tokens.space);
        let bar = c.surface.chrome;
        Self {
            height: tokens.size.bar.title,
            height_adwaita: ADWAITA_HEIGHT,
            pad: space.pad.panel,
            title_gap: space.gap.section,
            title_min_gap: TITLE_MIN_GAP,
            title_role: TypeRole::Title,
            edge: space.gap.stack,
            corner: CORNER_RESIZE,
            bar: bar.to_egui(),
            title: c.text.muted.to_egui(),
            glyph: c.text.normal.to_egui(),
            glyph_inactive: c.text.muted.to_egui(),
            hover: over(c.state.hover, bar).to_egui(),
            pressed: over(c.state.pressed, bar).to_egui(),
            separator: c.line.divider.to_egui(),
            windows_close: WINDOWS_CLOSE.to_egui(),
            windows_close_glyph: WINDOWS_CLOSE_GLYPH.to_egui(),
            breeze_pressed: bar.to_egui().lerp_to_gamma(c.text.normal.to_egui(), BREEZE_PRESSED_MIX),
            breeze_close_hover: BREEZE_CLOSE_HOVER.to_egui(),
            breeze_close_hover_glyph: BREEZE_CLOSE_HOVER_GLYPH.to_egui(),
            breeze_close_pressed: BREEZE_CLOSE_PRESSED.to_egui(),
            breeze_close_pressed_glyph: BREEZE_CLOSE_PRESSED_GLYPH.to_egui(),
        }
    }
}

#[cfg(test)]
mod tests {
    use craft_theme::{Appearance, Contrast, Mode, Size, TextSize, builtins, resolve};

    use super::*;
    use crate::theme::FontFamilies;

    fn tokens(size: Size, mode: Mode) -> UiTokens {
        let theme = &builtins()[0];
        let appearance = Appearance { mode, contrast: Contrast::Standard, size, text_size: TextSize::default(), reduced_motion: false };
        UiTokens::new(resolve(theme, appearance, None).tokens, appearance, mode, FontFamilies::default())
    }

    #[test]
    fn the_style_reads_the_semantic_tokens() {
        for (size, height, pad) in [(Size::Compact, 32.0, 8.0), (Size::Comfortable, 38.0, 12.0)] {
            for mode in [Mode::Light, Mode::Dark] {
                let t = tokens(size, mode);
                let s = TitleBarStyle::from_tokens(&t);
                let c = &t.color;
                assert_eq!((s.height, s.height_adwaita, s.pad), (height, 47.0, pad));
                assert_eq!((s.bar, s.glyph), (c.surface.chrome.to_egui(), c.text.normal.to_egui()));
                assert_eq!(s.windows_close, Color32::from_rgb(0xC4, 0x2B, 0x1C));
            }
        }
    }
}
