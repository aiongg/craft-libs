//! The sampling tint and the fill area's ants over Content-Aware Fill's canvas (task WS-CA2).

use egui::{AsIdSalt, Color32, IdSalt, Ui};

use craft_ui::canvas::CanvasView;

/// A bit plane from the app (lead call 40): `size` texels of `texel × texel` document pixels from
/// document pixel `origin`, row by row. PhotoCraft computes them from its plan
/// (`content_aware_plan`, `paint_strokes`) at a proxy of `k` pixels per texel.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SamplingPlane {
    pub origin: [i32; 2],
    pub size: [u32; 2],
    pub texel: u32,
    pub bits: Vec<bool>,
}

/// The two marks Content-Aware Fill draws over the document:
///
/// - The **tint**: `color` at `opacity` on every texel where the sampling plane differs from
///   `excluded` and that is outside the fill area (Indicates › Sampling Area tints what is
///   sampled; Excluded Area what is not). Built into one texture per change of the planes, the
///   colour, the opacity or Indicates (a texel-scaled image, nearest filtering), drawn while Show
///   Sampling Area is on and ⌘H has not hidden it.
/// - The **ants** around the fill area: `craft_imaging::trace::edges_of_grid` over a
///   `CoverageGrid` of the fill plane (at most `GRID_EDGE_LIMIT` edges, else the plane's bounds
///   as a 1 pt white rect), black and white dashes by `(x + y) / 4 + phase`, the phase moving 8
///   steps a second (a repaint every 120 ms).
///
/// The tint and the ants sit on the document: no AccessKit node.
#[expect(dead_code, reason = "skeleton: `show` (task WS-CA2) reads the fields")]
pub struct FillOverlay<'a> {
    id_salt: IdSalt,
    sampling: Option<&'a SamplingPlane>,
    fill_area: Option<&'a SamplingPlane>,
    color: Color32,
    excluded: bool,
    tint: bool,
}

impl<'a> FillOverlay<'a> {
    pub fn new(id_salt: impl AsIdSalt, sampling: Option<&'a SamplingPlane>, fill_area: Option<&'a SamplingPlane>) -> Self {
        Self { id_salt: IdSalt::new(id_salt), sampling, fill_area, color: Color32::TRANSPARENT, excluded: false, tint: true }
    }

    /// The tint's colour with the opacity as its alpha (the user's Color and Opacity).
    pub fn color(mut self, color: Color32) -> Self {
        self.color = color;
        self
    }

    /// Indicates › Excluded Area.
    pub fn excluded(mut self, excluded: bool) -> Self {
        self.excluded = excluded;
        self
    }

    /// Whether the tint shows (Show Sampling Area, ⌘H).
    pub fn tint(mut self, tint: bool) -> Self {
        self.tint = tint;
        self
    }

    /// Draws the tint and the ants in `ui`'s painter over `view`.
    #[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-CA2")]
    pub fn show(self, ui: &Ui, view: &CanvasView) {
        // pc: crates/ui-egui/src/dialogs/caf_ui/view.rs:87-101 (the tint texels), :377-397 (tint
        // and ants), caf_ui.rs:60 `MAX_OUTLINE_EDGES`.
        todo!()
    }
}
