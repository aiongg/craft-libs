//! `CompareTile`: one fitted image with zoom and pan shared across tiles, and its pending and
//! failed pills.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use craft_ui::canvas::CanvasView;
use craft_ui::canvas::textures::{CachedTexture, Revision};
use egui::{AsIdSalt, ColorImage, Painter, Rect, Response, Ui};

use super::{CompareCaption, CompareLabels};

/// The picture a compare tile shows.
pub enum CompareImage<'a> {
    /// A texture from the app's own `TextureCache`, with the image's full size in pixels (for Fit
    /// and 100 %).
    Texture { texture: CachedTexture, size: [u32; 2] },
    /// A picture the tile uploads into its own `TextureCache` (`get_or_put` keyed by `key` and
    /// `revision`; `make` runs once per revision).
    Picture { key: u64, revision: Revision, size: [u32; 2], make: Box<dyn FnOnce() -> ColorImage + 'a> },
    /// Nothing yet: the thumbnail placeholder.
    None,
}

/// The host's render state for the tile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RenderState {
    /// The picture is current.
    Ready,
    /// A render is running; `overdue` once it has taken longer than the host's wait, which shows
    /// the "Rendering…" pill (`surface.scrim` with a `Spinner`).
    Pending { overdue: bool },
    /// The render failed: the "Render failed" pill (`status.bad.solid`) at once, the message as
    /// its tooltip.
    Failed(String),
}

/// What [`CompareTile::show`] returns.
#[derive(Debug)]
pub struct CompareResponse {
    /// The tile.
    pub response: Response,
    /// A click: the host makes the pane active.
    pub clicked: bool,
    /// A double click: the host toggles Fit and 100 % ([`CompareTile::toggle_zoom`]).
    pub double_clicked: bool,
    /// The shared view changed this pass (zoom or pan).
    pub view_changed: bool,
    /// Where the picture was drawn.
    pub image_rect: Rect,
}

/// The host's drawing over the picture.
type Overlay<'a> = Box<dyn FnOnce(&Painter, &CanvasView, Rect) + 'a>;

/// One compare pane (N8.08): the image fitted into the area less the caption strip, zoomed and
/// panned through a `CanvasView` the tiles share (wheel, pinch, drag through
/// `canvas::WheelNavigation` and `canvas::pan`; the tile works on a copy whose `rect` is its own
/// image rect, and writes back `center` and `zoom`, so equal panes stay in step), the caption,
/// the pills, the rejected veil (`tile.veil`), and the active ring (`tile.ring.selected` 2 pt with
/// its halo). `.survey(true)`: no zoom or pan, always fitted. Window renders are the host's,
/// drawn through `.overlay(..)` over the picture. AccessKit: an `Image` named by
/// `CompareLabels::tile_name` (the caption read as text), the pill text as its description.
pub struct CompareTile<'a> {
    id_salt: egui::Id,
    view: &'a mut CanvasView,
    image: CompareImage<'a>,
    caption: Option<CompareCaption>,
    state: RenderState,
    active: bool,
    rejected: bool,
    survey: bool,
    overlay: Option<Overlay<'a>>,
    labels: Option<CompareLabels>,
}

impl<'a> CompareTile<'a> {
    /// A tile showing `image` through the shared `view`.
    pub fn new(id_salt: impl AsIdSalt, view: &'a mut CanvasView, image: CompareImage<'a>) -> Self {
        // lightcraft: crates/ui-egui/src/panels/compare.rs:213 (`photo_tile`).
        todo!()
    }

    /// The caption strip.
    #[must_use]
    pub fn caption(self, caption: CompareCaption) -> Self {
        // new: builder.
        todo!()
    }

    /// The render state (default `Ready`).
    #[must_use]
    pub fn state(self, state: RenderState) -> Self {
        // lightcraft: crates/ui-egui/src/panels/compare.rs:328 (the pills).
        todo!()
    }

    /// The active pane: the ring.
    #[must_use]
    pub fn active(self, active: bool) -> Self {
        // lightcraft: crates/ui-egui/src/panels/compare.rs:298 (the active tile's ring).
        todo!()
    }

    /// The rejected veil.
    #[must_use]
    pub fn rejected(self, rejected: bool) -> Self {
        // lightcraft: crates/ui-egui/src/panels/compare.rs:292 (black α110 veil).
        todo!()
    }

    /// Survey: fitted, no zoom or pan.
    #[must_use]
    pub fn survey(self, survey: bool) -> Self {
        // lightcraft: crates/ui-egui/src/panels/compare.rs:232 (zoom and pan only in Compare and Reference).
        todo!()
    }

    /// The host's drawing over the picture (a window render), with the tile's view and image rect.
    #[must_use]
    pub fn overlay(self, overlay: impl FnOnce(&Painter, &CanvasView, Rect) + 'a) -> Self {
        // lightcraft: crates/ui-egui/src/panels/compare.rs:446 (`draw_window`).
        todo!()
    }

    /// The words.
    #[must_use]
    pub fn labels(self, labels: CompareLabels) -> Self {
        // new: builder.
        todo!()
    }

    /// Draws the tile in `rect`.
    pub fn show(self, ui: &mut Ui, rect: Rect) -> CompareResponse {
        // lightcraft: crates/ui-egui/src/panels/compare.rs:213 (`photo_tile`).
        todo!()
    }

    /// Toggles `view` between Fit (`canvas::fit_view` for an image of `size` in `area`) and 100 %
    /// (one device pixel per image pixel, about the area's centre).
    pub fn toggle_zoom(view: &mut CanvasView, size: [u32; 2], area: Rect, pixels_per_point: f32) {
        // lightcraft: crates/ui-egui/src/panels/compare.rs:520 (double click toggles Fit and 100 %).
        todo!()
    }
}
