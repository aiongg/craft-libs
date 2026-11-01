//! [`MaskViewMode`], [`MaskViewState`] and the [`MaskViewRecipe`] an engine builds the view from
//! (task SE-F6).

#![expect(unused_variables, clippy::todo, reason = "skeleton: task SE-F6")]

use craft_theme::Srgba;
use serde::{Deserialize, Serialize};

use super::MaskViewLabels;
use crate::selection::ColorIndicates;

/// One of Select and Mask's seven views, in menu order (the order F cycles).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MaskViewMode {
    /// Only the masked layer, its unselected area see-through by the opacity. Default.
    #[default]
    OnionSkin,
    /// The image with the refined selection's ants.
    MarchingAnts,
    /// The image with the Overlay colour over the masked (or selected) area.
    Overlay,
    /// The selection over black.
    OnBlack,
    /// The selection over white.
    OnWhite,
    /// The coverage as a black and white mask.
    BlackAndWhite,
    /// The masked layer alone, its unselected area transparent.
    OnLayers,
}

impl MaskViewMode {
    /// Every view, in menu order.
    // pc: crates/ui-egui/src/canvas/select_mask_view.rs:42 `Mode::ALL`.
    pub const ALL: [MaskViewMode; 7] = [
        MaskViewMode::OnionSkin,
        MaskViewMode::MarchingAnts,
        MaskViewMode::Overlay,
        MaskViewMode::OnBlack,
        MaskViewMode::OnWhite,
        MaskViewMode::BlackAndWhite,
        MaskViewMode::OnLayers,
    ];

    /// PhotoCraft's id (dialog fields, automation): `onionSkin`, `marchingAnts`, `overlay`,
    /// `onBlack`, `onWhite`, `blackWhite`, `onLayers`.
    pub fn id(self) -> &'static str {
        // pc: crates/ui-egui/src/canvas/select_mask_view.rs:44-54 `Mode::id`.
        todo!()
    }

    /// The view with `id`, if any.
    pub fn from_id(id: &str) -> Option<MaskViewMode> {
        MaskViewMode::ALL.into_iter().find(|m| m.id() == id)
    }

    /// The letter that picks the view: O, M, V, A, T, K, Y.
    pub fn key(self) -> egui::Key {
        // pc: crates/ui-egui/src/canvas/select_mask_view.rs:73-83 `Mode::key`.
        todo!()
    }

    /// The view's opacity in percent until the user changes it: 50 for Onion Skin and Overlay, 100
    /// for On Black and On White, `None` for the three without one.
    pub fn default_opacity(self) -> Option<f32> {
        // pc: crates/ui-egui/src/canvas/select_mask_view.rs:86-92 `Mode::default_opacity`.
        todo!()
    }

    /// The next view in menu order, Onion Skin after On Layers (F).
    pub fn next(self) -> MaskViewMode {
        // pc: crates/ui-egui/src/canvas/select_mask_view.rs:94-97 `Mode::next`.
        todo!()
    }

    /// The view's name in `labels`.
    pub fn label(self, labels: &MaskViewLabels) -> &str {
        // new: one `MaskViewLabels` field per view.
        todo!()
    }
}

/// The view state of the Select and Mask dialog (PhotoCraft's `__view`, `__opacity`,
/// `__showEdge`, `__showOriginal`, `__hideView` fields). View state, not undoable.
#[derive(Clone, Debug, PartialEq)]
pub struct MaskViewState {
    /// The view shown.
    pub mode: MaskViewMode,
    /// Each view's opacity in percent (0..=100), by [`MaskViewMode::ALL`] index; the three
    /// without one keep 100 and ignore it.
    pub opacities: [f32; 7],
    /// J: show the band the edge detection refines instead of the refined selection.
    pub show_edge: bool,
    /// P: show the selection before refinement.
    pub show_original: bool,
    /// X held: the image without the view.
    pub hide_view: bool,
    /// Overlay's colour, encoded sRGB (Photoshop's rubylith red by default).
    pub overlay_color: Srgba,
    /// Whether Overlay's colour marks the masked areas (default) or the selected ones.
    pub overlay_indicates: ColorIndicates,
}

impl Default for MaskViewState {
    /// Onion Skin, the default opacities (50, 50, 100, 100), nothing shown extra, red masked areas.
    fn default() -> Self {
        // pc: crates/ui-egui/src/canvas/select_mask_view.rs:100-113 (`mode` default, `opacity`),
        // :456-465 (`View::of`: red, masked unless `indicates` is "selected").
        todo!()
    }
}

impl MaskViewState {
    /// The opacity of `mode` in percent, `None` for a view without one.
    pub fn opacity(&self, mode: MaskViewMode) -> Option<f32> {
        // pc: crates/ui-egui/src/canvas/select_mask_view.rs:109-113 `opacity` (clamped 0..=100).
        todo!()
    }

    /// Set `mode`'s opacity (percent, rounded, clamped to 0..=100); a view without one ignores it.
    pub fn set_opacity(&mut self, mode: MaskViewMode, percent: f32) {
        // pc: crates/ui-egui/src/canvas/select_mask_view.rs:115-119 `set_opacity`; :192 (rounded).
        todo!()
    }

    /// Apply one change (from the keys or the controls).
    pub fn apply(&mut self, change: MaskViewChange) {
        // new: one field per variant.
        todo!()
    }

    /// What the engine builds for the current view.
    ///
    /// With `o` the view's opacity (0..=1) and `c` the coverage:
    /// - coverage: [`MaskCoverage::EdgeBand`] with Show Edge, else [`MaskCoverage::Original`] with
    ///   Show Original, else [`MaskCoverage::Refined`];
    /// - Marching Ants: no veil, `ants` on;
    /// - Overlay: one veil of `overlay_color` at `o`, masked by `1 − c` (masked areas) or `c`
    ///   (selected areas);
    /// - On Black, On White: one black (white) veil at `o` masked by `1 − c`;
    /// - Black & White: a black veil at 1 over everything, then a white veil at 1 masked by `c`;
    /// - Onion Skin: the other layers hidden and the active layer masked by `c + (1 − c) · (1 − o)`
    ///   (`layer_floor` `1 − o`); On Layers: masked by `c` (`layer_floor` 0), the others kept;
    /// - `hidden` while X is held (the image without a view; the engine shows the document as is).
    pub fn recipe(&self) -> MaskViewRecipe {
        // pc: crates/ui-egui/src/canvas/select_mask_view.rs:497-507 (which coverage), :586-620
        // `show` (the veils and the layer mask).
        todo!()
    }
}

/// Which coverage the view shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum MaskCoverage {
    /// The refined selection (the engine runs the refinement).
    #[default]
    Refined,
    /// The selection before refinement (Show Original).
    Original,
    /// The band `radius` pixels either side of the edge, where grown and shrunk coverage differ,
    /// with the Refine Edge Brush strokes (Show Edge).
    EdgeBand,
}

/// Which part of the canvas a veil covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VeilMask {
    /// Everywhere.
    Everywhere,
    /// Masked by the coverage `c`: the selected area.
    Coverage,
    /// Masked by `1 − c`: the unselected area.
    Inverse,
}

/// A solid fill over the image, as the engine adds it (PhotoCraft's veil fill layers).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Veil {
    /// Encoded sRGB.
    pub color: Srgba,
    /// 0..=1.
    pub opacity: f32,
    pub mask: VeilMask,
}

/// What the engine builds for a view ([`MaskViewState::recipe`]).
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct MaskViewRecipe {
    pub mode: MaskViewMode,
    pub coverage: MaskCoverage,
    /// Fills over the image, bottom first.
    pub veils: Vec<Veil>,
    /// The active layer's mask becomes `c + (1 − c) · floor` (Onion Skin `1 − o`, On Layers 0);
    /// `None` leaves the layers' masks alone.
    pub layer_floor: Option<f32>,
    /// Hide every layer but the active one (with its groups and contents): Onion Skin.
    pub hide_other_layers: bool,
    /// The refined selection's ants show (Marching Ants only): `SelectionOverlay::visible`.
    pub ants: bool,
    /// X is held: show the image without the view.
    pub hidden: bool,
}

/// One change to a [`MaskViewState`], as the keys and the controls report it.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum MaskViewChange {
    /// A view picked (a letter, F, the drop-down).
    Mode(MaskViewMode),
    /// A view's opacity changed, in percent.
    Opacity(MaskViewMode, f32),
    ShowEdge(bool),
    ShowOriginal(bool),
    /// X pressed (`true`) or released.
    HideView(bool),
}
