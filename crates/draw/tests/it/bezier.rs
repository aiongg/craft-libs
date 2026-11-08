//! `BezierOverlay`: the pure path operations, and the overlay's pointer, keyboard and tools through the public API.

use craft_behavior::kurbo::{ParamCurve as _, Point, Vec2};
use craft_behavior::{CanvasView, EditEvent};
use craft_draw::ui::{
    AnchorsShown, BezierHit, BezierOverlay, BezierTool, EditPath, EditSubpath, HandleSide, HandlesShown, Knot, KnotRef, MAX_ANCHORS, PathEdit,
    PathSelection, SegmentMark,
};
use craft_ui::canvas::{Canvas, Guide, GuideKey};
use craft_ui::canvas_controls::Guides;
use craft_ui::contract::{Bind, Role, Show as _, ShowContainer as _};
use craft_ui::testing::{Fixture, pointer, trial};
use egui::{Id, Key, Modifiers, Pos2, Vec2 as ScreenVec, vec2};
use std::cell::RefCell;
use std::rc::Rc;

fn p(x: f64, y: f64) -> Point {
    Point::new(x, y)
}

fn kr(knot: usize) -> KnotRef {
    KnotRef::new(0, knot)
}

fn near(a: Point, b: Point) -> bool {
    a.distance(b) < 1e-9
}

fn near_vec(a: Vec2, b: Vec2) -> bool {
    (a - b).hypot() < 1e-3
}

// ---- the pure operations ----

fn ops_path(sub: EditSubpath) -> EditPath {
    EditPath::new(7, vec![sub])
}

fn ops_square(smooth: bool) -> EditSubpath {
    let corners = [p(0.0, 0.0), p(100.0, 0.0), p(100.0, 100.0), p(0.0, 100.0)];
    EditSubpath::new(corners.iter().map(|&c| Knot::new(c, c, c, smooth)).collect(), true)
}

// A curve split at t = 0.3 draws the same curve (sampled); a straight segment gets a corner on the cubic's point at
// t; t of 0, 1 or NaN is refused and changes nothing.
#[trial]
fn edit_subpath_insert_at_splits_a_curve_without_changing_it() {
    let mut sp = EditSubpath::new(vec![Knot::smooth(p(0.0, 0.0), p(30.0, 60.0)), Knot::smooth(p(100.0, 0.0), p(130.0, -20.0))], false);
    let before = sp.segment(0).unwrap();
    assert_eq!(sp.insert_at(0, 0.3), Some(1));
    assert_eq!(sp.knots.len(), 3);
    assert!(near(sp.knots[1].anchor, before.eval(0.3)));
    let (a, b) = (sp.segment(0).unwrap(), sp.segment(1).unwrap());
    for i in 0..=10 {
        let s = f64::from(i) / 10.0;
        assert!(near(a.eval(s), before.eval(0.3 * s)));
        assert!(near(b.eval(s), before.eval(0.3 + 0.7 * s)));
    }

    let mut line = EditSubpath::new(vec![Knot::corner(p(0.0, 0.0)), Knot::corner(p(100.0, 0.0))], false);
    let at = line.segment(0).unwrap().eval(0.3);
    assert_eq!(line.insert_at(0, 0.3), Some(1));
    assert!(near(line.knots[1].anchor, at));
    assert!(!line.knots[1].smooth);

    let snapshot = sp.clone();
    for t in [0.0, 1.0, f64::NAN] {
        assert_eq!(sp.insert_at(0, t), None);
    }
    assert_eq!(sp.insert_at(5, 0.5), None);
    assert_eq!(sp, snapshot);
}

// Removing an inner knot joins its neighbours with the previous out and next in handles; a closed subpath stays
// closed; removing the only knot leaves an empty subpath.
#[trial]
fn edit_subpath_remove_knot_joins_neighbours_and_keeps_closed() {
    let mut sp = EditSubpath::new(
        vec![Knot::smooth(p(0.0, 0.0), p(10.0, 10.0)), Knot::smooth(p(50.0, 0.0), p(60.0, 0.0)), Knot::smooth(p(100.0, 0.0), p(110.0, 5.0))],
        true,
    );
    let (out, inn) = (sp.knots[0].out_ctrl, sp.knots[2].in_ctrl);
    assert!(sp.remove_knot(1));
    assert!(sp.closed);
    let seg = sp.segment(0).unwrap();
    assert_eq!((seg.p1, seg.p2), (out, inn));
    assert!(!sp.remove_knot(9));
    let mut one = EditSubpath::new(vec![Knot::corner(p(1.0, 1.0))], false);
    assert!(one.remove_knot(0));
    assert!(one.knots.is_empty());
}

// PhotoCraft's closed four-anchor loop: moving one anchor reflows every smooth knot (the neighbours turn), corner
// knots keep their handles.
#[trial]
fn path_edit_move_curvature_reflows_every_smooth_knot() {
    let mut sp = ops_square(true);
    sp.reflow();
    sp.knots[2] = Knot::corner(p(100.0, 100.0));
    let mut ep = ops_path(sp);
    let corner = ep.subpaths[0].knots[2];
    let old_neighbour = ep.subpaths[0].knots[1];
    let edit = PathEdit::move_curvature(7, vec![kr(0)], Vec2::new(-20.0, -10.0));
    assert!(edit.apply(&mut ep));
    let k = &ep.subpaths[0].knots;
    assert_eq!(k[0].anchor, p(-20.0, -10.0));
    assert_ne!(k[1].out_ctrl, old_neighbour.out_ctrl);
    assert_eq!(k[2], corner);
    // Smooth knot handles are collinear with the chord of its neighbours.
    let chord = k[2].anchor - k[0].anchor;
    let o = k[1].out_ctrl - k[1].anchor;
    assert!((o.x * chord.y - o.y * chord.x).abs() < 1e-9);
}

// A corner square: toggling a knot makes it smooth and the segments around it curve; toggling again retracts its
// handles.
#[trial]
fn path_edit_toggle_curvature_flips_smooth_and_corner() {
    let mut ep = ops_path(ops_square(false));
    let toggle = PathEdit::toggle_curvature(7, kr(1));
    assert!(toggle.apply(&mut ep));
    assert!(ep.subpaths[0].knots[1].smooth);
    assert!(!ep.subpaths[0].is_straight(0));
    assert!(!ep.subpaths[0].is_straight(1));
    assert!(toggle.apply(&mut ep));
    let k = ep.subpaths[0].knots[1];
    assert!(!k.smooth && k.in_ctrl == k.anchor && k.out_ctrl == k.anchor);
}

// `InsertCurvature` on a straight segment adds a smooth knot moved by `by` and reflows.
#[trial]
fn path_edit_insert_curvature_adds_a_smooth_anchor_and_moves_it() {
    let mut ep = ops_path(ops_square(false));
    let at = ep.subpaths[0].segment(0).unwrap().eval(0.5);
    let edit = PathEdit::insert_curvature(7, kr(0), 0.5, Vec2::new(0.0, -30.0));
    assert!(edit.apply(&mut ep));
    let sp = &ep.subpaths[0];
    assert_eq!(sp.knots.len(), 5);
    assert!(sp.knots[1].smooth);
    assert!(near(sp.knots[1].anchor, at + Vec2::new(0.0, -30.0)));
    assert_ne!(sp.knots[1].out_ctrl, sp.knots[1].anchor);
}

// MoveAnchors, MoveHandle (smooth and alone), BendSegment (straight and curved), Convert give PhotoCraft's kernel
// results; a wrong key or index changes nothing and returns false.
#[trial]
fn path_edit_apply_matches_the_kernel_for_every_existing_edit() {
    let base = || {
        ops_path(EditSubpath::new(
            vec![Knot::smooth(p(0.0, 0.0), p(20.0, 0.0)), Knot::smooth(p(100.0, 0.0), p(120.0, 0.0)), Knot::corner(p(100.0, 80.0))],
            false,
        ))
    };
    let mut ep = base();
    let by = Vec2::new(5.0, 7.0);
    assert!(PathEdit::move_anchors(7, vec![kr(1), kr(1)], by).apply(&mut ep));
    assert_eq!(ep.subpaths[0].knots[1].anchor, p(105.0, 7.0));
    assert_eq!(ep.subpaths[0].knots[1].out_ctrl, p(125.0, 7.0));

    let mut ep = base();
    let to = p(0.0, 30.0);
    let h = PathEdit::move_handle(7, kr(0), HandleSide::Out, to, false);
    assert!(h.apply(&mut ep));
    let k = ep.subpaths[0].knots[0];
    assert_eq!(k.out_ctrl, to);
    assert!(near(k.in_ctrl, p(0.0, -20.0)));
    let mut ep = base();
    let h = PathEdit::move_handle(7, kr(0), HandleSide::Out, to, true);
    assert!(h.apply(&mut ep));
    assert!(!ep.subpaths[0].knots[0].smooth);
    assert_eq!(ep.subpaths[0].knots[0].in_ctrl, p(-20.0, 0.0));

    let mut ep = base();
    let before = ep.subpaths[0].segment(0).unwrap().eval(0.5);
    let bend = PathEdit::bend_segment(7, kr(0), 0.5, Vec2::new(0.0, 10.0));
    assert!(bend.apply(&mut ep));
    assert!(near(ep.subpaths[0].segment(0).unwrap().eval(0.5), before + Vec2::new(0.0, 10.0)));
    let mut ep = base();
    let bend = PathEdit::bend_segment(7, kr(1), 0.5, Vec2::new(0.0, 10.0));
    ep.subpaths[0].knots[1].out_ctrl = ep.subpaths[0].knots[1].anchor;
    ep.subpaths[0].knots[1].in_ctrl = ep.subpaths[0].knots[1].anchor;
    assert!(bend.apply(&mut ep));
    assert_eq!(ep.subpaths[0].knots[1].anchor, p(100.0, 10.0));
    assert_eq!(ep.subpaths[0].knots[2].anchor, p(100.0, 90.0));

    let mut ep = base();
    assert!(PathEdit::convert(7, kr(0), None).apply(&mut ep));
    assert_eq!(ep.subpaths[0].knots[0], Knot::corner(p(0.0, 0.0)));
    assert!(PathEdit::convert(7, kr(2), Some(p(110.0, 80.0))).apply(&mut ep));
    assert_eq!(ep.subpaths[0].knots[2], Knot::smooth(p(100.0, 80.0), p(110.0, 80.0)));

    let mut ep = base();
    let untouched = base();
    let edits = [
        PathEdit::move_anchors(8, vec![kr(0)], by),
        PathEdit::move_anchors(7, vec![kr(0), kr(9)], by),
        PathEdit::move_handle(7, KnotRef::new(3, 0), HandleSide::In, to, false),
        PathEdit::bend_segment(7, kr(2), 0.5, by),
        PathEdit::convert(7, kr(5), None),
        PathEdit::toggle_curvature(7, kr(5)),
        PathEdit::insert_curvature(7, kr(0), 0.0, by),
    ];
    for e in edits {
        assert!(!e.apply(&mut ep));
        assert_eq!(ep, untouched);
    }
}

// ---- the overlay on a canvas ----

/// The draft the anchor tests pass as one more path.
const DRAFT: u64 = 9;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Seen {
    Begin,
    Update,
    Commit,
    Cancel,
    Set,
}

struct State {
    paths: Vec<EditPath>,
    selection: PathSelection,
    guides: Vec<Guide>,
    tool: BezierTool,
    handles: HandlesShown,
    anchors: AnchorsShown,
    segment_mark: SegmentMark,
    rotation: f64,
    flip: bool,
    view: Option<CanvasView>,
    id: Id,
    seen: Vec<Seen>,
    changes: Vec<PathEdit>,
    inserts: Vec<(u64, KnotRef, f64)>,
    deletes: Vec<(u64, Vec<KnotRef>)>,
    hover: Option<BezierHit>,
    /// Passes that reported a selection and no edit event.
    lone_selects: usize,
    /// Presses the overlay left to the canvas's tool.
    tool_clicks: usize,
}

struct Harness {
    fixture: Fixture,
    state: Rc<RefCell<State>>,
}

/// Path 1: an open subpath of four anchors: a straight segment, then two curved ones.
fn path_a() -> EditPath {
    let knots = vec![
        Knot::corner(p(60.0, 60.0)),
        Knot::corner(p(160.0, 60.0)),
        Knot::new(p(260.0, 60.0), p(230.0, 20.0), p(290.0, 100.0), true),
        Knot::corner(p(340.0, 160.0)),
    ];
    EditPath::new(1, vec![EditSubpath::new(knots, false)])
}

/// Path 2, drawn after path 1: a closed square and an open line.
fn path_b() -> EditPath {
    let square = [(60.0, 200.0), (140.0, 200.0), (140.0, 260.0), (60.0, 260.0)].map(|(x, y)| Knot::corner(p(x, y)));
    let line = [(200.0, 220.0), (260.0, 260.0)].map(|(x, y)| Knot::corner(p(x, y)));
    let subpaths = vec![EditSubpath::new(square.to_vec(), true), EditSubpath::new(line.to_vec(), false)];
    EditPath::new(2, subpaths).colored(Some(craft_theme::Srgba::rgb(0.9, 0.43, 0.08)))
}

fn open(key: u64, y: f64) -> EditPath {
    let knots = [60.0, 160.0, 260.0].map(|x| Knot::corner(p(x, y))).to_vec();
    EditPath::new(key, vec![EditSubpath::new(knots, false)])
}

fn closed_square(key: u64) -> EditPath {
    let knots = [(60.0, 200.0), (140.0, 200.0), (140.0, 260.0), (60.0, 260.0)].map(|(x, y)| Knot::corner(p(x, y))).to_vec();
    EditPath::new(key, vec![EditSubpath::new(knots, true)])
}

/// An open path (key 1) at y 60, a closed square (key 2) and a draft (key 9) at y 150, in that order.
fn tool_paths() -> Vec<EditPath> {
    vec![open(1, 60.0), closed_square(2), open(DRAFT, 150.0)]
}

fn kind(event: &EditEvent<Vec<EditPath>>) -> Seen {
    match event {
        EditEvent::Begin { .. } => Seen::Begin,
        EditEvent::Update { .. } => Seen::Update,
        EditEvent::Commit { .. } => Seen::Commit,
        EditEvent::Cancel { .. } => Seen::Cancel,
        _ => Seen::Set,
    }
}

fn harness(paths: Vec<EditPath>) -> Harness {
    let state = Rc::new(RefCell::new(State {
        paths,
        selection: PathSelection::default(),
        guides: Vec::new(),
        tool: BezierTool::Direct,
        handles: HandlesShown::Selected,
        anchors: AnchorsShown::All,
        segment_mark: SegmentMark::HandlesOnly,
        rotation: 0.0,
        flip: false,
        view: None,
        id: Id::NULL,
        seen: Vec::new(),
        changes: Vec::new(),
        inserts: Vec::new(),
        deletes: Vec::new(),
        hover: None,
        lone_selects: 0,
        tool_clicks: 0,
    }));
    let shown = Rc::clone(&state);
    let mut fixture = Fixture::new(move |ui| {
        let mut held = shown.borrow_mut();
        let st = &mut *held;
        let room = ui.available_rect_before_wrap();
        let view = CanvasView::new(room, p(f64::from(room.width()) / 2.0, f64::from(room.height()) / 2.0), 1.0).rotation(st.rotation).flip_x(st.flip);
        st.view = Some(view);
        let selection = st.selection.clone();
        let canvas = Canvas::new("Document").view(view).show(ui, |ui, body| {
            // The guides lie under the paths: a stack position hides the levels of the ones below it.
            if !st.guides.is_empty() {
                Guides::new("Guides", Bind::bound(&mut st.guides)).interactive(true).show_at(ui, body.rect);
            }
            BezierOverlay::new("Paths", Bind::bound(&mut st.paths), &selection)
                .tool(st.tool)
                .handles(st.handles)
                .anchors(st.anchors)
                .segment_mark(st.segment_mark)
                .show_at(ui, body.rect)
        });
        if canvas.output.click_at().is_some() {
            st.tool_clicks += 1;
        }
        let Some(output) = canvas.inner else { return };
        st.id = output.edit.response.id;
        let event = output.edit.event.as_ref().map(kind);
        st.seen.extend(event);
        st.hover = output.hover;
        st.changes.extend(output.change);
        if let Some(select) = output.select {
            st.lone_selects += usize::from(event.is_none());
            st.selection = select;
        }
        if let Some((path, segment, t)) = output.insert {
            st.inserts.push((path, segment, t));
            if let Some(sub) = st.paths.iter_mut().find(|e| e.key == path).and_then(|e| e.subpaths.get_mut(segment.subpath)) {
                sub.insert_at(segment.knot, t);
            }
        }
        if let Some((path, mut knots)) = output.delete {
            st.deletes.push((path, knots.clone()));
            knots.sort_by(|a, b| b.cmp(a));
            if let Some(entry) = st.paths.iter_mut().find(|e| e.key == path) {
                for k in knots {
                    if let Some(sub) = entry.subpaths.get_mut(k.subpath) {
                        sub.remove_knot(k.knot);
                    }
                }
            }
        }
    });
    fixture.settle();
    Harness { fixture, state }
}

fn two_paths() -> Harness {
    harness(vec![path_a(), path_b()])
}

fn with_tool(tool: BezierTool) -> Harness {
    let mut h = harness(tool_paths());
    h.update(|st| st.tool = tool);
    h
}

impl Harness {
    /// Changes the state and shows a settled frame.
    fn update(&mut self, change: impl FnOnce(&mut State)) {
        change(&mut self.state.borrow_mut());
        self.fixture.settle();
    }

    fn screen(&self, at: Point) -> Pos2 {
        self.state.borrow().view.expect("shown").to_screen(at)
    }

    fn anchor(&self, path: usize, knot: usize) -> Point {
        self.state.borrow().paths[path].subpaths[0].knots[knot].anchor
    }

    fn selection(&self) -> PathSelection {
        self.state.borrow().selection.clone()
    }

    fn seen(&self) -> Vec<Seen> {
        self.state.borrow().seen.clone()
    }

    fn last_change(&self) -> Option<PathEdit> {
        self.state.borrow().changes.last().cloned()
    }

    fn held(&mut self, mods: Modifiers) -> &mut Self {
        self.fixture.hold(mods);
        self
    }

    fn press_at(&mut self, at: Pos2) {
        self.fixture.frame(vec![pointer::moved(at)]);
        self.fixture.frame(vec![pointer::press(at)]);
    }

    fn move_to(&mut self, at: Pos2) {
        self.fixture.frame(vec![pointer::moved(at)]);
    }

    fn release_at(&mut self, at: Pos2) {
        self.fixture.frame(vec![pointer::release(at)]);
        self.fixture.frame(Vec::new());
    }

    /// A drag over several frames: press, two moves, release.
    fn drag(&mut self, from: Pos2, by: ScreenVec) {
        self.press_at(from);
        self.move_to(from + by * 0.5);
        self.move_to(from + by);
        self.release_at(from + by);
    }

    fn click(&mut self, at: Pos2) {
        self.fixture.click_at(at);
    }

    fn click_with(&mut self, at: Pos2, mods: Modifiers) {
        self.fixture.hold(mods);
        self.fixture.click_at(at);
        self.fixture.hold(Modifiers::NONE);
    }

    fn click_doc(&mut self, x: f64, y: f64) {
        let at = self.screen(p(x, y));
        self.click(at);
    }

    fn key(&mut self, key: Key, mods: Modifiers) {
        self.fixture.key(key, mods);
    }

    /// Tab 1 is the canvas, Tab 2 the part the overlay enters on: the selected anchor, else the first.
    fn tab_to_parts(&mut self, entry: &str) {
        self.key(Key::Tab, Modifiers::NONE);
        assert!(self.fixture.focused_stop().is_some_and(|(_, part)| part.is_none()), "Tab 1 is the canvas");
        self.key(Key::Tab, Modifiers::NONE);
        assert!(self.picked(entry), "Tab 2 is {entry}");
    }

    /// Whether the focused part is the one named `name`.
    fn picked(&self, name: &str) -> bool {
        let Some((owner, Some(part))) = self.fixture.focused_stop() else { return false };
        self.fixture.part(owner, part).map(|t| t.node) == self.fixture.by_label(name).map(|t| t.node)
    }

    /// The names of the overlay's graphics objects, in tree order.
    fn objects(&self) -> Vec<String> {
        let tree = self.fixture.tree();
        self.fixture.by_role(Role::GRAPHICS_OBJECT).iter().filter_map(|t| tree.name_of(t.node).map(str::to_owned)).collect()
    }
}

fn moved_by(edit: Option<PathEdit>) -> Option<(Vec<KnotRef>, Vec2)> {
    match edit? {
        PathEdit::MoveAnchors { anchors, by, .. } => Some((anchors, by)),
        _ => None,
    }
}

fn sel(path: u64, anchors: &[(usize, usize)]) -> PathSelection {
    PathSelection::new(Some(path), anchors.iter().map(|&(s, k)| KnotRef::new(s, k)).collect(), None)
}

/// A distance past any click slop.
const FAR: f32 = 20.0;

#[trial]
fn a_shown_handle_beats_an_anchor_which_beats_a_segment_at_equal_distance() {
    // A smooth knot with short handles, selected so they show.
    let knots =
        vec![Knot::corner(p(100.0, 150.0)), Knot::new(p(200.0, 150.0), p(194.0, 150.0), p(206.0, 150.0), true), Knot::corner(p(300.0, 150.0))];
    let mut h = harness(vec![EditPath::new(7, vec![EditSubpath::new(knots, false)])]);
    h.update(|st| st.selection = sel(7, &[(0, 1)]));
    // 5 points from both the out handle's end and the anchor.
    let at = h.screen(p(203.0, 154.0));
    h.drag(at, vec2(0.0, FAR));
    assert!(matches!(h.last_change(), Some(PathEdit::MoveHandle { side: HandleSide::Out, .. })), "{:?}", h.last_change());

    // On path 1, 3 points above anchor 2 and above the straight segment through it.
    let mut h = two_paths();
    let at = h.screen(p(160.0, 57.0));
    h.drag(at, vec2(0.0, FAR));
    assert_eq!(moved_by(h.last_change()), Some((vec![kr(1)], Vec2::new(0.0, f64::from(FAR)))), "the anchor alone moves");
    assert_eq!(h.selection(), sel(1, &[(0, 1)]));
}

#[trial]
fn a_segment_takes_the_pointer_near_its_curve_in_a_rotated_flipped_view() {
    for (offset, takes) in [(2.0, true), (-2.0, true), (40.0, false)] {
        let mut h = harness(vec![path_a()]);
        h.update(|st| (st.rotation, st.flip) = (30.0, true));
        let view = h.state.borrow().view.expect("shown");
        let curve = h.state.borrow().paths[0].subpaths[0].segment(2).expect("segment");
        let on = view.to_screen(curve.eval(0.5));
        // The curve's tangent there as the view draws it, turned a quarter.
        let ahead = view.to_screen(curve.eval(0.5 + 1e-4));
        let tangent = (ahead - on).normalized();
        let at = on + vec2(-tangent.y, tangent.x) * offset;
        h.drag(at, vec2(15.0, 0.0));
        let bent = matches!(h.last_change(), Some(PathEdit::BendSegment { segment, .. }) if segment == kr(2));
        assert_eq!(bent, takes, "offset {offset}");
    }
}

#[trial]
fn shift_toggles_an_anchor_and_alt_selects_its_subpath() {
    let mut h = two_paths();
    let (a2, a4) = (h.screen(h.anchor(0, 1)), h.screen(h.anchor(0, 3)));
    h.click(a2);
    assert_eq!(h.selection(), sel(1, &[(0, 1)]));
    h.click_with(a4, Modifiers::SHIFT);
    assert_eq!(h.selection(), sel(1, &[(0, 1), (0, 3)]));
    // A press that deselects drags nothing.
    h.held(Modifiers::SHIFT).drag(a2, vec2(30.0, 0.0));
    h.held(Modifiers::NONE);
    assert_eq!(h.selection(), sel(1, &[(0, 3)]));
    assert!(h.seen().is_empty(), "{:?}", h.seen());
    let line = h.screen(h.state.borrow().paths[1].subpaths[1].knots[0].anchor);
    h.click_with(line, Modifiers::ALT);
    assert_eq!(h.selection(), sel(2, &[(1, 0), (1, 1)]), "alt selects the anchor's subpath");
}

#[trial]
fn a_drag_moves_the_selected_anchors_and_shift_keeps_it_at_45_degrees() {
    let mut h = two_paths();
    let (a1, a2) = (h.screen(h.anchor(0, 0)), h.screen(h.anchor(0, 1)));
    h.click(a1);
    h.click_with(a2, Modifiers::SHIFT);
    // Shift held after the press constrains the move; held at the press it would toggle.
    h.press_at(a2);
    h.move_to(a2 + vec2(10.0, 0.0));
    h.held(Modifiers::SHIFT);
    h.move_to(a2 + vec2(30.0, 26.0));
    h.release_at(a2 + vec2(30.0, 26.0));
    h.held(Modifiers::NONE);
    assert_eq!(h.seen().first(), Some(&Seen::Begin));
    assert_eq!(h.seen().last(), Some(&Seen::Commit));
    let (anchors, by) = moved_by(h.last_change()).expect("a move");
    assert_eq!(anchors, [kr(0), kr(1)]);
    assert!(near_vec(by, Vec2::new(28.0, 28.0)), "{by:?}");
    assert!(near_vec(h.anchor(0, 0) - p(60.0, 60.0), by), "the paths carry the move");
}

#[trial]
fn a_straight_segment_moves_its_anchors_and_a_curved_one_bends() {
    let mut h = two_paths();
    let mid = h.screen(p(110.0, 60.0));
    h.drag(mid, vec2(0.0, FAR));
    assert_eq!(moved_by(h.last_change()), Some((vec![kr(0), kr(1)], Vec2::new(0.0, f64::from(FAR)))));
    assert_eq!(h.selection().segment, Some(kr(0)), "a press on a segment selects it");

    let mut h = two_paths();
    let curve = h.state.borrow().paths[0].subpaths[0].segment(1).expect("segment");
    let at = h.screen(curve.eval(0.5));
    h.drag(at, vec2(0.0, 12.0));
    match h.last_change() {
        Some(PathEdit::BendSegment { path: 1, segment, t, by, .. }) => {
            assert_eq!(segment, kr(1));
            assert!((t - 0.5).abs() < 0.01, "t {t}");
            assert!(near_vec(by, Vec2::new(0.0, 12.0)), "{by:?}");
        }
        other => panic!("{other:?}"),
    }
}

#[trial]
fn a_release_within_the_click_slop_is_a_click_with_no_event() {
    let mut h = two_paths();
    let a2 = h.screen(h.anchor(0, 1));
    h.drag(a2, vec2(1.0, 0.0));
    assert!(h.seen().is_empty(), "{:?}", h.seen());
    assert_eq!(h.selection(), sel(1, &[(0, 1)]), "the click selects");
    h.drag(a2, vec2(FAR, 0.0));
    assert_eq!(h.seen().last(), Some(&Seen::Commit));
}

#[trial]
fn a_quick_drag_in_one_pass_moves_from_the_press() {
    let mut h = two_paths();
    let a2 = h.screen(h.anchor(0, 1));
    let to = a2 + vec2(25.0, 5.0);
    h.fixture.frame(vec![pointer::moved(a2)]);
    h.fixture.frame_through(vec![pointer::press(a2), pointer::moved(a2 + vec2(10.0, 0.0)), pointer::moved(to), pointer::release(to)]);
    h.fixture.frame(Vec::new());
    assert_eq!(h.seen(), [Seen::Begin, Seen::Commit], "a drag is one gesture, whatever the passes it spans");
    assert_eq!(moved_by(h.last_change()), Some((vec![kr(1)], Vec2::new(25.0, 5.0))));
}

#[trial]
fn an_anchor_beats_a_guide_under_it() {
    let mut h = harness(vec![path_a()]);
    h.update(|st| st.guides = vec![Guide::new(GuideKey::new(1), craft_behavior::Axis::Y, 160.0)]);
    let a2 = h.screen(h.anchor(0, 1));
    h.drag(a2, vec2(FAR, 0.0));
    assert_eq!(h.state.borrow().guides[0].position, 160.0, "the guide stays");
    assert_eq!(moved_by(h.last_change()), Some((vec![kr(1)], Vec2::new(f64::from(FAR), 0.0))));
}

#[trial]
fn convert_point_makes_a_corner_on_a_click_and_smooth_handles_on_a_drag() {
    let mut h = two_paths();
    h.update(|st| st.tool = BezierTool::Convert);
    let a3 = h.screen(h.anchor(0, 2));
    h.click(a3);
    assert_eq!(h.seen(), [Seen::Set]);
    assert_eq!(h.last_change(), Some(PathEdit::convert(1, kr(2), None)));
    let a2 = h.screen(h.anchor(0, 1));
    h.drag(a2, vec2(30.0, 10.0));
    assert_eq!(h.last_change(), Some(PathEdit::convert(1, kr(1), Some(p(190.0, 70.0)))));
    assert_eq!(h.state.borrow().paths[0].subpaths[0].knots[1], Knot::smooth(p(160.0, 60.0), p(190.0, 70.0)));
    // Segments and empty canvas are the tool's.
    let before = h.seen().len();
    let clicks = h.state.borrow().tool_clicks;
    let mid = h.screen(p(110.0, 60.0));
    h.click(mid);
    assert_eq!(h.seen().len(), before);
    assert_eq!(h.state.borrow().tool_clicks, clicks + 1, "the press reached the tool");
}

#[trial]
fn a_marquee_selects_on_the_frontmost_path_and_shift_adds() {
    let mut h = two_paths();
    // Round anchor 1 of path 1 and the top of path 2's square.
    let (from, to) = (h.screen(p(40.0, 40.0)), h.screen(p(150.0, 210.0)));
    h.drag(from, to - from);
    assert_eq!(h.selection(), sel(2, &[(0, 0), (0, 1)]));
    assert!(h.seen().is_empty(), "a marquee is no path edit");
    let (from, to) = (h.screen(p(130.0, 250.0)), h.screen(p(150.0, 275.0)));
    h.held(Modifiers::SHIFT).drag(from, to - from);
    h.held(Modifiers::NONE);
    assert_eq!(h.selection(), sel(2, &[(0, 0), (0, 1), (0, 2)]));
    h.click_doc(380.0, 280.0);
    assert_eq!(h.selection(), PathSelection::default(), "a click on empty canvas deselects");
}

#[trial]
fn page_keys_pick_each_anchor_then_its_in_and_out_handles() {
    let mut h = harness(vec![path_a()]);
    h.tab_to_parts("Anchor 1 of 4");
    assert!(h.picked("Anchor 1 of 4"));
    h.key(Key::PageDown, Modifiers::NONE);
    assert!(h.picked("Anchor 2 of 4"));
    assert_eq!(h.selection(), sel(1, &[(0, 1)]), "picking an anchor selects it alone");
    h.key(Key::PageDown, Modifiers::NONE);
    assert!(h.picked("Anchor 3 of 4"));
    h.key(Key::PageDown, Modifiers::NONE);
    assert!(h.picked("In handle"));
    h.key(Key::PageDown, Modifiers::NONE);
    assert!(h.picked("Out handle"));
    h.key(Key::PageDown, Modifiers::NONE);
    assert!(h.picked("Anchor 4 of 4"));
    assert_eq!(h.selection(), sel(1, &[(0, 3)]));
    h.key(Key::PageUp, Modifiers::NONE);
    assert!(h.picked("Out handle"));
    h.key(Key::Home, Modifiers::NONE);
    assert!(h.picked("Anchor 1 of 4"));
    h.key(Key::End, Modifiers::NONE);
    assert!(h.picked("Anchor 4 of 4"));
}

#[trial]
fn shift_page_keys_add_the_next_anchor_and_command_a_selects_all() {
    let mut h = harness(vec![path_a()]);
    h.tab_to_parts("Anchor 1 of 4");
    h.key(Key::PageDown, Modifiers::SHIFT);
    h.key(Key::PageDown, Modifiers::SHIFT);
    assert_eq!(h.selection(), sel(1, &[(0, 0), (0, 1), (0, 2)]));
    assert!(h.picked("Anchor 3 of 4"));
    h.key(Key::Escape, Modifiers::NONE);
    assert_eq!(h.selection(), PathSelection::default(), "Escape deselects");
    h.key(Key::A, Modifiers::COMMAND);
    assert_eq!(h.selection(), sel(1, &[(0, 0), (0, 1), (0, 2), (0, 3)]));
}

#[trial]
fn arrows_move_the_selected_anchors_or_the_picked_handle_and_alt_moves_it_alone() {
    let mut h = harness(vec![path_a()]);
    h.tab_to_parts("Anchor 1 of 4");
    h.key(Key::ArrowRight, Modifiers::NONE);
    h.key(Key::ArrowDown, Modifiers::SHIFT);
    assert_eq!(h.seen(), [Seen::Set, Seen::Set]);
    assert_eq!(h.anchor(0, 0), p(61.0, 70.0));
    // Anchor 3's in handle.
    for _ in 0..4 {
        h.key(Key::PageDown, Modifiers::NONE);
    }
    assert!(h.picked("In handle"));
    h.key(Key::ArrowLeft, Modifiers::ALT);
    let moved = |to: Point, alone: bool| Some(PathEdit::move_handle(1, kr(2), HandleSide::In, to, alone));
    assert_eq!(h.last_change(), moved(p(229.0, 20.0), true));
    h.key(Key::ArrowUp, Modifiers::NONE);
    assert_eq!(h.last_change(), moved(p(229.0, 19.0), false));
}

#[trial]
fn delete_reports_the_selected_anchors_to_remove() {
    let mut h = harness(vec![path_a()]);
    h.tab_to_parts("Anchor 1 of 4");
    h.key(Key::Delete, Modifiers::NONE);
    assert_eq!(h.state.borrow().deletes, [(1, vec![kr(0)])]);
    h.key(Key::PageDown, Modifiers::NONE);
    h.key(Key::Backspace, Modifiers::NONE);
    assert_eq!(h.state.borrow().deletes.len(), 2, "Backspace removes too");
}

#[trial]
fn anchors_and_shown_handles_are_graphics_objects_until_the_anchor_limit() {
    let mut h = harness(vec![path_a()]);
    h.update(|st| st.selection = sel(1, &[(0, 2)]));
    assert_eq!(h.objects(), ["Anchor 1 of 4", "Anchor 2 of 4", "Anchor 3 of 4", "In handle", "Out handle", "Anchor 4 of 4"]);
    let tree = h.fixture.tree();
    let values: Vec<_> = h.fixture.by_role(Role::GRAPHICS_OBJECT).iter().map(|t| tree.value_of(t.node).map(str::to_owned)).collect();
    assert_eq!(values[0].as_deref(), Some("60.0 px, 60.0 px"));
    assert_eq!(values[3].as_deref(), Some("230.0 px, 20.0 px"));

    let knots = (0..=MAX_ANCHORS).map(|i| Knot::corner(p(i as f64 * 0.001, 100.0))).collect();
    let h = harness(vec![EditPath::new(9, vec![EditSubpath::new(knots, false)])]);
    assert!(h.objects().is_empty(), "over the limit no object");
}

// `hit_test` answers what a press takes, without a pass: handles, then anchors, then segments, in a rotated and
// flipped view, as the overlay's own press does.
#[trial]
fn hit_test_matches_the_overlays_press_priority() {
    for (rotation, flip) in [(0.0, false), (40.0, true)] {
        let mut h = two_paths();
        h.update(|st| (st.rotation, st.flip, st.selection) = (rotation, flip, sel(1, &[(0, 2)])));
        let view = h.state.borrow().view.expect("shown");
        let test = |at: Pos2| {
            let st = h.state.borrow();
            BezierOverlay::hit_test(&st.paths, &st.selection, HandlesShown::Selected, &view, at, 6.0)
        };
        let handle = h.screen(p(290.0, 100.0));
        assert!(matches!(test(handle), Some(BezierHit::Handle { path: 1, knot, side: HandleSide::Out, .. }) if knot == kr(2)));
        assert!(matches!(test(h.screen(p(160.0, 60.0))), Some(BezierHit::Anchor { path: 1, knot, .. }) if knot == kr(1)));
        assert!(matches!(test(h.screen(p(110.0, 60.0))), Some(BezierHit::Segment { path: 1, segment, .. }) if segment == kr(0)));
        assert_eq!(test(h.screen(p(10.0, 10.0))), None);
        // The press on the handle drags it, as the test says.
        h.drag(handle, vec2(0.0, FAR));
        assert!(matches!(h.last_change(), Some(PathEdit::MoveHandle { side: HandleSide::Out, .. })), "{:?}", h.last_change());
    }
}

// Hovering a segment under Add reports the segment with its parameter, which the preview anchor stands at.
#[trial]
fn the_hover_reports_the_part_under_the_pointer_with_the_segment_parameter() {
    let mut h = with_tool(BezierTool::AddAnchor);
    let at = h.screen(p(110.0, 60.0));
    h.move_to(at);
    h.fixture.settle();
    match h.state.borrow().hover {
        Some(BezierHit::Segment { path: 1, segment, t, .. }) => assert!(segment == kr(0) && (t - 0.5).abs() < 0.05, "{segment:?} {t}"),
        other => panic!("{other:?}"),
    }
    let a2 = h.screen(p(160.0, 60.0));
    h.move_to(a2);
    h.fixture.settle();
    assert!(matches!(h.state.borrow().hover, Some(BezierHit::Anchor { path: 1, .. })));
}

// A selected segment shows its two handles (an object each), and no thicker line is a look the paint reads.
#[trial]
fn a_selected_segment_shows_its_handles() {
    let mut h = two_paths();
    h.update(|st| st.selection = PathSelection::new(Some(1), Vec::new(), Some(kr(2))));
    assert!(h.objects().iter().any(|n| n == "Out handle"), "the segment's leaving end shows its handle: {:?}", h.objects());
    h.update(|st| st.segment_mark = SegmentMark::Line);
    assert!(h.objects().iter().any(|n| n == "Out handle"));
}

// `AnchorsShown::Hidden` draws outlines only: no anchor, no handle, no part, no AccessKit object.
#[trial]
fn anchors_hidden_draws_outlines_only() {
    let mut h = two_paths();
    h.update(|st| st.selection = sel(1, &[(0, 2)]));
    assert!(!h.objects().is_empty());
    h.update(|st| st.anchors = AnchorsShown::Hidden);
    assert!(h.objects().is_empty(), "no AccessKit part");
    let a2 = h.screen(h.anchor(0, 1));
    let clicks = h.state.borrow().tool_clicks;
    h.click(a2);
    assert_eq!(h.state.borrow().tool_clicks, clicks + 1, "without parts the press reaches the tool");
    h.update(|st| st.anchors = AnchorsShown::All);
    assert!(!h.objects().is_empty(), "anchors back");
}

// ---- the anchor tools and the Curvature tool ----

#[trial]
fn add_anchor_tool_reports_insert_on_a_segment_click_and_alt_swaps() {
    let mut h = with_tool(BezierTool::AddAnchor);
    h.click_doc(110.0, 60.0);
    let [(path, segment, t)] = h.state.borrow().inserts[..] else { panic!("one insert: {:?}", h.state.borrow().inserts) };
    assert_eq!((path, segment), (1, kr(0)));
    assert!(t > 0.0 && t < 1.0);
    assert_eq!(h.state.borrow().tool_clicks, 0, "a segment is the tool's");
    assert_eq!(h.selection(), PathSelection::default(), "no selection change");

    h.click_doc(260.0, 60.0);
    h.click_doc(60.0, 60.0);
    assert_eq!((h.state.borrow().inserts.len(), h.state.borrow().deletes.len()), (1, 0), "nothing reported for an anchor");
    assert_eq!(h.state.borrow().tool_clicks, 0, "an anchor is taken too");

    let a1 = h.screen(p(60.0, 60.0));
    h.click_with(a1, Modifiers::ALT);
    assert_eq!(h.state.borrow().deletes, [(1, vec![kr(0)])]);
}

#[trial]
fn delete_anchor_tool_reports_delete_on_an_anchor_click() {
    let mut h = with_tool(BezierTool::DeleteAnchor);
    h.click_doc(260.0, 60.0);
    assert_eq!(h.state.borrow().deletes, [(1, vec![kr(2)])]);
    h.click_doc(110.0, 60.0);
    assert_eq!((h.state.borrow().inserts.len(), h.state.borrow().deletes.len()), (0, 1), "nothing reported for a segment");
    let mid = h.screen(p(110.0, 60.0));
    h.click_with(mid, Modifiers::ALT);
    assert!(matches!(h.state.borrow().inserts[..], [(1, segment, _)] if segment == kr(0)), "{:?}", h.state.borrow().inserts);
}

#[trial]
fn auto_add_delete_leaves_open_ends_to_the_pen() {
    let mut h = with_tool(BezierTool::AutoAddDelete);
    h.click_doc(60.0, 60.0);
    h.click_doc(260.0, 60.0);
    h.click_doc(360.0, 20.0);
    assert_eq!(h.state.borrow().tool_clicks, 3, "the open ends and empty canvas reach the pen");
    assert_eq!((h.state.borrow().inserts.len(), h.state.borrow().deletes.len()), (0, 0));

    h.click_doc(160.0, 60.0);
    assert_eq!(h.state.borrow().deletes, [(1, vec![kr(1)])], "an inner anchor");
    h.click_doc(60.0, 200.0);
    assert_eq!(h.state.borrow().deletes.len(), 2, "a closed subpath's first anchor");
    h.click_doc(140.0, 230.0);
    assert_eq!(h.state.borrow().inserts.len(), 1, "a segment");
    assert_eq!(h.state.borrow().tool_clicks, 3);
}

#[trial]
fn anchor_tools_edit_the_pen_draft_passed_as_a_path() {
    let mut h = with_tool(BezierTool::AddAnchor);
    h.click_doc(110.0, 150.0);
    assert!(matches!(h.state.borrow().inserts[..], [(DRAFT, segment, _)] if segment == kr(0)), "{:?}", h.state.borrow().inserts);
    let draft = |h: &Harness| h.state.borrow().paths[2].subpaths[0].clone();
    assert_eq!(draft(&h).knots.len(), 4);

    h.update(|st| st.tool = BezierTool::DeleteAnchor);
    h.click_doc(160.0, 150.0);
    assert_eq!(h.state.borrow().deletes, [(DRAFT, vec![kr(2)])]);
    let after = draft(&h);
    assert_eq!(after.knots.len(), 3);
    assert!(!after.closed, "the draft stays open");
    assert!(after.knots.last().is_some_and(|k| k.anchor == p(260.0, 150.0)), "drawing goes on from the last anchor");
}

#[trial]
fn curvature_tool_drags_an_anchor_and_reflows() {
    let mut h = with_tool(BezierTool::Curvature);
    let at = h.screen(p(160.0, 60.0));
    h.drag(at, vec2(0.0, 30.0));
    assert_eq!(h.seen().first(), Some(&Seen::Begin), "{:?}", h.seen());
    assert_eq!(h.seen().last(), Some(&Seen::Commit));
    let Some(PathEdit::MoveCurvature { path, anchors, by, .. }) = h.last_change() else { panic!("MoveCurvature: {:?}", h.last_change()) };
    assert_eq!((path, anchors), (1, vec![kr(1)]));
    assert!(near_vec(by, Vec2::new(0.0, 30.0)), "{by:?}");
    assert_eq!(h.selection(), sel(1, &[(0, 1)]));
    assert_eq!(h.anchor(0, 1), p(160.0, 90.0), "the paths carry the move");

    // Escape cancels; the rest of the press does nothing.
    let mut h = with_tool(BezierTool::Curvature);
    let before = h.state.borrow().paths.clone();
    let at = h.screen(p(160.0, 60.0));
    h.press_at(at);
    h.move_to(at + vec2(0.0, 10.0));
    h.move_to(at + vec2(0.0, 20.0));
    h.key(Key::Escape, Modifiers::NONE);
    assert_eq!(h.seen().last(), Some(&Seen::Cancel), "{:?}", h.seen());
    let n = h.seen().len();
    h.move_to(at + vec2(0.0, 50.0));
    h.release_at(at + vec2(0.0, 50.0));
    assert_eq!(h.seen().len(), n, "nothing after the cancel");
    assert_eq!(h.state.borrow().paths, before);
}

#[trial]
fn curvature_tool_insert_and_drag_is_one_gesture() {
    let mut h = with_tool(BezierTool::Curvature);
    let at = h.screen(p(110.0, 60.0));
    h.drag(at, vec2(0.0, 24.0));
    assert_eq!(h.seen().first(), Some(&Seen::Begin), "{:?}", h.seen());
    assert_eq!(h.seen().last(), Some(&Seen::Commit));
    assert!(h.seen().iter().all(|k| matches!(k, Seen::Begin | Seen::Update | Seen::Commit)), "{:?}", h.seen());
    let Some(PathEdit::InsertCurvature { path, segment, t, by, .. }) = h.last_change() else { panic!("InsertCurvature: {:?}", h.last_change()) };
    assert_eq!((path, segment), (1, kr(0)));
    assert!((0.01..=0.99).contains(&t) && (t - 0.5).abs() < 0.05, "{t}");
    assert!(near_vec(by, Vec2::new(0.0, 24.0)), "{by:?}");
    assert_eq!(h.selection().anchors, [kr(1)], "the new anchor is selected");
    assert_eq!(h.state.borrow().lone_selects, 0, "the selection came with the first Update");
}

#[trial]
fn curvature_tool_click_on_a_segment_inserts_only() {
    let mut h = with_tool(BezierTool::Curvature);
    h.click_doc(110.0, 60.0);
    assert_eq!(h.seen(), [Seen::Set]);
    let Some(PathEdit::InsertCurvature { path, segment, by, .. }) = h.last_change() else { panic!("InsertCurvature: {:?}", h.last_change()) };
    assert_eq!((path, segment, by), (1, kr(0), Vec2::ZERO));
    assert_eq!(h.state.borrow().lone_selects, 0, "the selection came with the insert");
    assert_eq!(h.selection(), sel(1, &[(0, 1)]));
}

#[trial]
fn curvature_tool_double_click_toggles_an_anchor() {
    let mut h = with_tool(BezierTool::Curvature);
    h.click_doc(160.0, 60.0);
    assert!(h.seen().is_empty(), "a single click on an anchor reports nothing");
    let at = h.screen(p(160.0, 60.0));
    h.fixture.frame(vec![pointer::moved(at)]);
    h.fixture.frame(vec![pointer::press(at)]);
    h.fixture.frame(vec![pointer::release(at)]);
    h.fixture.frame(Vec::new());
    h.click(at);
    assert_eq!(h.last_change(), Some(PathEdit::toggle_curvature(1, kr(1))));
}

#[trial]
fn curvature_tool_does_not_take_empty_canvas() {
    let mut h = with_tool(BezierTool::Curvature);
    h.click_doc(360.0, 20.0);
    assert!(h.seen().is_empty());
    assert_eq!(h.state.borrow().tool_clicks, 1, "empty canvas is the pen's");
    h.click_doc(110.0, 60.0);
    assert_eq!(h.state.borrow().tool_clicks, 1, "a segment is taken");
}

#[trial]
fn curvature_tool_keys_move_and_toggle() {
    let mut h = with_tool(BezierTool::Curvature);
    h.click_doc(160.0, 60.0);
    h.key(Key::ArrowRight, Modifiers::NONE);
    assert_eq!(h.last_change(), Some(PathEdit::move_curvature(1, vec![kr(1)], Vec2::new(1.0, 0.0))));
    h.key(Key::ArrowDown, Modifiers::SHIFT);
    assert_eq!(h.last_change(), Some(PathEdit::move_curvature(1, vec![kr(1)], Vec2::new(0.0, 10.0))));
    h.key(Key::Enter, Modifiers::NONE);
    assert_eq!(h.last_change(), Some(PathEdit::toggle_curvature(1, kr(1))));
    assert_eq!(h.seen(), [Seen::Set, Seen::Set, Seen::Set]);
}

/// Selects anchor 1 of path 1, whose in handle lies on the first segment, so a handle is shown over the segment.
fn with_handle_over_the_segment(h: &mut Harness) {
    h.update(|st| {
        st.paths[0].subpaths[0].knots[1].in_ctrl = p(110.0, 60.0);
        st.selection = sel(1, &[(0, 1)]);
    });
}

// A shown handle does not hide the segment under it for the anchor tools and Curvature: the press inserts instead
// of being left to the pen draft.
#[trial]
fn a_shown_handle_does_not_hide_the_segment_for_the_anchor_tools_and_curvature() {
    let mut h = with_tool(BezierTool::Curvature);
    with_handle_over_the_segment(&mut h);
    h.click_doc(110.0, 60.0);
    assert!(matches!(h.last_change(), Some(PathEdit::InsertCurvature { .. })), "{:?}", h.last_change());

    for tool in [BezierTool::AddAnchor, BezierTool::AutoAddDelete] {
        let mut h = with_tool(tool);
        with_handle_over_the_segment(&mut h);
        h.click_doc(110.0, 60.0);
        assert_eq!(h.state.borrow().inserts.len(), 1, "{tool:?}");
    }

    // Direct still grabs the handle.
    let mut h = with_tool(BezierTool::Direct);
    with_handle_over_the_segment(&mut h);
    let at = h.screen(p(110.0, 60.0));
    h.drag(at, vec2(0.0, 30.0));
    assert!(matches!(h.last_change(), Some(PathEdit::MoveHandle { .. })), "{:?}", h.last_change());
}

// Arrows, select all and Escape belong to Direct, Convert and Curvature; Add, Delete and Auto Add/Delete have none.
#[trial]
fn arrows_and_escape_act_only_under_direct_convert_and_curvature() {
    for (tool, acts) in [
        (BezierTool::Direct, true),
        (BezierTool::Convert, true),
        (BezierTool::Curvature, true),
        (BezierTool::AddAnchor, false),
        (BezierTool::DeleteAnchor, false),
        (BezierTool::AutoAddDelete, false),
    ] {
        let mut h = with_tool(tool);
        h.update(|st| st.selection = sel(1, &[(0, 1)]));
        h.tab_to_parts("Anchor 2 of 4");
        h.key(Key::ArrowRight, Modifiers::NONE);
        assert_eq!(!h.seen().is_empty(), acts, "{tool:?} arrows");
        h.key(Key::Escape, Modifiers::NONE);
        assert_eq!(h.selection() == PathSelection::default(), acts, "{tool:?} Escape");
    }
}

#[trial]
fn diag_page_keys() {
    let mut h = harness(vec![path_a()]);
    h.tab_to_parts("Anchor 1 of 4");
    let mut log = Vec::new();
    for i in 0..6 {
        h.key(Key::PageDown, Modifiers::NONE);
        let focus = h.fixture.focused_stop().map(|(_, p)| p.map(|p| (p.key, p.index)));
        log.push(format!("{i}: focus {focus:?} sel {:?} objects {:?}", h.selection(), h.objects()));
    }
    panic!("DIAG\n{}", log.join("\n"));
}
