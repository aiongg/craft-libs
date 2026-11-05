//! Tests of Vanishing Point's planes and editor, on a `ToolHost` with a fake scene.

#[cfg(feature = "screen-testing")]
use egui::{Key, pos2};
use egui::{Modifiers, Rect, Sense, vec2};
use egui_kittest::Harness;

use craft_ui::canvas::kurbo::Point;
use craft_ui::canvas::quad::Homography;
use craft_ui::canvas::{
    CanvasId, CanvasTool, CanvasView, PixelRect, RasterSource, ToolAction, ToolContext, ToolCursor, ToolHost, ToolInput, ToolKey, ToolParts,
    ToolPointer,
};
use craft_ui::dialog::{LocalHistory, Step};
use craft_ui::edit::CancelReason;
use craft_ui::edit::EditEvent;
use craft_ui::localized::Labels as _;
#[cfg(feature = "screen-testing")]
use craft_ui::testing::canvas as rig;
use craft_ui::theme::{Base, current, install};

use super::editor::paste_extent;
use super::planes::{PlaneCheck, PlaneEdge, TornPlane, VanishingPlane, VanishingScene, slide_edge};
use super::{
    MAX_PLANES, StampLimits, VanishingPointEditor, VanishingPointEvent, VanishingPointLabels, VanishingPointSession, VanishingPointSettings,
    VanishingPointSource, VanishingPointStyle, VanishingTool,
};

/// The engine's clone limits, as the app gives them.
const LIMITS: StampLimits = StampLimits { strokes: 4096, dabs: 200_000 };

/// A scene whose perspective is flat: torn planes are rectangles on the parent's edge, turned by
/// the angle. It has the contract of the app's scene, not its maths.
struct Flat;

fn turns(q: &[[f64; 2]; 4]) -> f64 {
    let e = |i: usize| [q[(i + 1) % 4][0] - q[i][0], q[(i + 1) % 4][1] - q[i][1]];
    (0..4).map(|i| e(i)[0] * e((i + 1) % 4)[1] - e(i)[1] * e((i + 1) % 4)[0]).sum()
}

impl VanishingScene for Flat {
    fn normalized(&self, plane: VanishingPlane) -> VanishingPlane {
        let mut q = plane.corners;
        if turns(&q) < 0.0 {
            q = [q[0], q[3], q[2], q[1]];
        }
        let first = (0..4).min_by(|&a, &b| (q[a][0] + q[a][1]).total_cmp(&(q[b][0] + q[b][1]))).unwrap_or(0);
        VanishingPlane { corners: [q[first], q[(first + 1) % 4], q[(first + 2) % 4], q[(first + 3) % 4]] }
    }

    fn resolve(&self, planes: &[VanishingPlane], torn: &[Option<TornPlane>]) -> Vec<VanishingPlane> {
        let mut out: Vec<VanishingPlane> = Vec::new();
        for (i, plane) in planes.iter().enumerate() {
            let Some(t) = torn.get(i).copied().flatten().filter(|t| t.from < i) else {
                out.push(*plane);
                continue;
            };
            let c = out[t.from].corners;
            let e = PlaneEdge::ALL.iter().position(|x| *x == t.edge).unwrap_or(0);
            let (a, b, inward) = (c[e], c[(e + 1) % 4], c[[3, 0, 1, 2][e]]);
            let v = [inward[0] - a[0], inward[1] - a[1]];
            let len = v[0].hypot(v[1]);
            let turn = (t.angle - 90.0).to_radians();
            let o = [-v[0] / len, -v[1] / len];
            let o = [o[0] * turn.cos() - o[1] * turn.sin(), o[0] * turn.sin() + o[1] * turn.cos()];
            let far = |p: [f64; 2]| [p[0] + o[0] * len * t.depth, p[1] + o[1] * len * t.depth];
            let mut corners = [[0.0; 2]; 4];
            corners[e] = far(a);
            corners[(e + 1) % 4] = far(b);
            corners[(e + 2) % 4] = b;
            corners[(e + 3) % 4] = a;
            out.push(VanishingPlane { corners });
        }
        out
    }

    fn check(&self, _: &[VanishingPlane], _: usize) -> PlaneCheck {
        PlaneCheck::Valid
    }

    fn plane_at(&self, planes: &[VanishingPlane], at: [f64; 2]) -> Option<usize> {
        let inside = |q: &[[f64; 2]; 4]| {
            let side = |i: usize| {
                let (a, b) = (q[i], q[(i + 1) % 4]);
                (b[0] - a[0]) * (at[1] - a[1]) - (b[1] - a[1]) * (at[0] - a[0])
            };
            (0..4).all(|i| side(i) >= 0.0) || (0..4).all(|i| side(i) <= 0.0)
        };
        planes.iter().rposition(|p| inside(&p.corners))
    }

    fn frame(&self, planes: &[VanishingPlane], index: usize) -> Option<(Homography, f64, f64)> {
        let p = planes.get(index)?;
        let len = |a: [f64; 2], b: [f64; 2]| (a[0] - b[0]).hypot(a[1] - b[1]);
        Some((p.homography()?, len(p.corners[0], p.corners[1]), len(p.corners[0], p.corners[3])))
    }

    fn stamp_ring(&self, _: &[VanishingPlane], _: [f64; 2], _: f64) -> Option<Vec<Point>> {
        None
    }

    fn clone_source(&self, planes: &[VanishingPlane], source: [f64; 2], anchor: [f64; 2], at: [f64; 2]) -> Option<[f64; 2]> {
        self.plane_at(planes, source)?;
        Some([source[0] + at[0] - anchor[0], source[1] + at[1] - anchor[1]])
    }
}

struct Blank;

impl RasterSource for Blank {
    fn size(&self) -> [u32; 2] {
        [400, 300]
    }
    fn revision(&self) -> u64 {
        1
    }
    fn read(&self, _: PixelRect, _: u32, _: &mut egui::ColorImage) {}
}

impl VanishingPointSource for Blank {
    fn image(&self) -> &dyn RasterSource {
        self
    }
    fn stamp_preview(&self) -> Option<&dyn RasterSource> {
        None
    }
    fn paste(&self) -> Option<egui::load::SizedTexture> {
        None
    }
}

/// The editor's pass without its painting (`draw` is another task's).
struct Quiet<'a>(super::VanishingPointPass<'a>);

impl CanvasTool for Quiet<'_> {
    fn cursor(&self, cx: &ToolContext<'_>) -> ToolCursor {
        self.0.cursor(cx)
    }
    fn press(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) -> ToolAction {
        self.0.press(pointer, cx)
    }
    fn drag(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        self.0.drag(pointer, cx);
    }
    fn release(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        self.0.release(pointer, cx);
    }
    fn hover(&mut self, pointer: &ToolPointer, cx: &mut ToolContext<'_>) {
        self.0.hover(pointer, cx);
    }
    fn cancel(&mut self, reason: CancelReason, cx: &mut ToolContext<'_>) {
        self.0.cancel(reason, cx);
    }
    fn key(&mut self, key: &ToolKey, cx: &mut ToolContext<'_>) -> bool {
        self.0.key(key, cx)
    }
    fn paint(&self, _: &egui::Painter, _: &ToolContext<'_>) {}
    fn parts(&self, cx: &ToolContext<'_>) -> Option<ToolParts> {
        self.0.parts(cx)
    }
    fn picked(&self) -> Option<u64> {
        self.0.picked()
    }
    fn pick(&mut self, key: Option<u64>, cx: &mut ToolContext<'_>) {
        self.0.pick(key, cx);
    }
}

struct Host {
    editor: VanishingPointEditor,
    session: VanishingPointSession,
    settings: VanishingPointSettings,
    history: LocalHistory<VanishingPointSession>,
    view: CanvasView,
    canvas: Option<CanvasId>,
    events: Vec<VanishingPointEvent>,
    /// Gives the canvas the keyboard in the next pass (a press does not: the app focuses it).
    focus_canvas: bool,
}

fn harness(tool: VanishingTool, session: VanishingPointSession) -> Harness<'static, Host> {
    let rect = Rect::from_min_size(egui::Pos2::ZERO, vec2(400.0, 300.0));
    let mut editor = VanishingPointEditor::new(LIMITS);
    editor.set_tool(tool);
    let host = Host {
        editor,
        history: LocalHistory::new(&session),
        session,
        settings: VanishingPointSettings::default(),
        view: CanvasView::new(rect, Point::new(200.0, 150.0), 1.0),
        canvas: None,
        events: Vec::new(),
        focus_canvas: false,
    };
    let mut h = Harness::builder()
        .with_size(vec2(400.0, 300.0))
        .with_step_dt(0.05)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, host: &mut Host| {
                let id = CanvasId::new(ui, "vp");
                host.canvas = Some(id);
                let canvas = id.interact(ui, host.view.rect, Sense::click_and_drag() | Sense::FOCUSABLE);
                if std::mem::take(&mut host.focus_canvas) {
                    canvas.request_focus();
                }
                let style = VanishingPointStyle::of(&current(ui.ctx()));
                {
                    let mut pass = Quiet(host.editor.pass(&mut host.session, &host.settings, &Flat, &Blank, &style));
                    let _ = ToolHost::new(id).show(ui, &host.view, &canvas, &mut pass);
                }
                host.history.settle(&host.session, host.editor.busy());
                host.events.extend(host.editor.take_events());
            },
            host,
        );
    h.step();
    h
}

fn input(h: &mut Harness<'_, Host>, input: ToolInput) {
    let Some(id) = h.state().canvas else { panic!("a pass ran") };
    ToolHost::inject(&h.ctx, id, input);
    h.step();
}

fn down(h: &mut Harness<'_, Host>, at: (f64, f64), modifiers: Modifiers) {
    input(
        h,
        ToolInput::Down {
            button: egui::PointerButton::Primary,
            doc: Point::new(at.0, at.1),
            pressure: 1.0,
            tilt: [0.0; 2],
            rotation: 0.0,
            modifiers,
        },
    );
}

fn to(h: &mut Harness<'_, Host>, at: (f64, f64), modifiers: Modifiers) {
    input(h, ToolInput::Move { doc: Point::new(at.0, at.1), pressure: 1.0, tilt: [0.0; 2], rotation: 0.0, modifiers });
}

fn up(h: &mut Harness<'_, Host>, at: (f64, f64), modifiers: Modifiers) {
    input(h, ToolInput::Up { doc: Point::new(at.0, at.1), modifiers });
}

fn click(h: &mut Harness<'_, Host>, at: (f64, f64), modifiers: Modifiers) {
    down(h, at, modifiers);
    up(h, at, modifiers);
}

fn drag(h: &mut Harness<'_, Host>, from: (f64, f64), via: (f64, f64), modifiers: Modifiers) {
    down(h, from, modifiers);
    to(h, via, modifiers);
    up(h, via, modifiers);
}

fn rect_plane(x0: f64, y0: f64, x1: f64, y1: f64) -> VanishingPlane {
    VanishingPlane { corners: [[x0, y0], [x1, y0], [x1, y1], [x0, y1]] }
}

fn with_planes(planes: Vec<VanishingPlane>) -> VanishingPointSession {
    VanishingPointSession { torn: vec![None; planes.len()], planes, ..VanishingPointSession::default() }
}

fn phases(h: &Harness<'_, Host>) -> Vec<&'static str> {
    h.state()
        .events
        .iter()
        .map(|e| match e {
            VanishingPointEvent::Planes(e) => phase("planes", e),
            VanishingPointEvent::Paste(e) => phase("paste", e),
            VanishingPointEvent::Stamp(e) => phase("stamp", e),
        })
        .collect()
}

fn phase<T>(kind: &'static str, event: &EditEvent<T>) -> &'static str {
    match (kind, event) {
        ("planes", EditEvent::Begin { .. }) => "planes begin",
        ("planes", EditEvent::Update { .. }) => "planes update",
        ("planes", EditEvent::Commit { .. }) => "planes commit",
        ("planes", EditEvent::Cancel { .. }) => "planes cancel",
        ("planes", EditEvent::Set { .. }) => "planes set",
        ("stamp", EditEvent::Begin { .. }) => "stamp begin",
        ("stamp", EditEvent::Update { .. }) => "stamp update",
        ("stamp", EditEvent::Commit { .. }) => "stamp commit",
        _ => "other",
    }
}

fn near(a: [f64; 2], b: [f64; 2]) -> bool {
    (a[0] - b[0]).hypot(a[1] - b[1]) < 1e-6
}

// Four clicks in any order make one plane (normalized), selected, and the tool turns to Edit Plane.
#[test]
fn vp_four_clicks_make_a_plane_in_any_order() {
    for order in [[(300.0, 200.0), (300.0, 100.0), (100.0, 100.0), (100.0, 200.0)], [(100.0, 200.0), (100.0, 100.0), (300.0, 100.0), (300.0, 200.0)]]
    {
        let mut h = harness(VanishingTool::CreatePlane, VanishingPointSession::default());
        for (n, at) in order.into_iter().enumerate() {
            click(&mut h, at, Modifiers::NONE);
            assert_eq!(h.state().editor.pending().len(), if n == 3 { 0 } else { n + 1 });
        }
        let s = h.state();
        assert_eq!(s.session.planes.len(), 1);
        assert_eq!(s.session.planes[0], rect_plane(100.0, 100.0, 300.0, 200.0), "top-left first, clockwise");
        assert_eq!((s.editor.selected(), s.editor.tool()), (Some(0), VanishingTool::EditPlane));
        assert_eq!(phases(&h), ["planes set"], "only the fourth click reports");
    }
}

// Dragging an edge node slides the edge along the plane's perspective at the press.
#[test]
fn vp_edge_node_slides_along_the_planes_perspective() {
    let plane = VanishingPlane { corners: [[100.0, 100.0], [300.0, 120.0], [280.0, 220.0], [120.0, 200.0]] };
    let node = plane.edge_nodes()[0];
    let mut h = harness(VanishingTool::EditPlane, with_planes(vec![plane]));
    drag(&mut h, (node[0], node[1]), (node[0], node[1] - 40.0), Modifiers::NONE);
    let c = h.state().session.planes[0].corners;
    assert!(near(c[2], plane.corners[2]) && near(c[3], plane.corners[3]), "the bottom edge stays: {c:?}");
    // The new top corners stay on the old left and right edges.
    let off = |p: [f64; 2], a: [f64; 2], b: [f64; 2]| ((b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])).abs();
    assert!(off(c[0], plane.corners[0], plane.corners[3]) < 1e-6 && off(c[1], plane.corners[1], plane.corners[2]) < 1e-6, "{c:?}");
    assert!(c[0][1] < plane.corners[0][1], "the edge moved up: {c:?}");
    assert_eq!((phases(&h).first().copied(), phases(&h).last().copied()), (Some("planes begin"), Some("planes commit")));
    // Dragging the node to where it is leaves the plane as it was.
    let at_press = plane.homography().expect("a valid plane");
    let same = slide_edge(&Flat, &at_press, 0, node).expect("the node's own place");
    assert!(same.corners.iter().zip(plane.corners).all(|(a, b)| near(*a, b)), "{same:?}");
}

// ⌘-drag on an edge node tears off a plane at 90° and the drag sets its depth; with Alt the far
// edge sets its angle.
#[test]
fn vp_ctrl_drag_tears_off_a_plane_and_far_edge_sets_depth_or_angle() {
    let mut h = harness(VanishingTool::EditPlane, with_planes(vec![rect_plane(100.0, 100.0, 300.0, 200.0)]));
    drag(&mut h, (200.0, 200.0), (200.0, 300.0), Modifiers::COMMAND);
    let Some(t) = h.state().session.torn[1] else { panic!("plane 1 was torn off") };
    assert_eq!((t.from, t.edge, t.angle), (0, PlaneEdge::Bottom, 90.0));
    assert!((t.depth - 1.0).abs() < 0.02, "depth {}", t.depth);
    assert_eq!(h.state().session.planes.len(), 2);
    assert_eq!((h.state().editor.selected(), h.state().editor.tool()), (Some(1), VanishingTool::EditPlane));
    assert_eq!(phases(&h).first().copied(), Some("planes begin"));
    assert_eq!(phases(&h).last().copied(), Some("planes commit"));

    // Alt on the far edge node turns the plane about the edge instead.
    let far = h.state().session.planes[1].edge_nodes()[2];
    drag(&mut h, (far[0], far[1]), (200.0 - 70.7, 200.0 + 70.7), Modifiers::ALT);
    let Some(t) = h.state().session.torn[1] else { panic!("still torn") };
    assert!((t.angle - 135.0).abs() <= 1.5, "angle {}", t.angle);
}

// Delete removes the selected plane, torn children become drawn planes; Esc drops pending corners
// before it cancels.
#[test]
fn vp_delete_removes_the_plane_and_esc_drops_a_half_plane_first() {
    let drawn = [rect_plane(100.0, 50.0, 300.0, 100.0), rect_plane(0.0, 0.0, 0.0, 0.0), rect_plane(0.0, 0.0, 0.0, 0.0)];
    let torn = vec![
        None,
        Some(TornPlane { from: 0, edge: PlaneEdge::Bottom, angle: 90.0, depth: 1.0 }),
        Some(TornPlane { from: 1, edge: PlaneEdge::Bottom, angle: 90.0, depth: 1.0 }),
    ];
    let planes = Flat.resolve(&drawn, &torn);
    let mut h = harness(VanishingTool::EditPlane, VanishingPointSession { planes: planes.clone(), torn, ..VanishingPointSession::default() });
    click(&mut h, (200.0, 75.0), Modifiers::NONE);
    assert_eq!(h.state().editor.selected(), Some(0));
    let s = h.state_mut();
    let Some(edit) = s.editor.delete_selected(&mut s.session, &Flat) else { panic!("a plane was selected") };
    assert_eq!(edit.planes, s.session.planes);
    assert_eq!(s.session.planes, planes[1..], "the children keep their place");
    assert_eq!(
        s.session.torn,
        [None, Some(TornPlane { from: 0, edge: PlaneEdge::Bottom, angle: 90.0, depth: 1.0 })],
        "the child is drawn, its child follows it"
    );
    assert_eq!(s.editor.selected(), None);
    assert!(matches!(s.editor.take_events().as_slice(), [VanishingPointEvent::Planes(EditEvent::Set { .. })]));

    // Esc: the half plane first, then the caller cancels.
    let mut h = harness(VanishingTool::CreatePlane, VanishingPointSession::default());
    click(&mut h, (10.0, 10.0), Modifiers::NONE);
    click(&mut h, (50.0, 10.0), Modifiers::NONE);
    assert_eq!(h.state().editor.pending().len(), 2);
    assert!(h.state_mut().editor.drop_pending());
    assert!(h.state().editor.pending().is_empty());
    assert!(!h.state_mut().editor.drop_pending(), "nothing pending: Esc cancels");
}

// Alt-click sets the source; strokes report Begin, Update, Commit with the settings; Aligned keeps
// the first stroke's offset.
#[test]
fn vp_stamp_alt_click_sets_source_and_strokes_report_events() {
    let mut h = harness(VanishingTool::Stamp, with_planes(vec![rect_plane(50.0, 50.0, 350.0, 250.0)]));
    click(&mut h, (20.0, 20.0), Modifiers::ALT);
    assert_eq!(h.state().session.source, None, "off every plane");
    let words = VanishingPointLabels::of_ctx(&h.ctx);
    assert_eq!(h.state_mut().editor.take_status().as_deref(), Some(words.stamp_no_source.as_str()), "the press says why");
    click(&mut h, (150.0, 150.0), Modifiers::ALT);
    assert_eq!(h.state().session.source, Some([150.0, 150.0]));

    drag(&mut h, (200.0, 200.0), (240.0, 200.0), Modifiers::NONE);
    assert_eq!(phases(&h).iter().filter(|p| p.starts_with("stamp")).copied().collect::<Vec<_>>().first().copied(), Some("stamp begin"));
    assert_eq!(phases(&h).last().copied(), Some("stamp commit"));
    let s = &h.state().session;
    let stroke = &s.strokes[0];
    assert_eq!((stroke.source, stroke.size, stroke.hardness, stroke.opacity), ([150.0, 150.0], 40.0, 0.5, 1.0));
    assert_eq!(stroke.points.len(), 11, "a dab every tenth of the diameter");
    assert_eq!(stroke.points[0], [200.0, 200.0]);

    // Aligned: the next stroke keeps the offset from its start; without Aligned it restarts at the source.
    drag(&mut h, (250.0, 200.0), (260.0, 200.0), Modifiers::NONE);
    assert_eq!(h.state().session.strokes[1].source, [200.0, 150.0]);
    h.state_mut().settings.stamp.aligned = false;
    drag(&mut h, (250.0, 200.0), (260.0, 200.0), Modifiers::NONE);
    assert_eq!(h.state().session.strokes[2].source, [150.0, 150.0]);
}

// Undo steps back through a stroke, a tear-off and a plane, in that order.
#[test]
fn vp_undo_steps_back_through_planes_paste_and_strokes() {
    let mut h = harness(VanishingTool::CreatePlane, VanishingPointSession::default());
    for at in [(100.0, 100.0), (300.0, 100.0), (300.0, 200.0), (100.0, 200.0)] {
        click(&mut h, at, Modifiers::NONE);
    }
    drag(&mut h, (200.0, 200.0), (200.0, 280.0), Modifiers::COMMAND);
    h.state_mut().editor.set_tool(VanishingTool::Stamp);
    click(&mut h, (150.0, 150.0), Modifiers::ALT);
    drag(&mut h, (200.0, 150.0), (230.0, 150.0), Modifiers::NONE);
    let counts = |h: &Harness<'_, Host>| (h.state().session.planes.len(), h.state().session.source.is_some(), h.state().session.strokes.len());
    assert_eq!(counts(&h), (2, true, 1));

    let step = |h: &mut Harness<'_, Host>, step: Step| {
        let s = h.state_mut();
        assert!(s.history.step(&mut s.session, step));
        s.editor.after_step(&s.session);
    };
    step(&mut h, Step::Undo);
    assert_eq!(counts(&h), (2, true, 0), "the stroke");
    step(&mut h, Step::Undo);
    assert_eq!(counts(&h), (2, false, 0), "the source");
    step(&mut h, Step::Undo);
    assert_eq!(counts(&h), (1, false, 0), "the tear-off");
    step(&mut h, Step::Undo);
    assert_eq!(counts(&h), (0, false, 0), "the plane");
    assert_eq!(h.state().editor.selected(), None);
    step(&mut h, Step::Redo);
    assert_eq!(counts(&h), (1, false, 0));
}

// At 64 planes Create Plane and tear-off add nothing.
#[test]
fn vp_planes_are_capped_at_sixty_four() {
    let planes: Vec<VanishingPlane> = (0..MAX_PLANES)
        .map(|i| if i == 0 { rect_plane(100.0, 100.0, 300.0, 200.0) } else { rect_plane(1000.0 * i as f64, 0.0, 1000.0 * i as f64 + 50.0, 50.0) })
        .collect();
    let mut h = harness(VanishingTool::CreatePlane, with_planes(planes));
    for at in [(10.0, 250.0), (60.0, 250.0), (60.0, 290.0), (10.0, 290.0)] {
        click(&mut h, at, Modifiers::NONE);
    }
    assert_eq!((h.state().session.planes.len(), h.state().editor.pending().len()), (MAX_PLANES, 0));
    h.state_mut().editor.set_tool(VanishingTool::EditPlane);
    drag(&mut h, (200.0, 200.0), (200.0, 280.0), Modifiers::COMMAND);
    assert_eq!(h.state().session.planes.len(), MAX_PLANES);
    assert!(!h.state().editor.busy());
    assert!(h.state().events.is_empty(), "{:?}", phases(&h));
}

/// A scene that gives every plane the same verdict (the contract of `check` for `Invalid`).
struct Verdict(PlaneCheck);

impl VanishingScene for Verdict {
    fn normalized(&self, plane: VanishingPlane) -> VanishingPlane {
        Flat.normalized(plane)
    }
    fn resolve(&self, planes: &[VanishingPlane], torn: &[Option<TornPlane>]) -> Vec<VanishingPlane> {
        Flat.resolve(planes, torn)
    }
    fn check(&self, _: &[VanishingPlane], _: usize) -> PlaneCheck {
        self.0
    }
    fn plane_at(&self, planes: &[VanishingPlane], at: [f64; 2]) -> Option<usize> {
        Flat.plane_at(planes, at)
    }
    fn frame(&self, planes: &[VanishingPlane], index: usize) -> Option<(Homography, f64, f64)> {
        Flat.frame(planes, index)
    }
    fn stamp_ring(&self, planes: &[VanishingPlane], at: [f64; 2], diameter: f64) -> Option<Vec<Point>> {
        Flat.stamp_ring(planes, at, diameter)
    }
    fn clone_source(&self, planes: &[VanishingPlane], source: [f64; 2], anchor: [f64; 2], at: [f64; 2]) -> Option<[f64; 2]> {
        Flat.clone_source(planes, source, anchor, at)
    }
}

// Sliding an edge asks the scene for the verdict: the slid plane is refused when the scene calls
// it Invalid, and nothing else decides.
#[test]
fn vp_slide_edge_takes_the_scenes_verdict() {
    let plane = rect_plane(100.0, 100.0, 300.0, 200.0);
    let at_press = plane.homography().expect("a valid plane");
    let node = plane.edge_nodes()[0];
    let up = [node[0], node[1] - 30.0];
    assert!(slide_edge(&Verdict(PlaneCheck::Valid), &at_press, 0, up).is_some());
    assert!(slide_edge(&Verdict(PlaneCheck::Questionable), &at_press, 0, up).is_some());
    assert!(slide_edge(&Verdict(PlaneCheck::Invalid), &at_press, 0, up).is_none());
}

// The app's limits stop the Stamp tool: a press past them paints nothing and says why, in the
// words the editor has.
#[test]
fn vp_stamp_stops_at_the_apps_limits_and_says_why() {
    let mut h = harness(
        VanishingTool::Stamp,
        VanishingPointSession { source: Some([150.0, 150.0]), ..with_planes(vec![rect_plane(50.0, 50.0, 350.0, 250.0)]) },
    );
    h.state_mut().editor = VanishingPointEditor::new(StampLimits { strokes: 1, dabs: 200_000 });
    h.state_mut().editor.set_tool(VanishingTool::Stamp);
    drag(&mut h, (200.0, 200.0), (240.0, 200.0), Modifiers::NONE);
    assert_eq!(h.state().session.strokes.len(), 1);
    assert_eq!(h.state_mut().editor.take_status(), None);
    drag(&mut h, (200.0, 100.0), (240.0, 100.0), Modifiers::NONE);
    assert_eq!(h.state().session.strokes.len(), 1, "the second stroke is refused");
    let words = VanishingPointLabels::of_ctx(&h.ctx);
    assert_eq!(h.state_mut().editor.take_status().as_deref(), Some(words.stamp_too_many.as_str()));
}

// The tool and the drawing share one paste extent: `width` of the plane, the height from the
// image's aspect in the plane's metric.
#[test]
fn vp_paste_extent_keeps_the_images_metric_aspect() {
    assert_eq!(paste_extent(vec2(40.0, 30.0), 0.5, 200.0, 100.0), [0.5, 0.75]);
    assert_eq!(paste_extent(vec2(0.0, 30.0), 0.5, 100.0, 100.0)[0], 0.5, "an empty image does not divide by zero");
}

// A selection outside the session's planes selects nothing.
#[test]
fn vp_select_clamps_to_the_sessions_planes() {
    let session = with_planes(vec![rect_plane(0.0, 0.0, 10.0, 10.0), rect_plane(20.0, 0.0, 30.0, 10.0)]);
    let mut editor = VanishingPointEditor::new(LIMITS);
    assert_eq!(editor.select(Some(1), &session).selected(), Some(1));
    assert_eq!(editor.select(Some(2), &session).selected(), None);
}

// A gesture the app cancels reports `Cancel` with the app's reason, puts the plane back and ends
// the busy state: the Removed of a tool switch and the UndoDuringGesture of ⌘Z.
#[cfg(feature = "screen-testing")]
#[test]
fn vp_a_gesture_the_app_cancels_reports_its_reason_and_ends() {
    for reason in [CancelReason::Removed, CancelReason::UndoDuringGesture] {
        let plane = rect_plane(100.0, 100.0, 300.0, 200.0);
        let mut h = harness(VanishingTool::EditPlane, with_planes(vec![plane]));
        rig::press(&mut h, pos2(100.0, 100.0), Modifiers::NONE);
        rig::move_to(&mut h, pos2(140.0, 130.0));
        assert!(h.state().editor.busy(), "the corner drag runs");
        assert_ne!(h.state().session.planes[0], plane);
        let (ctx, id) = (h.ctx.clone(), h.state().canvas.expect("a pass ran"));
        {
            let style = VanishingPointStyle::of(&current(&ctx));
            let host = h.state_mut();
            ToolHost::cancel(&ctx, id, &mut host.editor.pass(&mut host.session, &host.settings, &Flat, &Blank, &style), reason);
        }
        h.step();
        assert!(!h.state().editor.busy());
        assert_eq!(h.state().session.planes[0], plane, "the press's plane is back");
        let cancelled =
            h.state().events.iter().any(|e| matches!(e, VanishingPointEvent::Planes(EditEvent::Cancel { reason: r, .. }) if *r == reason));
        assert!(cancelled, "{:?}", h.state().events);
    }
}

// The arrows move the picked corner only while the corners' group has the keyboard; with the
// canvas focused they do nothing.
#[cfg(feature = "screen-testing")]
#[test]
fn vp_arrows_move_the_picked_corner_only_while_the_corners_have_the_keyboard() {
    let plane = rect_plane(100.0, 100.0, 300.0, 200.0);
    let mut h = harness(VanishingTool::EditPlane, with_planes(vec![plane]));
    // Focus the canvas, move the keyboard to the group (the next Tab stop) and pick the first corner.
    h.state_mut().focus_canvas = true;
    h.step();
    h.key_press(Key::Tab);
    h.run();
    rig::key(&mut h, Key::PageDown, Modifiers::NONE);
    rig::key(&mut h, Key::Home, Modifiers::NONE);
    let before = h.state().session.planes[0].corners;
    rig::key(&mut h, Key::ArrowRight, Modifiers::NONE);
    let moved = |h: &Harness<'_, Host>| -> Vec<(usize, [f64; 2])> {
        let now = h.state().session.planes[0].corners;
        (0..4).filter(|i| now[*i] != before[*i]).map(|i| (i, [now[i][0] - before[i][0], now[i][1] - before[i][1]])).collect()
    };
    let one = moved(&h);
    assert!(matches!(one.as_slice(), [(_, [1.0, 0.0])]), "the group has the keyboard: one corner, one pixel: {one:?}");
    // The canvas takes the focus back: the pick stays, the arrows are not aimed at it.
    h.state_mut().focus_canvas = true;
    h.step();
    rig::key(&mut h, Key::ArrowRight, Modifiers::NONE);
    assert_eq!(moved(&h), one, "the canvas has the focus: nothing moves further");
}
