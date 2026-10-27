//! The caption buttons' colours.

use egui::Color32;

use crate::theme::color32;

/// What the caption buttons paint in the app's colours. The app fills it from its theme. The OS
/// reds of the Windows and Breeze close buttons are fixed and not part of it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CaptionColors {
    /// The glyphs in the active window.
    pub glyph: Color32,
    /// The glyphs while the window is inactive, as Windows dims its captions.
    pub glyph_inactive: Color32,
    /// The fill under the pointer: minimize and maximize, and close too on Adwaita and Generic.
    pub hover: Color32,
    /// The fill while a button is held down.
    pub pressed: Color32,
    /// The bar's background: Breeze's glyph on its hovered or pressed disc.
    pub bar: Color32,
}

impl CaptionColors {
    /// The colours from the suite theme's roles: the glyphs in `text` (`text_muted` while the
    /// window is inactive), the fills of a control under the pointer and held down, and the bar
    /// in `surface`, the role the app's chrome is painted with.
    pub fn from_resolved(r: &craft_theme::Resolved) -> Self {
        Self {
            glyph: color32(r.roles.text),
            glyph_inactive: color32(r.roles.text_muted),
            hover: color32(r.derived.hover),
            pressed: color32(r.derived.pressed),
            bar: color32(r.roles.surface),
        }
    }
}

#[cfg(test)]
mod tests {
    use craft_theme::contrast::ratio;
    use craft_theme::{Resolved, Rgba, builtins, derive};

    use super::*;
    use crate::title_bar::{BREEZE_CLOSE_HOVER, BREEZE_CLOSE_HOVER_GLYPH, BREEZE_CLOSE_PRESSED, BREEZE_CLOSE_PRESSED_GLYPH, WINDOWS_CLOSE};

    fn rgba(c: Color32) -> Rgba {
        Rgba([c.r(), c.g(), c.b(), 255])
    }

    #[test]
    fn built_in_themes_fill_the_caption_colours_from_their_roles() {
        let mut variants = 0;
        for theme in builtins() {
            for (variant, roles) in theme.variants() {
                let derived = derive(roles, variant);
                let r = Resolved { theme: theme.id.clone(), variant, roles: *roles, derived, note: None };
                let c = CaptionColors::from_resolved(&r);
                let expected = [roles.text, roles.text_muted, derived.hover, derived.pressed, roles.surface].map(color32);
                assert_eq!([c.glyph, c.glyph_inactive, c.hover, c.pressed, c.bar], expected, "{} {variant:?}", theme.id);
                variants += 1;
            }
        }
        assert!(variants >= 2, "the built-ins have a light and a dark variant");
    }

    #[test]
    fn the_windows_close_glyph_reads_on_its_red() {
        assert!(ratio(rgba(Color32::WHITE), rgba(WINDOWS_CLOSE)) >= 4.5);
    }

    #[test]
    fn the_breeze_close_glyph_reads_on_its_reds() {
        // The reds and glyphs are fixed, so this holds for light and dark bars alike; at rest Close
        // has no red and uses the theme's glyph colour, checked by the theme's own contrast table.
        assert!(ratio(rgba(BREEZE_CLOSE_HOVER_GLYPH), rgba(BREEZE_CLOSE_HOVER)) >= 4.5);
        assert!(ratio(rgba(BREEZE_CLOSE_PRESSED_GLYPH), rgba(BREEZE_CLOSE_PRESSED)) >= 4.5);
    }
}
