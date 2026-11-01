//! The clipping warning buttons and [`ClippingOverlay`], one nearest-sampled texture over a
//! preview.

use craft_ui::theme::UiTokens;
use egui::{AsIdSalt, ColorImage, Context, IdSalt, Painter, Rect};

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
#[expect(dead_code, reason = "skeleton: removed when the bodies land")]
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
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn show(self, ctx: &Context, painter: &Painter, rect: Rect, mask: &ClippingMask, revision: u64, mode: ClippingMode) {
        // pc: crates/ui-egui/src/dialogs/camera_raw_scope_ui.rs:615-666 (key, rebuild, `load_texture`/`set` NEAREST, draw);
        // new: the texture goes through `TextureCache::get_or_put` with `Revision::of(&(revision, mode, tokens.mode))`.
        todo!()
    }

    /// The overlay picture of `mask` in `mode`: one pixel per mask pixel, colours from `tokens`
    /// (`data.red`, `data.green`, `data.blue` mixed by channel mask, `data.scope`), PhotoCraft's
    /// `histogram_color(mask)` rule.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
    pub fn image(mask: &ClippingMask, mode: ClippingMode, tokens: &UiTokens) -> ColorImage {
        // pc: crates/ui-egui/src/dialogs/camera_raw_scope_ui.rs:622-648 (`color` closure); crates/ui-egui/src/theme.rs:771 `histogram_color`.
        todo!()
    }
}

/// One clipping warning button: a triangle in `rect` (`scopes.warning_size` square, inset 3),
/// filled with the data colour of `mask` (black when empty), outlined 1 pt in `text.normal` when
/// `on`, `text.faint` when off; a click toggles `on`. AccessKit: a `CheckBox` named `label`
/// ("Shadow Clipping Warning (U)"), checked as `on`; its tooltip is `label`. Returns whether it
/// toggled.
#[expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
pub(super) fn warning_button(ui: &mut egui::Ui, rect: Rect, mask: ChannelMask, on: &mut bool, label: &str) -> bool {
    // pc: crates/ui-egui/src/dialogs/camera_raw_scope_ui.rs:345-360 `warning`.
    todo!()
}
