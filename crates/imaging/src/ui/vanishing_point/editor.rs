//! [`VanishingPointEditor`]: the plane tools, the paste and the stamp, without a frame (task
//! WS-VP1).

use egui::{CursorIcon, Modifiers, Painter, Rect, vec2};
use serde::{Deserialize, Serialize};

use craft_ui::behavior::nudge::NudgeKeys;
use craft_ui::canvas::kurbo::Point;
use craft_ui::canvas::quad::Homography;
use craft_ui::canvas::{CanvasTool, CursorHost, Part, ToolAction, ToolContext, ToolCursor, ToolKey, ToolParts, ToolPointer};
use craft_ui::edit::{CancelReason, EditEvent, GestureId};

use super::planes::{PlaneEdge, TornPlane, VanishingPlane, VanishingScene, fit_torn, slide_edge};
use super::style::VanishingPointStyle;
use super::{VanishingPointEvent, VanishingPointLabels, VanishingPointSource};

/// The most clone strokes and dabs a session holds (PhotoCraft `MAX_CLONE_STROKES`,
/// `MAX_CLONE_DABS`, engine `vp_cmds.rs:19-20`).
const MAX_STROKES: usize = 4096;
const MAX_DABS: usize = 200_000;
/// A stroke's dabs lie a tenth of the diameter apart, at least one document pixel (`stamp_to`).
const DAB_SPACING: f64 = 0.1;
const MIN_DAB_STEP: f64 = 1.0;

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
    /// What the drag changes, as it was at the press (Esc puts it back, Commit compares).
    before: Option<DragStart>,
    /// The pointer's last document position (the rubber band and the stamp ring follow it).
    pointer: Option<[f64; 2]>,
    picked: Option<u64>,
    events: Vec<VanishingPointEvent>,
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

/// The value a drag changes, as it was at the press.
#[derive(Clone, Debug)]
enum DragStart {
    Planes(PlaneEdit),
    Paste(PasteImage),
    /// The session's aligned offset before the stroke (the stroke is the last one).
    Stamp(Option<([f64; 2], [f64; 2])>),
}

/// What a press lands on in Edit Plane (and, for the tear-off, in Create Plane).
#[derive(Clone, Copy, Debug)]
enum Grab {
    Tear { plane: usize, edge: usize },
    Corner { plane: usize, corner: usize },
    Edge { plane: usize, edge: usize },
    Far { plane: usize },
    Paste { offset: [f64; 2] },
}

impl Default for VanishingPointEditor {
    fn default() -> Self {
        Self {
            tool: VanishingTool::CreatePlane,
            pending: Vec::new(),
            selected: None,
            drag: None,
            before: None,
            pointer: None,
            picked: None,
            events: Vec::new(),
        }
    }
}

fn at(p: [f64; 2]) -> Point {
    Point::new(p[0], p[1])
}

fn is_unset(plane: &VanishingPlane) -> bool {
    plane.corners.iter().flatten().all(|v| *v == 0.0)
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
        self.tool = tool;
        if tool != VanishingTool::CreatePlane {
            self.pending.clear();
        }
    }

    pub fn selected(&self) -> Option<usize> {
        self.selected
    }

    /// Esc: drops the pending corners; returns whether there were any (else the workspace
    /// cancels).
    pub fn drop_pending(&mut self) -> bool {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:1068 `escape`.
        let had = !self.pending.is_empty();
        self.pending.clear();
        had
    }

    /// Delete or Backspace: removes the selected plane from `session`; planes torn off it keep
    /// their place as drawn planes, later parents' indices shift down. Returns the edit, which is
    /// also reported (a `Planes` `Set`).
    pub fn delete_selected(&mut self, session: &mut VanishingPointSession, scene: &dyn VanishingScene) -> Option<PlaneEdit> {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:398 `delete_selected`.
        let i = self.selected.take().filter(|i| *i < session.planes.len())?;
        session.torn.resize(session.planes.len(), None);
        session.planes.remove(i);
        session.torn.remove(i);
        for t in &mut session.torn {
            match t {
                Some(c) if c.from == i => *t = None,
                Some(c) if c.from > i => c.from -= 1,
                _ => {}
            }
        }
        if !session.planes.is_empty() {
            session.planes = scene.resolve(&session.planes, &session.torn);
        }
        self.drag = None;
        self.before = None;
        self.picked = self.picked.filter(|k| usize::try_from(*k / 4).is_ok_and(|p| p < session.planes.len()));
        let edit = PlaneEdit { planes: session.planes.clone(), torn: session.torn.clone() };
        self.events.push(VanishingPointEvent::Planes(EditEvent::set(edit.clone(), Modifiers::NONE)));
        Some(edit)
    }

    /// After undo or redo: no drag, no pending corners, the selection clamped.
    pub fn after_step(&mut self, session: &VanishingPointSession) {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:383-396 `step`.
        self.drag = None;
        self.before = None;
        self.pending.clear();
        let n = session.planes.len();
        self.selected = self.selected.filter(|i| *i < n);
        self.picked = self.picked.filter(|k| usize::try_from(*k / 4).is_ok_and(|p| p < n));
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

    pub fn take_events(&mut self) -> Vec<VanishingPointEvent> {
        std::mem::take(&mut self.events)
    }

    /// A drag or a stroke runs: the history waits.
    pub fn busy(&self) -> bool {
        self.drag.is_some()
    }
}

/// [`VanishingPointEditor`] as the `CanvasTool` of one pass. Node presses reach half of
/// `vanishing_point.node` plus `handle.hit` screen points from a node's centre; the selected
/// plane's nodes win.
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
///   Commit for a drag (a tear-off included: Begin carries the new plane) and `Set` for a click
///   that made or deleted a plane; `Paste` for the image's drag; `Stamp` for a stroke (each
///   Update carries the stroke so far). Esc during a drag reports Cancel and puts back what the
///   press found. The Alt-click that sets the clone source is a session change with no event.
/// - Keyboard parts: each plane's four corners ("Plane 1, top-left corner"); Page picks (and
///   selects the plane), arrows move one document pixel (Shift ten), Delete removes the plane.
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

impl VanishingPointPass<'_> {
    /// How far from a node's centre a press reaches, in document units.
    fn reach(&self, cx: &ToolContext<'_>) -> f64 {
        cx.view.doc_len((self.style.node + cx.tokens.component.handle.hit) / 2.0)
    }

    fn plane_edit(&self) -> PlaneEdit {
        PlaneEdit { planes: self.session.planes.clone(), torn: self.session.torn.clone() }
    }

    fn push_planes(&mut self, event: EditEvent<PlaneEdit>) {
        self.editor.events.push(VanishingPointEvent::Planes(event));
    }

    /// Rebuilds the torn planes from their parents after an edit.
    fn resolve(&mut self) {
        self.session.torn.resize(self.session.planes.len(), None);
        if !self.session.planes.is_empty() {
            self.session.planes = self.scene.resolve(&self.session.planes, &self.session.torn);
        }
    }

    /// The plane's edge nodes; `None` for a plane with no map.
    fn nodes(plane: &VanishingPlane) -> Option<[[f64; 2]; 4]> {
        plane.homography().map(|_| plane.edge_nodes())
    }

    fn on_plane(&self, p: [f64; 2]) -> bool {
        self.scene.plane_at(&self.session.planes, p).is_some()
    }

    fn frame(&self, index: usize) -> Option<(Homography, f64, f64)> {
        let (h, lu, lv) = self.scene.frame(&self.session.planes, index)?;
        (lu.is_finite() && lv.is_finite() && lu > 1e-12 && lv > 1e-12).then_some((h, lu, lv))
    }

    /// The plane the paste lands on: the selected one, else the first.
    fn paste_plane(&self) -> Option<usize> {
        if !self.options.paste || self.source.paste().is_none() {
            return None;
        }
        let n = self.session.planes.len();
        self.editor.selected.filter(|i| *i < n).or((n > 0).then_some(0))
    }

    /// The pasted image's size in plane coordinates (it keeps its metric aspect).
    fn paste_extent(&self, lu: f64, lv: f64) -> Option<[f64; 2]> {
        let size = self.source.paste()?.size;
        let (w, h) = (f64::from(size.x), f64::from(size.y));
        let width = self.session.paste.width;
        Some([width, width * lu / lv * h / w.max(1.0)])
    }

    /// What a press at `p` lands on, in PhotoCraft's order: with ⌘ an edge node tears off; Create
    /// Plane grabs nothing else; then, selected plane first, a torn plane's far edge node or a
    /// drawn plane's corners and edge nodes; then the paste.
    fn grab_at(&self, p: [f64; 2], reach: f64, command: bool) -> Option<Grab> {
        let planes = &self.session.planes;
        let near = |c: [f64; 2]| (c[0] - p[0]).hypot(c[1] - p[1]) <= reach;
        let order: Vec<usize> = self.editor.selected.into_iter().chain((0..planes.len()).rev()).filter(|i| *i < planes.len()).collect();
        if command && let Some((plane, edge)) = order.iter().find_map(|&i| Some((i, Self::nodes(&planes[i])?.iter().position(|c| near(*c))?))) {
            return Some(Grab::Tear { plane, edge });
        }
        if self.editor.tool != VanishingTool::EditPlane {
            return None;
        }
        for &i in &order {
            // A torn plane follows its parent: only its far edge node drags.
            if let Some(t) = self.session.torn.get(i).copied().flatten() {
                let far = PlaneEdge::ALL.iter().position(|e| *e == t.edge)?;
                if Self::nodes(&planes[i]).is_some_and(|n| near(n[far])) {
                    return Some(Grab::Far { plane: i });
                }
                continue;
            }
            if let Some(corner) = planes[i].corners.iter().position(|c| near(*c)) {
                return Some(Grab::Corner { plane: i, corner });
            }
            if let Some(edge) = Self::nodes(&planes[i]).and_then(|n| n.iter().position(|c| near(*c))) {
                return Some(Grab::Edge { plane: i, edge });
            }
        }
        let i = self.paste_plane()?;
        let (h, lu, lv) = self.frame(i)?;
        let ext = self.paste_extent(lu, lv)?;
        let q = h.inverse()?.apply(at(p));
        let (a, b) = (self.session.paste.at, [self.session.paste.at[0] + ext[0], self.session.paste.at[1] + ext[1]]);
        ((a[0]..=b[0]).contains(&q.x) && (a[1]..=b[1]).contains(&q.y)).then_some(Grab::Paste { offset: [a[0] - q.x, a[1] - q.y] })
    }

    /// Starts a drag with its Begin event.
    fn begin(&mut self, drag: VanishingDrag, before: DragStart, event: impl FnOnce(GestureId) -> VanishingPointEvent) {
        let id = GestureId::next();
        self.editor.drag = Some((id, drag));
        self.editor.before = Some(before);
        self.editor.events.push(event(id));
    }

    fn begin_plane_drag(&mut self, drag: VanishingDrag) {
        let before = self.plane_edit();
        self.begin(drag, DragStart::Planes(before), |id| VanishingPointEvent::Planes(EditEvent::begin(id)));
    }

    /// Create Plane's click: the fourth corner makes the plane.
    fn add_corner(&mut self, p: [f64; 2], modifiers: Modifiers) {
        if self.session.planes.len() >= super::MAX_PLANES {
            return;
        }
        self.editor.pending.push(p);
        if let Ok(corners) = <[[f64; 2]; 4]>::try_from(self.editor.pending.as_slice()) {
            self.editor.pending.clear();
            let plane = self.scene.normalized(VanishingPlane { corners });
            self.session.torn.resize(self.session.planes.len(), None);
            self.session.planes.push(plane);
            self.session.torn.push(None);
            self.editor.selected = Some(self.session.planes.len() - 1);
            self.resolve();
            // Photoshop hands over to Edit Plane once a plane is closed.
            self.editor.tool = VanishingTool::EditPlane;
            let edit = self.plane_edit();
            self.push_planes(EditEvent::set(edit, modifiers));
        }
    }

    /// ⌘-press on an edge node: a perpendicular plane, half as deep, whose far edge the drag moves.
    fn tear_off(&mut self, from: usize, edge: usize, p: [f64; 2]) {
        if self.session.planes.len() >= super::MAX_PLANES {
            return;
        }
        let Some(&edge) = PlaneEdge::ALL.get(edge) else { return };
        let before = self.plane_edit();
        self.session.torn.resize(self.session.planes.len(), None);
        self.session.planes.push(VanishingPlane { corners: [[0.0; 2]; 4] });
        self.session.torn.push(Some(TornPlane { from, edge, angle: 90.0, depth: 0.5 }));
        let plane = self.session.planes.len() - 1;
        self.editor.selected = Some(plane);
        self.resolve();
        // As after closing a plane, Edit Plane takes over.
        self.editor.tool = VanishingTool::EditPlane;
        self.editor.pending.clear();
        let now = self.plane_edit();
        self.begin(VanishingDrag::Torn { plane, rotate: false, start: p, moved: false }, DragStart::Planes(before), |id| {
            VanishingPointEvent::Planes(EditEvent::begin_with(id, now))
        });
    }

    fn stamp_press(&mut self, p: [f64; 2], pointer: &ToolPointer) -> ToolAction {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:469 `stamp_press` (its status messages have no
        // channel here).
        if pointer.modifiers.alt {
            if self.on_plane(p) {
                self.session.source = Some(p);
                self.session.aligned_from = None;
            }
            return ToolAction::Done;
        }
        let Some(source) = self.session.source else { return ToolAction::Done };
        let dabs: usize = self.session.strokes.iter().map(|s| s.points.len()).sum();
        if !self.on_plane(p) || self.session.strokes.len() >= MAX_STROKES || dabs >= MAX_DABS {
            return ToolAction::Done;
        }
        let aligned = self.session.aligned_from.filter(|_| self.options.stamp.aligned);
        let src = match aligned {
            Some((s0, a0)) => self.scene.clone_source(&self.session.planes, s0, a0, p).filter(|s| self.on_plane(*s)).unwrap_or(source),
            None => source,
        };
        let before = self.session.aligned_from;
        if before.is_none() {
            self.session.aligned_from = Some((src, p));
        }
        let o = self.options.stamp;
        let stroke = StampStroke { source: src, points: vec![p], size: o.diameter, hardness: o.hardness / 100.0, opacity: o.opacity / 100.0 };
        self.session.strokes.push(stroke.clone());
        self.begin(VanishingDrag::Stamp, DragStart::Stamp(before), |id| VanishingPointEvent::Stamp(EditEvent::begin_with(id, stroke)));
        ToolAction::Claimed
    }

    /// Extends the running stroke to `p` with dabs a tenth of the brush apart.
    fn stamp_to(&mut self, p: [f64; 2]) -> bool {
        let used: usize = self.session.strokes.iter().map(|s| s.points.len()).sum();
        let room = MAX_DABS.saturating_sub(used);
        let Some(stroke) = self.session.strokes.last_mut() else { return false };
        let Some(last) = stroke.points.last().copied() else { return false };
        let step = (stroke.size * DAB_SPACING).max(MIN_DAB_STEP);
        let d = (p[0] - last[0]).hypot(p[1] - last[1]);
        if !d.is_finite() || d < step {
            return false;
        }
        // The count is a whole number of dabs: truncation is the intent.
        let n = ((d / step) as usize).min(room);
        stroke.points.extend((1..=n).map(|k| {
            let t = k as f64 * step / d;
            [last[0] + (p[0] - last[0]) * t, last[1] + (p[1] - last[1]) * t]
        }));
        n > 0
    }

    /// Moves what the drag holds to `p` and reports an Update when that changed something.
    fn drag_to(&mut self, p: [f64; 2], modifiers: Modifiers, dead: f64) {
        let Some((id, drag)) = self.editor.drag.clone() else { return };
        let event = match drag {
            VanishingDrag::Corner { plane, corner, offset } => {
                let to = [p[0] + offset[0], p[1] + offset[1]];
                match self.session.planes.get_mut(plane).and_then(|pl| pl.corners.get_mut(corner)) {
                    Some(c) if *c != to => {
                        *c = to;
                        self.resolve();
                        Some(VanishingPointEvent::Planes(EditEvent::update(id, self.plane_edit(), modifiers)))
                    }
                    _ => None,
                }
            }
            VanishingDrag::Torn { plane, rotate, start, moved } => {
                // A click without a drag leaves the plane as it is.
                let moved = moved || (p[0] - start[0]).hypot(p[1] - start[1]) > dead;
                if let Some((_, VanishingDrag::Torn { moved: m, .. })) = &mut self.editor.drag {
                    *m = moved;
                }
                let fit = moved.then(|| fit_torn(self.scene, &self.session.planes, &self.session.torn, plane, rotate, p)).flatten();
                match fit {
                    Some(t) if self.session.torn.get(plane).copied().flatten() != Some(t) => {
                        self.session.torn[plane] = Some(t);
                        self.resolve();
                        Some(VanishingPointEvent::Planes(EditEvent::update(id, self.plane_edit(), modifiers)))
                    }
                    _ => None,
                }
            }
            VanishingDrag::Edge { plane, edge, at_press } => match slide_edge(&at_press, edge, p) {
                Some(slid) if self.session.planes.get(plane).is_some_and(|pl| pl.corners != slid.corners) => {
                    self.session.planes[plane].corners = slid.corners;
                    self.resolve();
                    Some(VanishingPointEvent::Planes(EditEvent::update(id, self.plane_edit(), modifiers)))
                }
                _ => None,
            },
            VanishingDrag::Paste { offset } => {
                let inv = self.paste_plane().and_then(|i| self.session.planes.get(i)).and_then(VanishingPlane::homography).and_then(|h| h.inverse());
                inv.and_then(|inv| {
                    let q = inv.apply(at(p));
                    let to = [(q.x + offset[0]).clamp(-1.0, 2.0), (q.y + offset[1]).clamp(-1.0, 2.0)];
                    (to.iter().all(|v| v.is_finite()) && to != self.session.paste.at).then(|| {
                        self.session.paste.at = to;
                        VanishingPointEvent::Paste(EditEvent::update(id, self.session.paste, modifiers))
                    })
                })
            }
            VanishingDrag::Stamp => {
                if self.stamp_to(p) {
                    self.session.strokes.last().map(|s| VanishingPointEvent::Stamp(EditEvent::update(id, s.clone(), modifiers)))
                } else {
                    None
                }
            }
        };
        self.editor.events.extend(event);
    }

    /// The click-versus-drag distance of a torn plane's far edge node, in document units.
    fn dead_zone(&self, cx: &ToolContext<'_>) -> f64 {
        cx.view.doc_len(self.style.node / 2.0)
    }

    /// Whether `key` is a plane's corner that arrows may move: a drawn plane's.
    fn corner_of(&self, key: u64) -> Option<(usize, usize)> {
        let (plane, corner) = (usize::try_from(key / 4).ok()?, usize::try_from(key % 4).ok()?);
        (self.session.planes.get(plane).is_some_and(|p| !is_unset(p)) && self.session.torn.get(plane).copied().flatten().is_none())
            .then_some((plane, corner))
    }
}

impl CanvasTool for VanishingPointPass<'_> {
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        // new: the move cursor over nodes and the paste, the crosshair for Create Plane, the
        // stamp ring's (PhotoCraft draws none of its own).
        let crosshair = || {
            let (length, gap) = CursorHost::style(cx.tokens).crosshair;
            ToolCursor::Crosshair { length, gap }
        };
        let Some(pointer) = cx.pointer else { return crosshair() };
        let p = [pointer.doc.x, pointer.doc.y];
        match self.editor.tool {
            // The ring the editor paints is the cursor on a plane.
            VanishingTool::Stamp if self.on_plane(p) => ToolCursor::Hidden,
            VanishingTool::Stamp => crosshair(),
            tool => {
                if self.grab_at(p, self.reach(cx), cx.modifiers.command).is_some() {
                    ToolCursor::System(CursorIcon::Move)
                } else if tool == VanishingTool::CreatePlane {
                    crosshair()
                } else {
                    ToolCursor::System(CursorIcon::Default)
                }
            }
        }
    }

    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:658 `press`, :353 `add_corner`, :416 `tear_off`,
        // :469 `stamp_press`.
        let p = [pointer.doc.x, pointer.doc.y];
        if !p.iter().all(|v| v.is_finite()) {
            return ToolAction::Done;
        }
        self.editor.pointer = Some(p);
        if self.editor.tool == VanishingTool::Stamp {
            return self.stamp_press(p, pointer);
        }
        match self.grab_at(p, self.reach(cx), pointer.modifiers.command) {
            Some(Grab::Tear { plane, edge }) => {
                self.tear_off(plane, edge, p);
                return if self.editor.drag.is_some() { ToolAction::Claimed } else { ToolAction::Done };
            }
            Some(Grab::Corner { plane, corner }) => {
                let c = self.session.planes[plane].corners[corner];
                self.editor.selected = Some(plane);
                self.begin_plane_drag(VanishingDrag::Corner { plane, corner, offset: [c[0] - p[0], c[1] - p[1]] });
            }
            Some(Grab::Edge { plane, edge }) => {
                let Some(at_press) = self.session.planes[plane].homography() else { return ToolAction::Done };
                self.editor.selected = Some(plane);
                self.begin_plane_drag(VanishingDrag::Edge { plane, edge, at_press });
            }
            Some(Grab::Far { plane }) => {
                self.editor.selected = Some(plane);
                self.begin_plane_drag(VanishingDrag::Torn { plane, rotate: pointer.modifiers.alt, start: p, moved: false });
            }
            Some(Grab::Paste { offset }) => {
                let before = self.session.paste;
                self.begin(VanishingDrag::Paste { offset }, DragStart::Paste(before), |id| VanishingPointEvent::Paste(EditEvent::begin(id)));
            }
            None if self.editor.tool == VanishingTool::CreatePlane => {
                self.add_corner(p, pointer.modifiers);
                return ToolAction::Done;
            }
            None => {
                self.editor.selected = self.scene.plane_at(&self.session.planes, p);
                return ToolAction::Done;
            }
        }
        ToolAction::Claimed
    }

    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:718 `drag_to`, :501 `stamp_to`.
        let p = [pointer.doc.x, pointer.doc.y];
        if p.iter().all(|v| v.is_finite()) {
            self.editor.pointer = Some(p);
            let dead = self.dead_zone(cx);
            self.drag_to(p, pointer.modifiers, dead);
        }
    }

    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:1025 `pointer` (Up ends the drag; the history
        // settles).
        let p = [pointer.doc.x, pointer.doc.y];
        if p.iter().all(|v| v.is_finite()) {
            self.editor.pointer = Some(p);
            let dead = self.dead_zone(cx);
            self.drag_to(p, pointer.modifiers, dead);
        }
        let Some((id, drag)) = self.editor.drag.take() else { return };
        let modifiers = pointer.modifiers;
        let event = match (drag, self.editor.before.take()) {
            (VanishingDrag::Stamp, _) => {
                self.session.strokes.last().map(|s| VanishingPointEvent::Stamp(EditEvent::commit(id, s.clone(), true, modifiers)))
            }
            (VanishingDrag::Paste { .. }, Some(DragStart::Paste(before))) => {
                let now = self.session.paste;
                Some(VanishingPointEvent::Paste(EditEvent::commit(id, now, now != before, modifiers)))
            }
            (_, Some(DragStart::Planes(before))) => {
                let now = self.plane_edit();
                let changed = now != before;
                Some(VanishingPointEvent::Planes(EditEvent::commit(id, now, changed, modifiers)))
            }
            _ => None,
        };
        self.editor.events.extend(event);
    }

    fn hover(&mut self, pointer: &ToolPointer, _cx: &mut ToolContext<'_>) {
        // new: the rubber band and the stamp ring follow the pointer.
        let p = [pointer.doc.x, pointer.doc.y];
        self.editor.pointer = p.iter().all(|v| v.is_finite()).then_some(p);
    }

    fn cancel(&mut self, _cx: &mut ToolContext<'_>) {
        // new: Esc or a lost pointer abandons the drag and puts back what the press found.
        let Some((id, drag)) = self.editor.drag.take() else { return };
        let event = match (drag, self.editor.before.take()) {
            (VanishingDrag::Stamp, Some(DragStart::Stamp(aligned))) => {
                self.session.strokes.pop();
                self.session.aligned_from = aligned;
                Some(VanishingPointEvent::Stamp(EditEvent::cancel(id, CancelReason::Escape)))
            }
            (VanishingDrag::Paste { .. }, Some(DragStart::Paste(before))) => {
                self.session.paste = before;
                Some(VanishingPointEvent::Paste(EditEvent::cancel(id, CancelReason::Escape)))
            }
            (_, Some(DragStart::Planes(before))) => {
                self.session.planes = before.planes;
                self.session.torn = before.torn;
                let n = self.session.planes.len();
                self.editor.selected = self.editor.selected.filter(|i| *i < n);
                Some(VanishingPointEvent::Planes(EditEvent::cancel(id, CancelReason::Escape)))
            }
            _ => None,
        };
        self.editor.events.extend(event);
    }

    fn key(&mut self, key: &ToolKey, _cx: &mut ToolContext<'_>) -> bool {
        // pc: crates/ui-egui/src/dialogs/vp_ui.rs:1045 `keys` (Delete, Backspace); arrows on the
        // picked corner are new (lead call 49).
        if !key.pressed || self.editor.drag.is_some() {
            return false;
        }
        if matches!(key.key, egui::Key::Delete | egui::Key::Backspace) && key.modifiers.matches_exact(Modifiers::NONE) {
            return self.editor.delete_selected(self.session, self.scene).is_some();
        }
        let Some(nudge) = NudgeKeys::new().from_key(key) else { return false };
        let Some((plane, corner)) = self.editor.picked.and_then(|k| self.corner_of(k)) else { return false };
        let c = &mut self.session.planes[plane].corners[corner];
        *c = [c[0] + nudge.by.x, c[1] + nudge.by.y];
        self.resolve();
        let edit = self.plane_edit();
        self.push_planes(EditEvent::set(edit, key.modifiers));
        true
    }

    fn paint(&self, painter: &Painter, cx: &ToolContext<'_>) {
        super::draw::paint_editor(painter, cx.view, self.editor, self.session, self.options, self.scene, self.source, self.style);
    }

    fn parts(&self, cx: &ToolContext<'_>) -> Option<ToolParts> {
        // new: corners as keyboard parts (lead call 49).
        if self.session.planes.is_empty() {
            return None;
        }
        let corners = [&self.labels.top_left, &self.labels.top_right, &self.labels.bottom_right, &self.labels.bottom_left];
        let size = cx.tokens.component.handle.hit;
        let parts = self
            .session
            .planes
            .iter()
            .enumerate()
            .filter(|(_, plane)| !is_unset(plane))
            .flat_map(|(i, plane)| plane.corners.iter().zip(corners).enumerate().map(move |(c, (at, name))| (i, c, *at, name)))
            .map(|(i, c, corner, name)| {
                let key = (i * 4 + c) as u64;
                let centre = cx.view.to_screen(at(corner));
                Part {
                    key,
                    name: craft_i18n::fmt(&self.labels.corner_part, &[("n", &(i + 1).to_string()), ("corner", name)]),
                    value: Some(format!("{}, {}", cx.numbers.format(corner[0], 0..=1), cx.numbers.format(corner[1], 0..=1))),
                    rect: Rect::from_center_size(centre, vec2(size, size)),
                    selected: self.editor.picked == Some(key),
                }
            })
            .collect();
        Some(ToolParts::new(self.labels.name.clone(), parts))
    }

    fn picked(&self) -> Option<u64> {
        self.editor.picked
    }

    fn pick(&mut self, key: Option<u64>, _cx: &mut ToolContext<'_>) {
        self.editor.picked = key;
        if let Some(plane) = key.and_then(|k| usize::try_from(k / 4).ok()).filter(|p| *p < self.session.planes.len()) {
            self.editor.selected = Some(plane);
        }
    }
}

impl VanishingPointEditor {
    /// The corners clicked so far with Create Plane (for drawing).
    pub(super) fn pending(&self) -> &[[f64; 2]] {
        &self.pending
    }

    /// The pointer's last position in document pixels, for the rubber band and the stamp ring.
    pub(super) fn pointer(&self) -> Option<[f64; 2]> {
        self.pointer
    }
}
