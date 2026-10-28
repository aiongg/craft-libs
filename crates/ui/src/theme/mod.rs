//! The Craft design system in egui: resolved [`craft_theme`] tokens installed in a context, read
//! by components, and turned into egui [`Style`](egui::Style)s for stock widgets.
//!
//! An app resolves its selection with `craft-theme` (and its store) into a [`Base`] and calls
//! [`install`] every frame. Components read [`Tokens::of(ui)`](TokensOf::of). [`scope`] shows a
//! part of the UI with other tokens (a dark panel in a light app, a theme preview), and
//! [`apply_popup_style`] carries a scope into the popups opened inside it. Without
//! [`install`], tokens are the suite default theme, so tests and stories need no setup.
//!
//! ```
//! use craft_ui::theme::{Overrides, Tokens, TokensOf, scope};
//! # egui::__run_test_ui(|ui| {
//! let tokens = Tokens::of(ui);
//! ui.label(egui::RichText::new("Body").font(tokens.font(craft_theme::TypeRole::Body)));
//! scope(ui, |o| o.mode = Some(craft_theme::Mode::Dark), |ui| {
//!     assert_eq!(Tokens::of(ui).mode, craft_theme::Mode::Dark);
//! });
//! # });
//! ```

mod bridge;
mod convert;
mod roles;
mod style;
#[cfg(test)]
mod tests;
mod tokens;

pub use bridge::{Base, Captured, Inputs, Overrides, apply_popup_style, capture, install, scope, scoped_style};
pub use convert::{FontFamilies, ToEgui, corner_radius, margin, stroke};
pub use craft_theme::Tokens;
pub use roles::{TABULAR_FIGURES, role_format, style_format, tabular};
pub use style::style_for;
pub use tokens::{Radii, Shadows, TokensOf, UiTokens};

use egui::{Color32, CornerRadius, Stroke, style::Widgets};

/// Background colors for egui's five widget interaction states.
///
/// These are rendering inputs, not a theme schema: applications construct them from their own
/// tokens. In particular, `noninteractive` describes egui's noninteractive widget state, not a
/// disabled control palette. egui and the application retain their disabled-state behavior.
#[deprecated(note = "use `craft_ui::theme::install` and `style_for`, which style every widget state from the theme's tokens")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WidgetPalette {
    pub noninteractive: Color32,
    pub inactive: Color32,
    pub hovered: Color32,
    pub active: Color32,
    pub open: Color32,
}

#[allow(deprecated, reason = "the deprecated type's own impl")]
impl WidgetPalette {
    /// Set each state's strong and weak background fill, with a common radius and foreground.
    ///
    /// Borders, expansion and every other widget property are retained. Apply per-state
    /// exceptions afterward, e.g. transparent weak fill for outline-only inactive buttons.
    /// This deliberately does not choose selection colors, surfaces, shadows or text styles.
    pub fn apply_to(self, widgets: &mut Widgets, radius: CornerRadius, foreground: Stroke) {
        for (widget, fill) in [
            (&mut widgets.noninteractive, self.noninteractive),
            (&mut widgets.inactive, self.inactive),
            (&mut widgets.hovered, self.hovered),
            (&mut widgets.active, self.active),
            (&mut widgets.open, self.open),
        ] {
            widget.bg_fill = fill;
            widget.weak_bg_fill = fill;
            widget.corner_radius = radius;
            widget.fg_stroke = foreground;
        }
    }
}

#[cfg(test)]
#[allow(deprecated, reason = "tests the deprecated WidgetPalette")]
mod palette_tests {
    use super::*;

    #[test]
    fn switching_palette_retains_custom_borders_and_expansion() {
        let mut widgets = egui::Visuals::light().widgets;
        widgets.active.bg_stroke = Stroke::new(3.0, Color32::YELLOW);
        widgets.hovered.expansion = 7.0;
        let original_borders = [
            widgets.noninteractive.bg_stroke,
            widgets.inactive.bg_stroke,
            widgets.hovered.bg_stroke,
            widgets.active.bg_stroke,
            widgets.open.bg_stroke,
        ];
        let palette = WidgetPalette {
            noninteractive: Color32::BLACK,
            inactive: Color32::RED,
            hovered: Color32::GREEN,
            active: Color32::BLUE,
            open: Color32::TRANSPARENT,
        };
        let foreground = Stroke::new(2.0, Color32::WHITE);
        palette.apply_to(&mut widgets, CornerRadius::same(4), foreground);
        assert_eq!(widgets.noninteractive.bg_fill, Color32::BLACK);
        assert_eq!(widgets.inactive.bg_fill, Color32::RED);
        assert_eq!(widgets.hovered.bg_fill, Color32::GREEN);
        assert_eq!(widgets.active.bg_fill, Color32::BLUE);
        assert_eq!(widgets.open.bg_fill, Color32::TRANSPARENT);
        for widget in [&widgets.noninteractive, &widgets.inactive, &widgets.hovered, &widgets.active, &widgets.open] {
            assert_eq!(widget.weak_bg_fill, widget.bg_fill);
            assert_eq!(widget.corner_radius, CornerRadius::same(4));
            assert_eq!(widget.fg_stroke, foreground);
        }
        assert_eq!(widgets.hovered.expansion, 7.0);
        assert_eq!(
            [
                widgets.noninteractive.bg_stroke,
                widgets.inactive.bg_stroke,
                widgets.hovered.bg_stroke,
                widgets.active.bg_stroke,
                widgets.open.bg_stroke
            ],
            original_borders
        );
        // A second theme application replaces prior fills, including an app's outline override.
        widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
        WidgetPalette { inactive: Color32::GRAY, ..palette }.apply_to(&mut widgets, CornerRadius::ZERO, foreground);
        assert_eq!(widgets.inactive.bg_fill, Color32::GRAY);
        assert_eq!(widgets.inactive.weak_bg_fill, Color32::GRAY);
        assert_eq!(widgets.active.bg_stroke, Stroke::new(3.0, Color32::YELLOW));
    }
}
