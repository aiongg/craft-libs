//! [`TitleBarStyle`]: the title bar's sizes and colours from the `title_bar` tokens.

use craft_theme::TypeRole;
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
    /// Side of the resize zones at the window corners: `title_bar.corner`.
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
    /// Breeze's Close button under the pointer.
    pub breeze_close_hover: Color32,
    /// The glyph on Breeze's hovered Close button.
    pub breeze_close_hover_glyph: Color32,
    /// Breeze's Close button held down.
    pub breeze_close_pressed: Color32,
    /// The glyph on Breeze's pressed Close button.
    pub breeze_close_pressed_glyph: Color32,
}

impl TitleBarStyle {
    /// The style for the installed theme.
    pub fn from_tokens(tokens: &UiTokens) -> Self {
        let b = &tokens.component.title_bar;
        Self {
            height: b.height,
            height_adwaita: b.height_adwaita,
            pad: b.pad,
            title_gap: b.title_gap,
            title_min_gap: b.title_min_gap,
            title_role: b.title_role,
            edge: b.edge,
            corner: b.corner,
            bar: b.bar.to_egui(),
            title: b.title.to_egui(),
            glyph: b.glyph.to_egui(),
            glyph_inactive: b.glyph_inactive.to_egui(),
            hover: b.hover.to_egui(),
            pressed: b.pressed.to_egui(),
            separator: b.separator.to_egui(),
            windows_close: b.windows_close.to_egui(),
            windows_close_glyph: b.windows_close_glyph.to_egui(),
            breeze_close_hover: b.breeze_close_hover.to_egui(),
            breeze_close_hover_glyph: b.breeze_close_hover_glyph.to_egui(),
            breeze_close_pressed: b.breeze_close_pressed.to_egui(),
            breeze_close_pressed_glyph: b.breeze_close_pressed_glyph.to_egui(),
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
    fn the_style_reads_the_title_bar_tokens() {
        for (size, height, pad) in [(Size::Compact, 32.0, 8.0), (Size::Comfortable, 38.0, 12.0)] {
            for mode in [Mode::Light, Mode::Dark] {
                let t = tokens(size, mode);
                let s = TitleBarStyle::from_tokens(&t);
                let c = &t.component.title_bar;
                assert_eq!((s.height, s.height_adwaita, s.pad), (height, 47.0, pad));
                assert_eq!((s.bar, s.glyph, s.hover), (c.bar.to_egui(), c.glyph.to_egui(), c.hover.to_egui()));
                assert_eq!(s.windows_close, Color32::from_rgb(0xC4, 0x2B, 0x1C));
            }
        }
    }
}
