//! The clipping warning buttons and [`ClippingOverlay`], one nearest-sampled texture over a
//! preview.

use craft_ui::canvas::textures::{Revision, TextureCache, TextureCacheOptions};
use craft_ui::primitives::{Corner, FocusRing};
use craft_ui::theme::{ToEgui as _, UiTokens};
use egui::{AsIdSalt, Color32, ColorImage, Context, IdSalt, Painter, Pos2, Rect, Role, Sense, Shape, Stroke, TextureOptions, WidgetInfo, pos2};

use crate::scopes::{ChannelMask, ClippingMask};

/// The clipped channels the warning buttons show: their fill is these channels' data colour
/// (`data.red`, `data.green`, `data.blue` mixed by the mask, lead call L14), black when none clip.
/// The host folds its histogram's end bins or [`ClippingMask::any`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ClippingFrame {
    /// Channels clipped at 0 anywhere.
    pub shadows: ChannelMask,
    /// Channels clipped at 1 anywhere.
    pub highlights: ChannelMask,
}

/// The clipping warnings that are on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ClippingWarnings {
    /// The shadow warning (U): shadow-clipped pixels in `data.blue`.
    pub shadows: bool,
    /// The highlight warning (O): highlight-clipped pixels in `data.red`.
    pub highlights: bool,
}

/// An Alt-drag on a tone control that shows the per-channel diagnostics instead of the warnings;
/// the host knows which control is dragged.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClippingDiagnostic {
    /// Blacks or Shadows: each pixel in the colour of the channels that do not clip at 0.
    Shadows,
    /// Highlights, Whites or Exposure: each pixel in the colour of the channels that clip at 1.
    Highlights,
}

/// What the clipping overlay draws ([`ScopeSettings::clipping_mode`](super::ScopeSettings::clipping_mode)).
/// Warnings and diagnostics are separate modes, so both warnings never read as a diagnostic.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ClippingMode {
    /// Nothing.
    #[default]
    None,
    /// The warnings that are on: highlight-clipped pixels in `data.red` (they win), else
    /// shadow-clipped ones in `data.blue`, else transparent.
    Warnings(ClippingWarnings),
    /// Shadow diagnostics: each pixel in the data colour of the channels *not* clipped at 0
    /// (`RGB ^ shadows`).
    ShadowChannels,
    /// Highlight diagnostics: each pixel in the data colour of its highlight-clipped channels,
    /// `data.scope` where none clip.
    HighlightChannels,
}

/// The clipping overlay over a preview: one texture of the mask's size, nearest-sampled, drawn
/// over the preview rect. The texture lives in an internal
/// [`TextureCache`](craft_ui::canvas::textures::TextureCache) with
/// [`TextureOptions::NEAREST`](egui::TextureOptions::NEAREST) under the overlay's id, rebuilt
/// only when `revision`, the mode or the theme's mode changes (lead call L33). Transparent and
/// non-finite pixels are transparent in every mode.
///
/// ```ignore
/// ClippingOverlay::new("clip").show(ui.ctx(), ui.painter(), preview_rect, &mask, revision, mode);
/// ```
#[must_use = "draw the overlay with `show`"]
pub struct ClippingOverlay {
    pub(super) id_salt: IdSalt,
}

impl ClippingOverlay {
    /// An overlay; `id_salt` keys its texture.
    pub fn new(id_salt: impl AsIdSalt) -> Self {
        Self { id_salt: IdSalt::new(id_salt) }
    }

    /// Draws `mask` over `rect` in `mode`; [`ClippingMode::None`] draws nothing and keeps the
    /// texture. `revision` is the host's revision of the image the mask was counted from.
    pub fn show(self, ctx: &Context, painter: &Painter, rect: Rect, mask: &ClippingMask, revision: u64, mode: ClippingMode) {
        // pc: crates/ui-egui/src/dialogs/camera_raw_scope_ui.rs:615-666 (key, rebuild, `load_texture`/`set` NEAREST, draw);
        // new: the texture goes through `TextureCache::get_or_put` with `Revision::of(&(revision, mode, tokens.mode))`.
        if mode == ClippingMode::None {
            return;
        }
        let tokens = craft_ui::theme::current(ctx);
        let cache = TextureCache::<()>::in_context(ctx, self.id_salt, TextureCacheOptions::frames().filter(TextureOptions::NEAREST));
        let texture = cache.get_or_put(ctx, (), Revision::of(&(revision, mode, tokens.mode)), || Self::image(mask, mode, &tokens));
        painter.image(texture.id, rect, Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)), Color32::WHITE); // design: allow(identity tint)
    }

    /// The overlay picture of `mask` in `mode`: one pixel per mask pixel, colours from `tokens`
    /// (`data.red`, `data.green`, `data.blue` mixed by channel mask, `data.scope`), PhotoCraft's
    /// `histogram_color(mask)` rule.
    pub fn image(mask: &ClippingMask, mode: ClippingMode, tokens: &UiTokens) -> ColorImage {
        // pc: crates/ui-egui/src/dialogs/camera_raw_scope_ui.rs:622-648 (`color` closure); crates/ui-egui/src/theme.rs:771 `histogram_color`.
        let scope = tokens.color.data.scope.to_egui();
        let pixels = (0..mask.size[1])
            .flat_map(|y| (0..mask.size[0]).map(move |x| (x, y)))
            .map(|(x, y)| {
                let clipped = mask.at(x, y);
                match mode {
                    ClippingMode::None => Color32::TRANSPARENT,
                    ClippingMode::Warnings(warnings) => {
                        if warnings.highlights && !clipped.highlights.is_empty() {
                            channels_color(ChannelMask::R, tokens)
                        } else if warnings.shadows && !clipped.shadows.is_empty() {
                            channels_color(ChannelMask::B, tokens)
                        } else {
                            Color32::TRANSPARENT
                        }
                    }
                    ClippingMode::ShadowChannels => channels_color(ChannelMask(ChannelMask::RGB.bits() ^ clipped.shadows.bits()), tokens),
                    ClippingMode::HighlightChannels if clipped.highlights.is_empty() => scope,
                    ClippingMode::HighlightChannels => channels_color(clipped.highlights, tokens),
                }
            })
            .collect();
        ColorImage::new(mask.size, pixels)
    }
}

/// The data colour of the channels in `mask`: `data.red`, `data.green` and `data.blue` added
/// (saturating), `data.scope` for none.
fn channels_color(mask: ChannelMask, tokens: &UiTokens) -> Color32 {
    // pc: crates/ui-egui/src/theme.rs:771 `histogram_color` (the channel level is the data token).
    let data = &tokens.color.data;
    let parts = [(ChannelMask::R, data.red), (ChannelMask::G, data.green), (ChannelMask::B, data.blue)];
    let mut sum = [0u8; 3];
    for (_, color) in parts.into_iter().filter(|(bit, _)| mask.contains(*bit)) {
        let color = color.to_egui();
        for (total, part) in sum.iter_mut().zip([color.r(), color.g(), color.b()]) {
            *total = total.saturating_add(part);
        }
    }
    if mask.is_empty() { data.scope.to_egui() } else { Color32::from_rgb(sum[0], sum[1], sum[2]) }
}

/// One clipping warning button: a triangle in `rect` (`scopes.warning_size` square, inset 3),
/// filled with the data colour of `mask` (black when empty), outlined 1 pt in `text.normal` when
/// `on`, `text.faint` when off; a click toggles `on`. AccessKit: a `CheckBox` named `label`
/// ("Shadow Clipping Warning (U)"), checked as `on`; its tooltip is `label`. Returns whether it
/// toggled.
pub(super) fn warning_button(ui: &mut egui::Ui, rect: Rect, mask: ChannelMask, on: &mut bool, label: &str) -> bool {
    // pc: crates/ui-egui/src/dialogs/camera_raw_scope_ui.rs:345-360 `warning`.
    use craft_ui::controls::TooltipExt as _;
    use craft_ui::theme::{Tokens, TokensOf as _};

    let tokens = Tokens::of(ui);
    let response = ui.interact(rect, ui.scope_id().with(("scope-warning", label)), Sense::click());
    response.widget_info(|| WidgetInfo::selected(Role::CheckBox, ui.is_enabled(), *on, label));
    let response = response.tooltip(label);
    let triangle = rect.shrink(3.0);
    let outline = if *on { tokens.color.text.normal } else { tokens.color.text.faint };
    ui.painter().add(Shape::convex_polygon(
        vec![pos2(triangle.center().x, triangle.top()), triangle.left_bottom(), triangle.right_bottom()],
        channels_color(mask, &tokens),
        Stroke::new(1.0, outline.to_egui()),
    ));
    FocusRing::paint(ui, rect, Corner::Control, &response);
    let toggled = response.clicked();
    if toggled {
        *on = !*on;
    }
    toggled
}
