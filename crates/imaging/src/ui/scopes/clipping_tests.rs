//! The clipping overlay's picture.

use craft_theme::{Appearance, Contrast, Mode, Size, TextSize, builtins, resolve};
use craft_ui::theme::{FontFamilies, ToEgui as _, UiTokens};
use egui::Color32;

use super::{ClippingMode, ClippingOverlay, ClippingWarnings};
use crate::scopes::{ChannelMask, ClippingMask};

fn tokens() -> UiTokens {
    let appearance =
        Appearance { mode: Mode::Dark, contrast: Contrast::Standard, size: Size::Compact, text_size: TextSize::Small, reduced_motion: false };
    let theme = builtins().iter().find(|t| t.id.as_str() == "craft.studio").cloned().unwrap();
    let families = FontFamilies {
        ui: egui::FontFamily::Proportional,
        ui_semibold: egui::FontFamily::Proportional,
        mono: egui::FontFamily::Monospace,
        optical_size: false,
    };
    UiTokens::new(resolve(&theme, appearance, None).tokens, appearance, Mode::Dark, families)
}

// Warnings (highlights win over shadows), shadow diagnostics (`RGB ^ shadows`), highlight
// diagnostics (`data.scope` where none clip); transparent stays transparent.
#[test]
fn clipping_overlay_colours_follow_the_mode() {
    let tokens = tokens();
    let data = &tokens.color.data;
    // Pixels: highlight red and shadow blue, shadow red only, nothing, highlight green and blue.
    let mut mask = ClippingMask::new([4, 1]);
    mask.highlights[0] = ChannelMask::R;
    mask.shadows[0] = ChannelMask::B;
    mask.shadows[1] = ChannelMask::R;
    mask.highlights[3] = ChannelMask(ChannelMask::G.bits() | ChannelMask::B.bits());
    let both = ClippingWarnings { shadows: true, highlights: true };
    let image = |mode| ClippingOverlay::image(&mask, mode, &tokens).pixels;

    let warnings = image(ClippingMode::Warnings(both));
    assert_eq!(warnings, [data.red.to_egui(), data.blue.to_egui(), Color32::TRANSPARENT, data.red.to_egui()]);
    let shadows_only = image(ClippingMode::Warnings(ClippingWarnings { shadows: true, highlights: false }));
    assert_eq!(shadows_only, [data.blue.to_egui(), data.blue.to_egui(), Color32::TRANSPARENT, Color32::TRANSPARENT]);

    let shadow_channels = image(ClippingMode::ShadowChannels);
    let add = |a: Color32, b: Color32| Color32::from_rgb(a.r().saturating_add(b.r()), a.g().saturating_add(b.g()), a.b().saturating_add(b.b()));
    assert_eq!(shadow_channels[0], add(data.red.to_egui(), data.green.to_egui()));
    assert_eq!(shadow_channels[1], add(data.green.to_egui(), data.blue.to_egui()));

    let highlight_channels = image(ClippingMode::HighlightChannels);
    assert_eq!(highlight_channels[0], data.red.to_egui());
    assert_eq!(highlight_channels[1], data.scope.to_egui());
    assert_eq!(highlight_channels[3], add(data.green.to_egui(), data.blue.to_egui()));
    assert!(image(ClippingMode::None).iter().all(|c| *c == Color32::TRANSPARENT));
}
