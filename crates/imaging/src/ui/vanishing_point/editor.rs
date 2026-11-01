//! [`VanishingPointEditor`]: the plane tools, the paste and the stamp, without a frame (task
//! WS-VP1).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: task WS-VP1")]

use egui::Painter;
use serde::{Deserialize, Serialize};

use craft_ui::canvas::quad::Homography;
use craft_ui::canvas::{CanvasTool, ToolAction, ToolContext, ToolCursor, ToolKey, ToolParts, ToolPointer};
use craft_ui::edit::GestureId;

use super::VanishingPointLabels;
use super::VanishingPointSource;
use super::planes::{TornPlane, VanishingPlane, VanishingScene};
use super::style::VanishingPointStyle;

/// The editor's tools: Edit Plane (V), Create Plane (C), Stamp (S). The workspace's strip adds
/// Hand and Zoom.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum VanishingTool {
    EditPlane,
    #[default]
    CreatePlane,
    Stamp,
}

/// The Stamp tool's options.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StampOptions {
    /// The brush's diameter in pixels where a stroke starts.
    pub diameter: f64,
    /// 0..=100.
    pub hardness: f64,
    /// 0..=100.
    pub opacity: f64,
    /// Later strokes keep the first stroke's source offset instead of restarting at the source.
    pub aligned: bool,
}

impl Default for StampOptions {
    fn default() -> Self {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:826.
        Self { diameter: 40.0, hardness: 50.0, opacity: 100.0, aligned: true }
    }
}

/// A clone stroke as the engine paints it (`filter.vanishingPoint`'s `clone` entries, PhotoCraft
/// `CloneSpec`): the source, the points, the diameter, and hardness and opacity in 0..=1 (the
/// options' percent / 100).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StampStroke {
    pub source: [f64; 2],
    pub points: Vec<[f64; 2]>,
    pub size: f64,
    pub hardness: f64,
    pub opacity: f64,
}

/// The pasted image on the selected plane: its top-left in plane coordinates (0..=1) and its width
/// as a fraction of the plane's width.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PasteImage {
    pub at: [f64; 2],
    pub width: f64,
}

/// The planes as a gesture reports them: every plane and how it was torn off.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlaneEdit {
    pub planes: Vec<VanishingPlane>,
    pub torn: Vec<Option<TornPlane>>,
}

/// What OK applies and the history snapshots (PhotoCraft's `VpEdit`): the planes, how they were
/// torn off, where the paste sits, the clone source and strokes. The tool, the pending corners,
/// the selection and the bar's options are outside it (risk 8).
#[derive(Clone, Debug, PartialEq)]
pub struct VanishingPointSession {
    pub planes: Vec<VanishingPlane>,
    pub torn: Vec<Option<TornPlane>>,
    pub paste: PasteImage,
    /// The clone source (Alt-click), document pixels.
    pub source: Option<[f64; 2]>,
    pub strokes: Vec<StampStroke>,
    /// The first stroke's (source, start) since the source was set: Aligned strokes keep its
    /// offset.
    pub aligned_from: Option<([f64; 2], [f64; 2])>,
}

impl Default for VanishingPointSession {
    fn default() -> Self {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:814-829 (the paste at 0.25, 0.25, half the plane wide).
        Self {
            planes: Vec::new(),
            torn: Vec::new(),
            paste: PasteImage { at: [0.25, 0.25], width: 0.5 },
            source: None,
            strokes: Vec::new(),
            aligned_from: None,
        }
    }
}

/// The options bar's values that are not undone: the grid's cell size (100), whether to paste the
/// clipboard (on when it holds an image; the app sets it), whether the result goes to a new layer
/// (off), the stamp options.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct VanishingPointOptions {
    /// Grid cell size in pixels at the plane's nearest edge.
    pub grid_size: f64,
    pub paste: bool,
    pub new_layer: bool,
    pub stamp: StampOptions,
}

impl Default for VanishingPointOptions {
    fn default() -> Self {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:819-826.
        Self { grid_size: 100.0, paste: false, new_layer: false, stamp: StampOptions::default() }
    }
}

/// The editor's state between passes: the tool (Create Plane at first, PhotoCraft
/// `vp_ui.rs:816`), the corners clicked so far with Create Plane, the
/// selected plane, the drag, the picked part, the events not yet taken. Runs in the workspace's
/// canvas and, without a frame, on any canvas (lead call 46).
#[derive(Debug)]
pub struct VanishingPointEditor {
    tool: VanishingTool,
    pending: Vec<[f64; 2]>,
    selected: Option<usize>,
    drag: Option<(GestureId, VanishingDrag)>,
    picked: Option<u64>,
    events: Vec<super::VanishingPointEvent>,
}

/// A drag in progress.
#[derive(Clone, Debug)]
enum VanishingDrag {
    /// A corner node and the pointer's offset from it.
    Corner { plane: usize, corner: usize, offset: [f64; 2] },
    /// An edge node: the edge slides along the plane's perspective at the press.
    Edge { plane: usize, edge: usize, at_press: Homography },
    /// The pasted image and the pointer's offset from its top-left in plane coordinates.
    Paste { offset: [f64; 2] },
    /// The far edge node of a torn plane: depth, or with Alt its angle, once it left `start`.
    Torn { plane: usize, rotate: bool, start: [f64; 2], moved: bool },
    /// A clone stroke.
    Stamp,
}

impl Default for VanishingPointEditor {
    fn default() -> Self {
        Self { tool: VanishingTool::CreatePlane, pending: Vec::new(), selected: None, drag: None, picked: None, events: Vec::new() }
    }
}

impl VanishingPointEditor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tool(&self) -> VanishingTool {
        self.tool
    }

    /// Picks a tool; Edit Plane and Stamp drop the pending corners.
    pub fn set_tool(&mut self, tool: VanishingTool) {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:1052-1059.
        todo!()
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Esc: drops the pending corners; returns whether there were any (else the workspace
    /// cancels).
    pub fn drop_pending(&mut self) -> bool {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:1068 `escape`.
        todo!()
    }

    /// Delete or Backspace: removes the selected plane from `session`; planes torn off it keep
    /// their place as drawn planes, later parents' indices shift down. Returns the edit.
    pub fn delete_selected(&mut self, session: &mut VanishingPointSession, scene: &dyn VanishingScene) -> Option<PlaneEdit> {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:398 `delete_selected`.
        todo!()
    }

    /// After undo or redo: no drag, no pending corners, the selection clamped.
    pub fn after_step(&mut self, session: &VanishingPointSession) {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:383-396 `step`.
        todo!()
    }

    /// The editor as the `CanvasTool` of one pass over `session` (changed in place; each change is
    /// also reported).
    pub fn pass<'a>(
        &'a mut self,
        session: &'a mut VanishingPointSession,
        options: &'a VanishingPointOptions,
        scene: &'a dyn VanishingScene,
        source: &'a dyn VanishingPointSource,
        style: &'a VanishingPointStyle,
        labels: &'a VanishingPointLabels,
    ) -> VanishingPointPass<'a> {
        VanishingPointPass { editor: self, session, options, scene, source, style, labels }
    }

    pub fn take_events(&mut self) -> Vec<super::VanishingPointEvent> {
        std::mem::take(&mut self.events)
    }

    /// A drag or a stroke runs: the history waits.
    pub fn busy(&self) -> bool {
        self.drag.is_some()
    }
}

/// [`VanishingPointEditor`] as the `CanvasTool` of one pass. Node presses reach within
/// `vanishing_point.node` plus `canvas.hit` screen points; the selected plane's nodes win.
///
/// - **Create Plane**: four clicks in any order make a plane ([`VanishingScene::normalized`]),
///   selected; then Edit Plane takes over (as Photoshop). A rubber band runs from the last corner
///   to the pointer. Nothing past [`MAX_PLANES`](super::MAX_PLANES).
/// - **Edit Plane**: drag a corner node; drag an edge node to slide the edge
///   ([`slide_edge`](super::slide_edge)); a click selects the plane under it
///   ([`VanishingScene::plane_at`]) or none; on a torn plane only its far edge node drags
///   ([`fit_torn`](super::fit_torn): depth, with Alt the angle). ⌘/Ctrl on an edge node tears off
///   a perpendicular plane (angle 90, depth 0.5) and drags its far edge (with either plane tool).
///   A press on the pasted image moves it on the plane.
/// - **Stamp**: Alt-click sets the source; a press, drag, release paints a [`StampStroke`] with the
///   options (Aligned keeps the first stroke's offset).
/// - Events: [`VanishingPointEvent::Planes`](super::VanishingPointEvent::Planes) Begin, Update,
///   Commit for a drag and `Set` for a click that made, tore off or deleted a plane;
///   `Paste` for the image's drag; `Stamp` for a stroke.
/// - Keyboard parts: each plane's four corners ("Plane 1, top-left corner"); Page picks, arrows
///   move one document pixel (Shift ten), Delete removes the plane.
/// - Paint: [`draw::paint_editor`](super::draw::paint_editor).
pub struct VanishingPointPass<'a> {
    editor: &'a mut VanishingPointEditor,
    session: &'a mut VanishingPointSession,
    options: &'a VanishingPointOptions,
    scene: &'a dyn VanishingScene,
    source: &'a dyn VanishingPointSource,
    style: &'a VanishingPointStyle,
    labels: &'a VanishingPointLabels,
}

impl CanvasTool for VanishingPointPass<'_> {
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // new: the move cursor over nodes and the paste, the crosshair for Create Plane, the
        // stamp ring's (PhotoCraft draws none of its own).
        todo!()
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:658 `press`, :353 `add_corner`, :416 `tear_off`,
        // :469 `stamp_press`.
        todo!()
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:718 `drag_to`, :501 `stamp_to`.
        todo!()
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:1025 `pointer` (Up ends the drag; the history
        // settles).
        todo!()
    }

    fn hover(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // new: the rubber band and the stamp ring follow the pointer.
        todo!()
    }

    fn cancel(&mut self, cx: &mut ToolContext<'_>) {
        // new: Esc or a lost pointer ends the drag where it is (Cancel).
        todo!()
    }

    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:1045 `keys` (Delete, Backspace); arrows on the
        // picked corner are new (lead call 49).
        todo!()
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        super::draw::paint_editor(painter, cx.view, self.editor, self.session, self.options, self.scene, self.source, self.style);
    }

    fn parts(&self, cx: &ToolContext<'_>) -> Option<ToolParts> {
        // new: corners as keyboard parts (lead call 49).
        todo!()
    }

    fn picked(&self) -> Option<u64> {
        self.editor.picked
    }

    fn pick(&mut self, key: Option<u64>, cx: &mut ToolContext<'_>) {
        self.editor.picked = key;
    }
}

impl VanishingPointEditor {
    /// The corners clicked so far with Create Plane (for drawing).
    pub(super) fn pending(&self) -> &[[f64; 2]] {
        &self.pending
    }
}
