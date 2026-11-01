//! Vanishing Point's drawing: planes, grids, nodes, the paste, the stamp (task WS-VP2).

use egui::Painter;

use craft_ui::canvas::CanvasView;

use super::VanishingPointSource;
use super::editor::{VanishingPointEditor, VanishingPointOptions, VanishingPointSession};
use super::planes::VanishingScene;
use super::style::VanishingPointStyle;

/// Draws the editor over the image, in this order:
///
/// 1. The stamp preview: the app's `VanishingPointSource::stamp_preview` over the image while
///    strokes exist (lead call 47).
/// 2. Each plane ([`VanishingScene::resolve`]d): an invalid plane only its outline; the others
///    their grid (cells of `grid_size` at the nearest edge, at most `MAX_GRID_LINES` a direction,
///    `vanishing_point.grid` 0.75 at `.grid_alpha` 0.6) and outline (`.line` 1.25, the selected
///    plane `.line_selected` 2), in the check's colour, each over the `.halo`.
/// 3. The paste on the selected plane while Paste is on: the app's image mapped through the
///    plane's frame on a 24 × 24 mesh.
/// 4. The selected plane's nodes: corners and edge middles as `.node` (7) white squares with a
///    1.25 outline in the plane's colour; Create Plane's pending corners as `.pending` (6)
///    squares with the rubber band.
/// 5. Stamp: the source cross (`.source_arm` 7 arms, 1 pt white over the halo) and the
///    foreshortened ring of the diameter ([`VanishingScene::stamp_ring`]).
///
/// Decoration: the parts' AccessKit nodes are the host's (`ToolParts`).
#[expect(clippy::too_many_arguments, reason = "one painter over the editor's whole state")]
#[expect(unused_variables, clippy::todo, reason = "skeleton: task WS-VP2")]
pub fn paint_editor(
    painter: &Painter,
    view: &CanvasView,
    editor: &VanishingPointEditor,
    session: &VanishingPointSession,
    options: &VanishingPointOptions,
    scene: &dyn VanishingScene,
    source: &dyn VanishingPointSource,
    style: &VanishingPointStyle,
) {
    // pc: crates/ui-egui/src/dialogs/vp_ui.rs:1078 `draw_content`, :1091 `draw` (colours, widths,
    // nodes :1091-1135), :1148 `draw_stamp`, :1189 `grid_steps`, :1205 `draw_paste`.
    todo!()
}
