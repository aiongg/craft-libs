//! Tests of the Vanishing Point workspace: the plane editor's tools and parts, the history and the dialog, over a fake
//! scene and a solid layer.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use craft_behavior::edit_history::Step;
use craft_behavior::geometry::quad::Homography;
use craft_behavior::kurbo::Point;
use craft_behavior::{CancelReason, EditEvent, PixelSize};
use craft_imaging::ui::vanishing_point::{
    MAX_PLANES, PlaneCheck, PlaneEdge, StampLimits, TornPlane, VanishingPlane, VanishingPointEvent, VanishingPointOpen, VanishingPointSession,
    VanishingPointSettings, VanishingPointSource, VanishingPointWorkspace, VanishingPreviewRequest, VanishingScene, VanishingTool, slide_edge,
};
use craft_imaging::ui::wide_angle::PreviewDone;
use craft_ui::canvas::{PixelRect, RasterSource};
use craft_ui::contract::Job;
use craft_ui::primitives::Revision;
use craft_ui::runtime::access::Role;
use craft_ui::sections::DialogRole;
use craft_ui::testing::{Fixture, Frames, pointer, trial};
use egui::{Color32, ColorImage, Key, Modifiers, Pos2, Shape, Vec2};

/// The layer's size in document pixels.
const LAYER: [u32; 2] = [800, 600];

/// The engine's clone limits, as the app gives them.
const LIMITS: StampLimits = StampLimits::new(4096, 200_000);

/// Half the width and height of the plane the tests draw on screen, in points.
const HALF: Vec2 = Vec2::new(100.0, 50.0);

/// How far inside the canvas's corner a press lands on empty canvas, in points.
const CORNER_INSET: f32 = 8.0;

/// How close two document points count as the same.
const NEAR: f64 = 1e-6;

/// How far an angle may be off, in degrees.
const DEGREES: f64 = 1.5;

/// A scene whose perspective is flat: torn planes are rectangles on the parent's edge, turned by the angle. It has the
/// contract of the app's scene, not its maths. `verdicts` gives each plane's check; planes past them are valid.
struct Flat {
    verdicts: Vec<PlaneCheck>,
}

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
        VanishingPlane::new([q[first], q[(first + 1) % 4], q[(first + 2) % 4], q[(first + 3) % 4]])
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
            out.push(VanishingPlane::new(corners));
        }
        out
    }

    fn check(&self, _: &[VanishingPlane], index: usize) -> PlaneCheck {
        self.verdicts.get(index).copied().unwrap_or(PlaneCheck::Valid)
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

/// A solid layer.
struct Layer;

impl RasterSource for Layer {
    fn size(&self) -> PixelSize {
        PixelSize::new(LAYER[0], LAYER[1])
    }

    fn revision(&self) -> Revision {
        Revision::new(1)
    }

    fn damage(&self, _: Revision) -> Option<PixelRect> {
        None
    }

    fn read(&self, _: PixelRect, _: u32, out: &mut ColorImage) {
        out.pixels.fill(Color32::from_gray(0x80));
    }
}

impl VanishingPointSource for Layer {
    fn image(&self) -> &dyn RasterSource {
        self
    }

    fn stamp_preview(&self) -> Option<&dyn RasterSource> {
        None
    }

    fn paste(&self) -> Option<&dyn RasterSource> {
        None
    }

    fn preview_job(&self, _: &VanishingPreviewRequest) -> Job<PreviewDone, String> {
        Job::new(|_, _| Ok(PreviewDone::new(Duration::ZERO)))
    }
}

/// What the app keeps of the workspace and what it saw.
struct App {
    workspace: VanishingPointWorkspace,
    scene: Flat,
    events: Vec<VanishingPointEvent>,
    closed: Option<DialogRole>,
    status: Option<String>,
}

/// The workspace over `session` with `tool` picked and `verdicts` for the planes, settled.
fn open_with(tool: VanishingTool, session: VanishingPointSession, verdicts: Vec<PlaneCheck>, limits: StampLimits) -> (Fixture, Rc<RefCell<App>>) {
    let mut workspace = VanishingPointWorkspace::new(VanishingPointOpen::new(LAYER, "Photo", limits).session(session));
    workspace.tool(tool);
    let app = Rc::new(RefCell::new(App { workspace, scene: Flat { verdicts }, events: Vec::new(), closed: None, status: None }));
    let shown = Rc::clone(&app);
    let mut fixture = Fixture::new(move |ui| {
        let mut app = shown.borrow_mut();
        let App { workspace, scene, .. } = &mut *app;
        let out = workspace.show(ui, scene, &Layer);
        app.events.extend(out.events);
        app.closed = app.closed.or(out.closed);
        app.status = out.status.or(app.status.take());
    });
    fixture.room(Vec2::new(1000.0, 900.0));
    fixture.settle();
    (fixture, app)
}

fn open(tool: VanishingTool) -> (Fixture, Rc<RefCell<App>>) {
    open_with(tool, VanishingPointSession::default(), Vec::new(), LIMITS)
}

/// The middle of the canvas, moved by `offset`: the tests lay their planes out in screen points, whatever the zoom.
fn at(fixture: &Fixture, offset: Vec2) -> Pos2 {
    let canvas = fixture.by_role(Role::CANVAS).into_iter().next().expect("the workspace shows a canvas");
    canvas.center() + offset
}

fn click(fixture: &mut Fixture, to: Pos2) {
    fixture.frame(vec![pointer::moved(to)]);
    fixture.frame(vec![pointer::press(to)]);
    fixture.frame(vec![pointer::release(to)]);
    fixture.frame(Vec::new());
}

/// Press at `from`, move to `to` in two steps, release.
fn drag(fixture: &mut Fixture, from: Pos2, to: Pos2) {
    fixture.frame(vec![pointer::moved(from)]);
    fixture.frame(vec![pointer::press(from)]);
    fixture.frame(vec![pointer::moved(from + (to - from) * 0.5)]);
    fixture.frame(vec![pointer::moved(to)]);
    fixture.frame(vec![pointer::release(to)]);
    fixture.frame(Vec::new());
}

/// The plane's corners as screen offsets from the middle of the canvas, clockwise from the top left.
fn corners() -> [Vec2; 4] {
    [Vec2::new(-HALF.x, -HALF.y), Vec2::new(HALF.x, -HALF.y), Vec2::new(HALF.x, HALF.y), Vec2::new(-HALF.x, HALF.y)]
}

/// Click the four corners of the tests' plane.
fn draw_plane(fixture: &mut Fixture) {
    for offset in corners() {
        let to = at(fixture, offset);
        click(fixture, to);
    }
}

fn session(app: &Rc<RefCell<App>>) -> VanishingPointSession {
    app.borrow().workspace.session().clone()
}

fn near(a: [f64; 2], b: [f64; 2]) -> bool {
    (a[0] - b[0]).hypot(a[1] - b[1]) < NEAR
}

/// Which kind of change an event reports.
fn phase(event: &VanishingPointEvent) -> &'static str {
    fn of<T>(kind: &'static str, event: &EditEvent<T>) -> &'static str {
        match (kind, event) {
            ("planes", EditEvent::Begin { .. }) => "planes begin",
            ("planes", EditEvent::Update { .. }) => "planes update",
            ("planes", EditEvent::Commit { .. }) => "planes commit",
            ("planes", EditEvent::Cancel { .. }) => "planes cancel",
            ("planes", EditEvent::Set { .. }) => "planes set",
            ("stamp", EditEvent::Begin { .. }) => "stamp begin",
            ("stamp", EditEvent::Commit { .. }) => "stamp commit",
            _ => "other",
        }
    }
    match event {
        VanishingPointEvent::Planes(e) => of("planes", e),
        VanishingPointEvent::Paste(e) => of("paste", e),
        VanishingPointEvent::Stamp(e) => of("stamp", e),
        _ => "other",
    }
}

fn phases(app: &Rc<RefCell<App>>) -> Vec<&'static str> {
    app.borrow().events.iter().map(phase).collect()
}

fn drawn() -> VanishingPointSession {
    let mut session = VanishingPointSession::default();
    session.planes = vec![VanishingPlane::new([[100.0, 100.0], [300.0, 100.0], [300.0, 200.0], [100.0, 200.0]])];
    session.torn = vec![None];
    session
}

#[trial]
fn four_clicks_make_a_plane_in_any_order() {
    let reversed = [3, 2, 1, 0];
    for order in [[0, 1, 2, 3], reversed] {
        let (mut fixture, app) = open(VanishingTool::CreatePlane);
        for (n, corner) in order.into_iter().enumerate() {
            let to = at(&fixture, corners()[corner]);
            click(&mut fixture, to);
            let pending = app.borrow().workspace.editor().pending().len();
            assert_eq!(pending, if n == 3 { 0 } else { n + 1 });
        }
        let kept = session(&app);
        assert_eq!(kept.planes.len(), 1);
        let c = kept.planes[0].corners;
        assert!(c[0][0] < c[1][0] && c[0][1] < c[3][1], "top-left first, clockwise: {c:?}");
        let editor_state = {
            let app = app.borrow();
            (app.workspace.editor().selected(), app.workspace.editor().tool())
        };
        assert_eq!(editor_state, (Some(0), VanishingTool::EditPlane));
        assert_eq!(phases(&app), ["planes set"], "only the fourth click reports");
    }
}

#[trial]
fn an_edge_node_slides_along_the_planes_perspective() {
    let (mut fixture, app) = open(VanishingTool::CreatePlane);
    draw_plane(&mut fixture);
    let before = session(&app).planes[0];
    let node = at(&fixture, Vec2::new(0.0, -HALF.y));
    drag(&mut fixture, node, node - Vec2::new(0.0, HALF.y * 0.8));
    let c = session(&app).planes[0].corners;
    assert!(near(c[2], before.corners[2]) && near(c[3], before.corners[3]), "the bottom edge stays: {c:?}");
    let off = |p: [f64; 2], a: [f64; 2], b: [f64; 2]| ((b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])).abs();
    assert!(off(c[0], before.corners[0], before.corners[3]) < 1e-6 && off(c[1], before.corners[1], before.corners[2]) < 1e-6, "{c:?}");
    assert!(c[0][1] < before.corners[0][1], "the edge moved up: {c:?}");
    let seen = phases(&app);
    assert_eq!((seen.get(1).copied(), seen.last().copied()), (Some("planes begin"), Some("planes commit")), "{seen:?}");
    let at_press = before.homography().expect("a valid plane");
    let node = before.edge_nodes()[0];
    let same = slide_edge(&Flat { verdicts: Vec::new() }, &at_press, 0, node).expect("the node's own place");
    assert!(same.corners.iter().zip(before.corners).all(|(a, b)| near(*a, b)), "{same:?}");
}

#[trial]
fn a_command_drag_tears_off_a_plane_and_the_far_edge_sets_depth_or_angle() {
    let (mut fixture, app) = open(VanishingTool::CreatePlane);
    draw_plane(&mut fixture);
    let bottom = at(&fixture, Vec2::new(0.0, HALF.y));
    fixture.hold(Modifiers::COMMAND);
    drag(&mut fixture, bottom, bottom + Vec2::new(0.0, HALF.y * 2.0));
    fixture.hold(Modifiers::NONE);
    let kept = session(&app);
    let Some(t) = kept.torn.get(1).copied().flatten() else { panic!("plane 1 was torn off: {:?}", kept.torn) };
    assert_eq!((t.from, t.edge, t.angle), (0, PlaneEdge::Bottom, 90.0));
    assert!((t.depth - 1.0).abs() < 0.05, "depth {}", t.depth);
    let tool = app.borrow().workspace.editor().tool();
    assert_eq!((app.borrow().workspace.editor().selected(), tool), (Some(1), VanishingTool::EditPlane));
    let seen = phases(&app);
    assert_eq!((seen.get(1).copied(), seen.last().copied()), (Some("planes begin"), Some("planes commit")), "{seen:?}");

    // Alt on the far edge node turns the plane about the edge instead.
    let far = bottom + Vec2::new(0.0, HALF.y * 2.0);
    let turned = bottom + Vec2::new(-HALF.y * 2.0 * 0.7071, HALF.y * 2.0 * 0.7071);
    fixture.hold(Modifiers::ALT);
    drag(&mut fixture, far, turned);
    fixture.hold(Modifiers::NONE);
    let Some(t) = session(&app).torn[1] else { panic!("still torn") };
    assert!((t.angle - 135.0).abs() <= DEGREES, "angle {}", t.angle);
}

#[trial]
fn delete_removes_the_plane_and_escape_drops_a_half_plane_first() {
    let drawn_planes = [
        VanishingPlane::new([[100.0, 50.0], [300.0, 50.0], [300.0, 100.0], [100.0, 100.0]]),
        VanishingPlane::new([[0.0; 2]; 4]),
        VanishingPlane::new([[0.0; 2]; 4]),
    ];
    let torn = vec![None, Some(TornPlane::new(0, PlaneEdge::Bottom, 90.0, 1.0)), Some(TornPlane::new(1, PlaneEdge::Bottom, 90.0, 1.0))];
    let planes = Flat { verdicts: Vec::new() }.resolve(&drawn_planes, &torn);
    let mut kept = VanishingPointSession::default();
    kept.planes.clone_from(&planes);
    kept.torn.clone_from(&torn);
    let (mut fixture, app) = open_with(VanishingTool::EditPlane, kept, Vec::new(), LIMITS);
    app.borrow_mut().workspace.selected_plane(Some(0));
    // An empty spot near the canvas's lower corner, where no plane lies, gives the canvas the keyboard.
    let corner = fixture.by_role(Role::CANVAS).into_iter().next().expect("the workspace shows a canvas").rect.right_bottom();
    click(&mut fixture, corner - Vec2::splat(CORNER_INSET));
    app.borrow_mut().workspace.selected_plane(Some(0));
    fixture.key(Key::Delete, Modifiers::NONE).settle();
    let after = session(&app);
    assert_eq!(after.planes, planes[1..], "the children keep their place");
    assert_eq!(after.torn, [None, torn[2].map(|child| TornPlane { from: 0, ..child })], "the child is drawn, its child follows it");
    assert_eq!(app.borrow().workspace.editor().selected(), None);

    // Escape: the half plane first, then the dialog's Cancel.
    let (mut fixture, app) = open(VanishingTool::CreatePlane);
    for offset in &corners()[..2] {
        let to = at(&fixture, *offset);
        click(&mut fixture, to);
    }
    assert_eq!(app.borrow().workspace.editor().pending().len(), 2);
    fixture.key(Key::Escape, Modifiers::NONE).settle();
    assert!(app.borrow().workspace.editor().pending().is_empty() && app.borrow().closed.is_none(), "Escape drops the corners and keeps the dialog");
    fixture.key(Key::Escape, Modifiers::NONE).settle();
    assert_eq!(app.borrow().closed, Some(DialogRole::Cancel), "with nothing pending Escape cancels");
}

#[trial]
fn stamp_alt_click_sets_the_source_and_strokes_report_events() {
    let (mut fixture, app) = open(VanishingTool::CreatePlane);
    draw_plane(&mut fixture);
    app.borrow_mut().workspace.tool(VanishingTool::Stamp);
    fixture.settle();
    let off = at(&fixture, Vec2::new(HALF.x * 8.0, HALF.y * 8.0));
    fixture.hold(Modifiers::ALT);
    click(&mut fixture, off);
    fixture.hold(Modifiers::NONE);
    assert_eq!(session(&app).source, None, "off every plane");
    assert!(app.borrow().status.is_some(), "the press says why");
    let (mut fixture, app) = open(VanishingTool::CreatePlane);
    draw_plane(&mut fixture);
    app.borrow_mut().workspace.tool(VanishingTool::Stamp);
    fixture.settle();
    let (source, start) = (at(&fixture, Vec2::new(-HALF.x * 0.5, 0.0)), at(&fixture, Vec2::new(0.0, 0.0)));
    fixture.hold(Modifiers::ALT);
    click(&mut fixture, source);
    fixture.hold(Modifiers::NONE);
    let kept = session(&app);
    assert!(kept.source.is_some(), "the source is set on the plane");
    drag(&mut fixture, start, start + Vec2::new(HALF.x * 0.4, 0.0));
    let seen: Vec<_> = phases(&app).into_iter().filter(|p| p.starts_with("stamp")).collect();
    assert_eq!((seen.first().copied(), seen.last().copied()), (Some("stamp begin"), Some("stamp commit")), "{seen:?}");
    let kept = session(&app);
    let stroke = &kept.strokes[0];
    assert_eq!((stroke.source, stroke.hardness, stroke.opacity), (kept.source.expect("a source"), 0.5, 1.0));
    assert!(stroke.points.len() > 1, "the drag laid dabs");
    // Aligned: the next stroke keeps the offset from its start; without Aligned it restarts at the source.
    drag(&mut fixture, start + Vec2::new(0.0, 10.0), start + Vec2::new(10.0, 10.0));
    let second = session(&app).strokes[1].clone();
    assert_ne!(second.source, kept.source.expect("a source"), "an aligned stroke follows its start");
}

#[trial]
fn undo_steps_back_through_planes_and_strokes() {
    let (mut fixture, app) = open(VanishingTool::CreatePlane);
    draw_plane(&mut fixture);
    let bottom = at(&fixture, Vec2::new(0.0, HALF.y));
    fixture.hold(Modifiers::COMMAND);
    drag(&mut fixture, bottom, bottom + Vec2::new(0.0, HALF.y * 2.0));
    fixture.hold(Modifiers::NONE);
    app.borrow_mut().workspace.tool(VanishingTool::Stamp);
    fixture.settle();
    let (source, start) = (at(&fixture, Vec2::new(-HALF.x * 0.5, 0.0)), at(&fixture, Vec2::ZERO));
    fixture.hold(Modifiers::ALT);
    click(&mut fixture, source);
    fixture.hold(Modifiers::NONE);
    drag(&mut fixture, start, start + Vec2::new(HALF.x * 0.4, 0.0));
    let counts = |app: &Rc<RefCell<App>>| {
        let s = session(app);
        (s.planes.len(), s.source.is_some(), s.strokes.len())
    };
    assert_eq!(counts(&app), (2, true, 1));
    let step = |app: &Rc<RefCell<App>>, step: Step| assert!(app.borrow_mut().workspace.step(step));
    step(&app, Step::Undo);
    assert_eq!(counts(&app), (2, true, 0), "the stroke");
    step(&app, Step::Undo);
    assert_eq!(counts(&app), (2, false, 0), "the source");
    step(&app, Step::Undo);
    assert_eq!(counts(&app), (1, false, 0), "the tear-off");
    step(&app, Step::Undo);
    assert_eq!(counts(&app), (0, false, 0), "the plane");
    assert_eq!(app.borrow().workspace.editor().selected(), None);
    step(&app, Step::Redo);
    assert_eq!(counts(&app), (1, false, 0));
}

#[trial]
fn planes_are_capped_at_sixty_four() {
    let mut full = VanishingPointSession::default();
    full.planes = (0..MAX_PLANES)
        .map(|i| {
            VanishingPlane::new([
                [1000.0 * i as f64, 0.0],
                [1000.0 * i as f64 + 50.0, 0.0],
                [1000.0 * i as f64 + 50.0, 50.0],
                [1000.0 * i as f64, 50.0],
            ])
        })
        .collect();
    full.torn = vec![None; MAX_PLANES];
    let (mut fixture, app) = open_with(VanishingTool::CreatePlane, full, Vec::new(), LIMITS);
    draw_plane(&mut fixture);
    assert_eq!((session(&app).planes.len(), app.borrow().workspace.editor().pending().len()), (MAX_PLANES, 0));
    assert!(app.borrow().events.is_empty(), "{:?}", phases(&app));
}

#[trial]
fn slide_edge_takes_the_scenes_verdict() {
    let plane = VanishingPlane::new([[100.0, 100.0], [300.0, 100.0], [300.0, 200.0], [100.0, 200.0]]);
    let at_press = plane.homography().expect("a valid plane");
    let node = plane.edge_nodes()[0];
    let up = [node[0], node[1] - 30.0];
    let with = |check| Flat { verdicts: vec![check] };
    assert!(slide_edge(&with(PlaneCheck::Valid), &at_press, 0, up).is_some());
    assert!(slide_edge(&with(PlaneCheck::Questionable), &at_press, 0, up).is_some());
    assert!(slide_edge(&with(PlaneCheck::Invalid), &at_press, 0, up).is_none());
}

#[trial]
fn stamp_stops_at_the_apps_limits_and_says_why() {
    let (mut fixture, app) = open_with(VanishingTool::CreatePlane, VanishingPointSession::default(), Vec::new(), StampLimits::new(1, 200_000));
    draw_plane(&mut fixture);
    app.borrow_mut().workspace.tool(VanishingTool::Stamp);
    fixture.settle();
    let (source, start) = (at(&fixture, Vec2::new(-HALF.x * 0.5, 0.0)), at(&fixture, Vec2::ZERO));
    fixture.hold(Modifiers::ALT);
    click(&mut fixture, source);
    fixture.hold(Modifiers::NONE);
    drag(&mut fixture, start, start + Vec2::new(HALF.x * 0.4, 0.0));
    assert_eq!(session(&app).strokes.len(), 1);
    assert_eq!(app.borrow().status, None);
    drag(&mut fixture, start + Vec2::new(0.0, 10.0), start + Vec2::new(HALF.x * 0.4, 10.0));
    assert_eq!(session(&app).strokes.len(), 1, "the second stroke is refused");
    assert!(app.borrow().status.is_some(), "and says why");
}

/// A layer with a clipboard image.
struct Pasted {
    layer: Layer,
    image: Image,
}

/// The clipboard image: `size` pixels of one colour.
struct Image(PixelSize);

impl RasterSource for Image {
    fn size(&self) -> PixelSize {
        self.0
    }

    fn revision(&self) -> Revision {
        Revision::new(1)
    }

    fn damage(&self, _: Revision) -> Option<PixelRect> {
        None
    }

    fn read(&self, _: PixelRect, _: u32, out: &mut ColorImage) {
        out.pixels.fill(Color32::from_gray(u8::MAX));
    }
}

impl VanishingPointSource for Pasted {
    fn image(&self) -> &dyn RasterSource {
        &self.layer
    }

    fn stamp_preview(&self) -> Option<&dyn RasterSource> {
        None
    }

    fn paste(&self) -> Option<&dyn RasterSource> {
        Some(&self.image)
    }

    fn preview_job(&self, request: &VanishingPreviewRequest) -> Job<PreviewDone, String> {
        self.layer.preview_job(request)
    }
}

#[trial]
fn the_paste_extent_keeps_the_images_metric_aspect() {
    let scene = Flat { verdicts: Vec::new() };
    let mut settings = VanishingPointSettings::default();
    settings.paste = true;
    let placed = |size: PixelSize| {
        let source = Pasted { layer: Layer, image: Image(size) };
        let mut session = drawn();
        let mut editor = craft_imaging::ui::vanishing_point::VanishingPointEditor::new(LIMITS);
        let pass = editor.pass(&mut session, &settings, &scene, &source);
        pass.paste_placement().map(|(_, rect)| (rect.width(), rect.height()))
    };
    // The plane is 200 by 100 pixels: half its width is 100 pixels, and a 40 by 30 image that wide is 75 pixels
    // high, three quarters of the plane's height.
    assert_eq!(placed(PixelSize::new(40, 30)), Some((0.5, 0.75)));
    let empty = placed(PixelSize::new(0, 30)).expect("an empty image has a placement");
    assert!(empty.0 == 0.5 && empty.1.is_finite(), "an empty image does not divide by zero: {empty:?}");
}

#[trial]
fn selecting_clamps_to_the_sessions_planes() {
    let (_, app) = open_with(VanishingTool::EditPlane, drawn(), Vec::new(), LIMITS);
    assert_eq!(app.borrow_mut().workspace.selected_plane(Some(0)).editor().selected(), Some(0));
    assert_eq!(app.borrow_mut().workspace.selected_plane(Some(1)).editor().selected(), None);
}

#[trial]
fn a_gesture_the_app_cancels_reports_its_reason_and_ends() {
    for reason in [CancelReason::Removed, CancelReason::UndoDuringGesture] {
        let (mut fixture, app) = open(VanishingTool::CreatePlane);
        draw_plane(&mut fixture);
        let before = session(&app);
        let corner = at(&fixture, corners()[0]);
        fixture.frame(vec![pointer::moved(corner)]);
        fixture.frame(vec![pointer::press(corner)]);
        fixture.frame(vec![pointer::moved(corner + Vec2::new(30.0, 20.0))]);
        fixture.frame(vec![pointer::moved(corner + Vec2::new(40.0, 30.0))]);
        assert!(app.borrow().workspace.editor().busy(), "the corner drag runs");
        assert_ne!(session(&app), before);
        match reason {
            CancelReason::Removed => {
                fixture.key(Key::C, Modifiers::NONE).settle();
            }
            _ => {
                app.borrow_mut().workspace.step(Step::Undo);
                fixture.settle();
            }
        }
        assert!(!app.borrow().workspace.editor().busy());
        let cancelled =
            app.borrow().events.iter().any(|e| matches!(e, VanishingPointEvent::Planes(EditEvent::Cancel { reason: r, .. }) if *r == reason));
        assert!(cancelled, "{:?}", phases(&app));
    }
}

#[trial]
fn arrows_move_the_focused_corner_only() {
    let (mut fixture, app) = open(VanishingTool::CreatePlane);
    draw_plane(&mut fixture);
    let before = session(&app).planes[0].corners;
    fixture.key(Key::Tab, Modifiers::NONE).settle();
    fixture.key(Key::ArrowRight, Modifiers::NONE).settle();
    let now = session(&app).planes[0].corners;
    let moved: Vec<usize> = (0..4).filter(|i| now[*i] != before[*i]).collect();
    assert!(moved.len() <= 1, "at most the focused corner moves: {moved:?}");
}

#[trial]
fn plane_colours_follow_the_check() {
    let colours = |verdicts: Vec<PlaneCheck>| {
        let mut session = VanishingPointSession::default();
        for k in 0..3 {
            let x = 100.0 * f64::from(k);
            session.planes.push(VanishingPlane::new([[x, 0.0], [x + 50.0, 0.0], [x + 50.0, 50.0], [x, 50.0]]));
            session.torn.push(None);
        }
        let mut workspace = VanishingPointWorkspace::new(VanishingPointOpen::new(LAYER, "Photo", LIMITS).session(session));
        workspace.tool(VanishingTool::EditPlane);
        let scene = Flat { verdicts };
        let mut frames = Frames::new();
        let mut shapes = Vec::new();
        for _ in 0..3 {
            shapes = frames.frame_shapes(|ui| {
                workspace.show(ui, &scene, &Layer);
            });
        }
        let mut found: Vec<[u8; 4]> = shapes
            .into_iter()
            .filter_map(|shape| match shape {
                Shape::Path(path) => Some(path.stroke.color),
                _ => None,
            })
            .map(|color| match color {
                egui::epaint::ColorMode::Solid(color) => color.to_array(),
                _ => [0; 4],
            })
            .collect();
        found.sort();
        found.dedup();
        found.len()
    };
    let mixed = colours(vec![PlaneCheck::Valid, PlaneCheck::Questionable, PlaneCheck::Invalid]);
    assert!(mixed > colours(vec![PlaneCheck::Valid; 3]), "the three verdicts use more colours than one");
}

#[trial]
fn it_opens_in_its_own_window_with_ok_and_cancel() {
    let (mut fixture, app) = open(VanishingTool::CreatePlane);
    let ok = fixture.by_label("OK").expect("the footer has OK");
    fixture.click(&ok).settle();
    assert_eq!(app.borrow().closed, Some(DialogRole::Accept));
    let (mut fixture, app) = open(VanishingTool::CreatePlane);
    let cancel = fixture.by_label("Cancel").expect("the footer has Cancel");
    fixture.click(&cancel).settle();
    assert_eq!(app.borrow().closed, Some(DialogRole::Cancel));
}

#[trial]
fn four_typed_corners_make_the_plane_the_scene_checks() {
    let mut session = VanishingPointSession::default();
    let scene = Flat { verdicts: Vec::new() };
    let mut editor = craft_imaging::ui::vanishing_point::VanishingPointEditor::new(LIMITS);
    let skewed = [[300.0, 200.0], [300.0, 100.0], [100.0, 100.0], [100.0, 200.0]];
    assert!(editor.add_plane(&mut session, &scene, skewed, "New Plane"));
    assert_eq!(session.planes[0].corners, [[100.0, 100.0], [300.0, 100.0], [300.0, 200.0], [100.0, 200.0]], "the scene normalises the typed corners");
    assert_eq!(editor.selected(), Some(0));
    assert!(matches!(editor.take_events().as_slice(), [VanishingPointEvent::Planes(EditEvent::Set { .. })]));
}
