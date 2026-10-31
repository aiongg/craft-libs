use egui::accesskit::Role;
use egui::{Color32, Event, Key, Modifiers, PointerButton, Pos2, Sense, Shape, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use kurbo::{ParamCurve as _, ParamCurveDeriv as _, Point, Vec2};

use super::story::PathHost;
use super::{
    BezierHit, BezierNames, BezierOverlay, BezierResponse, BezierTool, EditPath, EditSubpath, HandleSide, HandlesShown, Knot, KnotRef, MAX_ANCHORS,
    PathEdit, PathSelection,
};
use crate::canvas::{CanvasView, Guide, GuideNames, Guides, Orientation, RulerUnits};
use crate::edit::{CancelReason, EditEvent};
use crate::localized::LocaleNumbers;
use crate::primitives::FocusRing;
use crate::testing::TypingField;
use crate::theme::{Base, current, install};

/// A host with a 400 × 300 canvas showing document (0, 0)–(400, 300) at zoom 1.
struct Host {
    paths: PathHost,
    rotation: f64,
    flip: bool,
    tool: BezierTool,
    handles: HandlesShown,
    /// A vertical guide at this x, shown after the paths.
    guide: Option<f64>,
    view: Option<CanvasView>,
    events: Vec<EditEvent<PathEdit>>,
    guide_events: usize,
    last: Option<BezierResponse>,
    inserts: Vec<(u64, KnotRef, f64)>,
    deletes: Vec<(u64, Vec<KnotRef>)>,
    field: TypingField,
    modifiers: Option<Modifiers>,
    highlight: Option<BezierHit>,
}

/// Path 1: an open subpath of four anchors: a straight segment, then two curved ones.
fn path_a() -> EditPath {
    let knots = vec![
        Knot::corner(Point::new(60.0, 60.0)),
        Knot::corner(Point::new(160.0, 60.0)),
        Knot { anchor: Point::new(260.0, 60.0), in_ctrl: Point::new(230.0, 20.0), out_ctrl: Point::new(290.0, 100.0), smooth: true },
        Knot::corner(Point::new(340.0, 160.0)),
    ];
    EditPath { key: 1, subpaths: vec![EditSubpath { knots, closed: false }], color: None }
}

/// Path 2, drawn after path 1: a closed square and an open line.
fn path_b() -> EditPath {
    let square = [(60.0, 200.0), (140.0, 200.0), (140.0, 260.0), (60.0, 260.0)].map(|(x, y)| Knot::corner(Point::new(x, y)));
    let line = [(200.0, 220.0), (260.0, 260.0)].map(|(x, y)| Knot::corner(Point::new(x, y)));
    let subpaths = vec![EditSubpath { knots: square.to_vec(), closed: true }, EditSubpath { knots: line.to_vec(), closed: false }];
    EditPath { key: 2, subpaths, color: Some(Color32::from_rgb(230, 110, 20)) }
}

fn harness(paths: Vec<EditPath>) -> Harness<'static, Host> {
    let host = Host {
        paths: PathHost::new(paths),
        rotation: 0.0,
        flip: false,
        tool: BezierTool::Direct,
        handles: HandlesShown::Selected,
        guide: None,
        view: None,
        events: Vec::new(),
        guide_events: 0,
        last: None,
        inserts: Vec::new(),
        deletes: Vec::new(),
        field: TypingField::default(),
        modifiers: None,
        highlight: None,
    };
    Harness::<Host>::builder().with_size([480.0, 400.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            host.field.show(ui);
            ui.add_space(40.0);
            // Not a Tab stop, so Tab goes to the paths.
            let (rect, canvas) = ui.allocate_exact_size(vec2(400.0, 300.0), Sense::click_and_drag() - Sense::FOCUSABLE);
            let view = CanvasView { rotation: host.rotation, flip_x: host.flip, ..CanvasView::new(rect, Point::new(200.0, 150.0), 1.0) };
            host.view = Some(view);
            let px = RulerUnits::pixels(LocaleNumbers::from_tag("en"));
            let names = BezierNames::new("Paths", "In handle", "Out handle", "Selection", |i, n| format!("Anchor {i} of {n}"));
            let mut overlay = BezierOverlay::new("paths", &host.paths.paths, &host.paths.selection, &px, &px, names);
            if let Some(m) = host.modifiers {
                overlay = overlay.modifiers(m);
            }
            let response = overlay.highlight(host.highlight).tool(host.tool).handles(host.handles).show(ui, &canvas, &view);
            if let Some(x) = host.guide {
                let guides = [Guide::new(1, Orientation::Vertical, x)];
                let names = GuideNames::new("Guides", "Vertical guide", "Horizontal guide");
                let g = Guides::new("guides", &guides, &px, &px, names).interactive(true).show(ui, &canvas, &view);
                host.guide_events += usize::from(g.event.is_some());
            }
            host.field.note(response.event.is_some());
            host.events.extend(response.event.clone());
            host.inserts.extend(response.insert);
            host.deletes.extend(response.delete.clone());
            host.paths.apply(&response);
            host.last = Some(response);
        },
        host,
    )
}

fn two_paths() -> Harness<'static, Host> {
    let mut h = harness(vec![path_a(), path_b()]);
    h.run();
    h
}

fn screen(h: &Harness<'_, Host>, p: Point) -> Pos2 {
    h.state().view.expect("drawn").to_screen(p)
}

fn button(pos: Pos2, pressed: bool, modifiers: Modifiers) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers }
}

fn press(h: &mut Harness<'_, Host>, at: Pos2, modifiers: Modifiers) {
    h.event(Event::ModifiersChanged(modifiers));
    h.event(Event::PointerMoved(at));
    h.event(button(at, true, modifiers));
    h.run();
}

fn move_to(h: &mut Harness<'_, Host>, at: Pos2) {
    h.event(Event::PointerMoved(at));
    h.run();
}

fn release(h: &mut Harness<'_, Host>, at: Pos2) {
    let modifiers = h.ctx.input(|i| i.modifiers);
    h.event(button(at, false, modifiers));
    h.event(Event::ModifiersChanged(Modifiers::NONE));
    h.run();
}

/// A drag over several frames: press, two moves, release.
fn drag(h: &mut Harness<'_, Host>, from: Pos2, to: Pos2, modifiers: Modifiers) {
    press(h, from, modifiers);
    move_to(h, from.lerp(to, 0.5));
    move_to(h, to);
    release(h, to);
}

fn click(h: &mut Harness<'_, Host>, at: Pos2, modifiers: Modifiers) {
    press(h, at, modifiers);
    release(h, at);
}

fn key(h: &mut Harness<'_, Host>, modifiers: Modifiers, key: Key) {
    h.key_press_modifiers(modifiers, key);
    h.run();
}

fn kind<T>(e: &EditEvent<T>) -> &'static str {
    match e {
        EditEvent::Begin { .. } => "begin",
        EditEvent::Update { .. } => "update",
        EditEvent::Commit { .. } => "commit",
        EditEvent::Cancel { .. } => "cancel",
        EditEvent::Set { .. } => "set",
    }
}

fn kinds(h: &Harness<'_, Host>) -> Vec<&'static str> {
    h.state().events.iter().map(kind).collect()
}

/// The value of the last event that carries one.
fn last_edit(h: &Harness<'_, Host>) -> Option<PathEdit> {
    h.state().events.iter().rev().find_map(|e| e.value().cloned())
}

fn selection(h: &Harness<'_, Host>) -> PathSelection {
    h.state().paths.selection.clone()
}

fn sel(path: u64, anchors: &[(usize, usize)]) -> PathSelection {
    PathSelection { path: Some(path), anchors: anchors.iter().map(|&(s, k)| KnotRef::new(s, k)).collect(), segment: None }
}

fn anchor(h: &Harness<'_, Host>, path: usize, sub: usize, knot: usize) -> Point {
    h.state().paths.paths[path].subpaths[sub].knots[knot].anchor
}

fn near(a: Vec2, b: Vec2) -> bool {
    (a - b).hypot() < 1e-3
}

fn moved_by(edit: Option<PathEdit>) -> Option<(Vec<KnotRef>, Vec2)> {
    match edit? {
        PathEdit::MoveAnchors { anchors, by, .. } => Some((anchors, by)),
        _ => None,
    }
}

#[test]
fn a_shown_handle_beats_an_anchor_which_beats_a_segment_at_equal_distance() {
    // A smooth knot with short handles, selected so they show.
    let knots = vec![
        Knot::corner(Point::new(100.0, 150.0)),
        Knot { anchor: Point::new(200.0, 150.0), in_ctrl: Point::new(194.0, 150.0), out_ctrl: Point::new(206.0, 150.0), smooth: true },
        Knot::corner(Point::new(300.0, 150.0)),
    ];
    let mut h = harness(vec![EditPath { key: 7, subpaths: vec![EditSubpath { knots, closed: false }], color: None }]);
    h.state_mut().paths.selection = sel(7, &[(0, 1)]);
    h.run();
    // 5 points from both the out handle's end and the anchor.
    let at = screen(&h, Point::new(203.0, 154.0));
    drag(&mut h, at, at + vec2(0.0, 20.0), Modifiers::NONE);
    assert!(matches!(last_edit(&h), Some(PathEdit::MoveHandle { side: HandleSide::Out, .. })), "{:?}", last_edit(&h));

    // On path 1, 3 points above anchor 2 and above the straight segment through it.
    let mut h = two_paths();
    let at = screen(&h, Point::new(160.0, 57.0));
    drag(&mut h, at, at + vec2(0.0, 20.0), Modifiers::NONE);
    assert_eq!(moved_by(last_edit(&h)), Some((vec![KnotRef::new(0, 1)], Vec2::new(0.0, 20.0))), "the anchor alone moves");
    assert_eq!(selection(&h), sel(1, &[(0, 1)]));
}

#[test]
fn a_segment_takes_the_pointer_within_6_points_in_a_rotated_flipped_view() {
    for (offset, takes) in [(5.9, true), (-5.9, true), (6.1, false)] {
        let mut h = harness(vec![path_a()]);
        (h.state_mut().rotation, h.state_mut().flip) = (30.0, true);
        h.run();
        let view = h.state().view.expect("drawn");
        // The curve from anchor 3 to anchor 4 as the view draws it, at t = 0.5.
        let c = h.state().paths.paths[0].subpaths[0].segment(2).expect("segment");
        let on = view.to_screen(c.eval(0.5));
        let tangent = view.map_vec(c.deriv().eval(0.5).to_vec2()).normalized();
        let at = on + vec2(-tangent.y, tangent.x) * offset;
        drag(&mut h, at, at + vec2(15.0, 0.0), Modifiers::NONE);
        let bent = matches!(last_edit(&h), Some(PathEdit::BendSegment { segment, .. }) if segment == KnotRef::new(0, 2));
        assert_eq!(bent, takes, "offset {offset}");
    }
}

#[test]
fn shift_toggles_an_anchor_and_alt_selects_its_subpath() {
    let mut h = two_paths();
    let (a2, a4) = (screen(&h, anchor(&h, 0, 0, 1)), screen(&h, anchor(&h, 0, 0, 3)));
    click(&mut h, a2, Modifiers::NONE);
    assert_eq!(selection(&h), sel(1, &[(0, 1)]));
    click(&mut h, a4, Modifiers::SHIFT);
    assert_eq!(selection(&h), sel(1, &[(0, 1), (0, 3)]));
    // A press that deselects drags nothing.
    drag(&mut h, a2, a2 + vec2(30.0, 0.0), Modifiers::SHIFT);
    assert_eq!(selection(&h), sel(1, &[(0, 3)]));
    assert!(h.state().events.is_empty(), "{:?}", kinds(&h));
    let line = screen(&h, anchor(&h, 1, 1, 0));
    click(&mut h, line, Modifiers::ALT);
    assert_eq!(selection(&h), sel(2, &[(1, 0), (1, 1)]), "⌥ selects the anchor's subpath");
}

#[test]
fn a_drag_moves_the_selected_anchors_and_shift_keeps_it_at_45_degrees() {
    let mut h = two_paths();
    let (a1, a2) = (screen(&h, anchor(&h, 0, 0, 0)), screen(&h, anchor(&h, 0, 0, 1)));
    click(&mut h, a1, Modifiers::NONE);
    click(&mut h, a2, Modifiers::SHIFT);
    // ⇧ held after the press constrains the move; held at the press it would toggle.
    press(&mut h, a2, Modifiers::NONE);
    move_to(&mut h, a2 + vec2(10.0, 0.0));
    h.event(Event::ModifiersChanged(Modifiers::SHIFT));
    move_to(&mut h, a2 + vec2(30.0, 26.0));
    release(&mut h, a2 + vec2(30.0, 26.0));
    assert_eq!(kinds(&h).first(), Some(&"begin"));
    assert_eq!(kinds(&h).last(), Some(&"commit"));
    let (anchors, by) = moved_by(last_edit(&h)).expect("a move");
    assert_eq!(anchors, [KnotRef::new(0, 0), KnotRef::new(0, 1)]);
    assert!(near(by, Vec2::new(28.0, 28.0)), "{by:?}");
    assert!(near(anchor(&h, 0, 0, 0) - Point::new(60.0, 60.0), by), "the host applied it");
}

#[test]
fn a_straight_segment_moves_its_anchors_and_a_curved_one_bends() {
    let mut h = two_paths();
    let mid = screen(&h, Point::new(110.0, 60.0));
    drag(&mut h, mid, mid + vec2(0.0, 20.0), Modifiers::NONE);
    assert_eq!(moved_by(last_edit(&h)), Some((vec![KnotRef::new(0, 0), KnotRef::new(0, 1)], Vec2::new(0.0, 20.0))));
    assert_eq!(selection(&h).segment, Some(KnotRef::new(0, 0)), "a press on a segment selects it");

    let mut h = two_paths();
    let c = h.state().paths.paths[0].subpaths[0].segment(1).expect("segment");
    let at = screen(&h, c.eval(0.5));
    drag(&mut h, at, at + vec2(0.0, 12.0), Modifiers::NONE);
    match last_edit(&h) {
        Some(PathEdit::BendSegment { path: 1, segment, t, by }) => {
            assert_eq!(segment, KnotRef::new(0, 1));
            assert!((t - 0.5).abs() < 0.01, "t {t}");
            assert!(near(by, Vec2::new(0.0, 12.0)), "{by:?}");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_release_within_1_point_is_a_click_with_no_event() {
    let mut h = two_paths();
    let a2 = screen(&h, anchor(&h, 0, 0, 1));
    drag(&mut h, a2, a2 + vec2(0.9, 0.0), Modifiers::NONE);
    assert!(h.state().events.is_empty(), "{:?}", kinds(&h));
    assert_eq!(selection(&h), sel(1, &[(0, 1)]), "the click selects");
    drag(&mut h, a2, a2 + vec2(1.1, 0.0), Modifiers::NONE);
    assert_eq!(kinds(&h).last(), Some(&"commit"));
}

#[test]
fn esc_cancels_a_drag_and_the_rest_of_the_press_does_nothing() {
    let mut h = two_paths();
    let before = h.state().paths.paths.clone();
    let a2 = screen(&h, anchor(&h, 0, 0, 1));
    press(&mut h, a2, Modifiers::NONE);
    move_to(&mut h, a2 + vec2(20.0, 0.0));
    move_to(&mut h, a2 + vec2(30.0, 0.0));
    key(&mut h, Modifiers::NONE, Key::Escape);
    assert!(matches!(h.state().events.last(), Some(EditEvent::Cancel { reason: CancelReason::Escape, .. })), "{:?}", kinds(&h));
    let n = h.state().events.len();
    move_to(&mut h, a2 + vec2(60.0, 0.0));
    assert!(h.state().last.as_ref().is_some_and(|r| r.claimed), "the press stays the overlay's");
    release(&mut h, a2 + vec2(60.0, 0.0));
    assert_eq!(h.state().events.len(), n, "nothing after the cancel");
    assert_eq!(h.state().paths.paths, before);
}

#[test]
fn a_quick_drag_in_one_frame_moves_from_the_press() {
    let mut h = two_paths();
    let a2 = screen(&h, anchor(&h, 0, 0, 1));
    let to = a2 + vec2(25.0, 5.0);
    h.input_mut().events.extend([
        Event::PointerMoved(a2),
        button(a2, true, Modifiers::NONE),
        Event::PointerMoved(a2 + vec2(10.0, 0.0)),
        Event::PointerMoved(to),
        button(to, false, Modifiers::NONE),
    ]);
    h.step();
    assert_eq!(kinds(&h), ["set"]);
    assert_eq!(moved_by(last_edit(&h)), Some((vec![KnotRef::new(0, 1)], Vec2::new(25.0, 5.0))));
}

#[test]
fn an_anchor_beats_a_guide_under_it() {
    let mut h = harness(vec![path_a()]);
    h.state_mut().guide = Some(160.0);
    h.run();
    let a2 = screen(&h, anchor(&h, 0, 0, 1));
    drag(&mut h, a2, a2 + vec2(20.0, 0.0), Modifiers::NONE);
    assert_eq!(h.state().guide_events, 0, "the guide stays");
    assert_eq!(moved_by(last_edit(&h)), Some((vec![KnotRef::new(0, 1)], Vec2::new(20.0, 0.0))));
}

#[test]
fn convert_point_makes_a_corner_on_a_click_and_smooth_handles_on_a_drag() {
    let mut h = two_paths();
    h.state_mut().tool = BezierTool::Convert;
    h.run();
    let a3 = screen(&h, anchor(&h, 0, 0, 2));
    click(&mut h, a3, Modifiers::NONE);
    assert_eq!(kinds(&h), ["set"]);
    assert_eq!(last_edit(&h), Some(PathEdit::Convert { path: 1, knot: KnotRef::new(0, 2), out: None }));
    let a2 = screen(&h, anchor(&h, 0, 0, 1));
    drag(&mut h, a2, a2 + vec2(30.0, 10.0), Modifiers::NONE);
    assert_eq!(last_edit(&h), Some(PathEdit::Convert { path: 1, knot: KnotRef::new(0, 1), out: Some(Point::new(190.0, 70.0)) }));
    assert_eq!(h.state().paths.paths[0].subpaths[0].knots[1], Knot::smooth(Point::new(160.0, 60.0), Point::new(190.0, 70.0)));
    // Segments and empty canvas are the tool's.
    let n = h.state().events.len();
    let mid = screen(&h, Point::new(110.0, 60.0));
    press(&mut h, mid, Modifiers::NONE);
    assert!(h.state().last.as_ref().is_some_and(|r| !r.claimed));
    release(&mut h, mid + vec2(0.0, 20.0));
    assert_eq!(h.state().events.len(), n);
}

#[test]
fn a_marquee_selects_on_the_frontmost_path_and_shift_adds() {
    let mut h = two_paths();
    // Round anchor 1 of path 1 and the top of path 2's square.
    let (from, to) = (screen(&h, Point::new(40.0, 40.0)), screen(&h, Point::new(150.0, 210.0)));
    drag(&mut h, from, to, Modifiers::NONE);
    assert_eq!(selection(&h), sel(2, &[(0, 0), (0, 1)]));
    assert!(h.state().events.is_empty(), "a marquee is no path edit");
    let (from, to) = (screen(&h, Point::new(130.0, 250.0)), screen(&h, Point::new(150.0, 275.0)));
    drag(&mut h, from, to, Modifiers::SHIFT);
    assert_eq!(selection(&h), sel(2, &[(0, 0), (0, 1), (0, 2)]));
    let empty = screen(&h, Point::new(380.0, 280.0));
    click(&mut h, empty, Modifiers::NONE);
    assert_eq!(selection(&h), PathSelection::default(), "a click on empty canvas deselects");
}

/// The label of the overlay's active descendant.
fn active(h: &Harness<'_, Host>) -> Option<String> {
    h.get_by_label("Paths").accesskit_node().active_descendant().and_then(|n| n.label())
}

#[test]
fn page_keys_pick_each_anchor_then_its_in_and_out_handles() {
    let mut h = harness(vec![path_a()]);
    h.run();
    key(&mut h, Modifiers::NONE, Key::Tab);
    assert_eq!(active(&h).as_deref(), Some("Anchor 1 of 4"));
    assert_eq!(selection(&h), sel(1, &[(0, 0)]), "picking an anchor selects it alone");
    let mut seen = Vec::new();
    for _ in 0..5 {
        key(&mut h, Modifiers::NONE, Key::PageDown);
        seen.push(active(&h).unwrap_or_default());
    }
    assert_eq!(seen, ["Anchor 2 of 4", "Anchor 3 of 4", "In handle", "Out handle", "Anchor 4 of 4"]);
    assert_eq!(selection(&h), sel(1, &[(0, 3)]));
    key(&mut h, Modifiers::NONE, Key::PageUp);
    assert_eq!(active(&h).as_deref(), Some("Anchor 3 of 4"));
}

#[test]
fn shift_page_keys_add_the_next_and_previous_anchor_and_command_a_selects_all() {
    let mut h = harness(vec![path_a()]);
    h.run();
    key(&mut h, Modifiers::NONE, Key::Tab);
    key(&mut h, Modifiers::SHIFT, Key::PageDown);
    key(&mut h, Modifiers::SHIFT, Key::PageDown);
    assert_eq!(selection(&h), sel(1, &[(0, 0), (0, 1), (0, 2)]));
    assert_eq!(active(&h).as_deref(), Some("Anchor 3 of 4"));
    key(&mut h, Modifiers::NONE, Key::Escape);
    assert_eq!(selection(&h), PathSelection::default(), "Esc deselects");
    key(&mut h, Modifiers::COMMAND, Key::A);
    assert_eq!(selection(&h), sel(1, &[(0, 0), (0, 1), (0, 2), (0, 3)]));
}

#[test]
fn arrows_move_the_selected_anchors_or_the_picked_handle_and_alt_moves_it_alone() {
    let mut h = harness(vec![path_a()]);
    h.run();
    key(&mut h, Modifiers::NONE, Key::Tab);
    key(&mut h, Modifiers::NONE, Key::ArrowRight);
    key(&mut h, Modifiers::SHIFT, Key::ArrowDown);
    assert_eq!(kinds(&h), ["set", "set"]);
    assert_eq!(anchor(&h, 0, 0, 0), Point::new(61.0, 70.0));
    // Anchor 3's in handle.
    for _ in 0..3 {
        key(&mut h, Modifiers::NONE, Key::PageDown);
    }
    assert_eq!(active(&h).as_deref(), Some("In handle"));
    key(&mut h, Modifiers::ALT, Key::ArrowLeft);
    let moved = |to: Point, alone: bool| Some(PathEdit::MoveHandle { path: 1, knot: KnotRef::new(0, 2), side: HandleSide::In, to, alone });
    assert_eq!(last_edit(&h), moved(Point::new(229.0, 20.0), true));
    key(&mut h, Modifiers::NONE, Key::ArrowUp);
    assert_eq!(last_edit(&h), moved(Point::new(229.0, 19.0), false));
}

#[test]
fn insert_and_delete_report_the_anchor_to_add_and_the_ones_to_remove() {
    let mut h = harness(vec![path_a()]);
    h.run();
    key(&mut h, Modifiers::NONE, Key::Tab);
    key(&mut h, Modifiers::NONE, Key::Insert);
    assert_eq!(h.state().inserts, [(1, KnotRef::new(0, 0), 0.5)]);
    assert_eq!(anchor(&h, 0, 0, 1), Point::new(110.0, 60.0), "the host split the segment");
    key(&mut h, Modifiers::NONE, Key::Delete);
    assert_eq!(h.state().deletes, [(1, vec![KnotRef::new(0, 1)])]);
    // The last anchor of an open subpath has no segment leaving it.
    key(&mut h, Modifiers::NONE, Key::End);
    key(&mut h, Modifiers::NONE, Key::Insert);
    assert_eq!(h.state().inserts.len(), 1);
}

/// The shapes of the last frame stroked in `color`.
fn painted(h: &Harness<'_, Host>, color: Color32) -> usize {
    fn count(shape: &Shape, color: Color32) -> usize {
        match shape {
            Shape::Vec(shapes) => shapes.iter().map(|s| count(s, color)).sum(),
            Shape::Rect(rect) => usize::from(rect.stroke.color == color && rect.stroke.width > 0.0),
            _ => 0,
        }
    }
    h.output().shapes.iter().map(|c| count(&c.shape, color)).sum()
}

#[test]
fn a_click_gives_the_paths_focus_and_the_ring_shows_after_keyboard_input_only() {
    let mut h = two_paths();
    let focus = FocusRing::style(&current(&h.ctx)).stroke.color;
    let a2 = screen(&h, anchor(&h, 0, 0, 1));
    click(&mut h, a2, Modifiers::NONE);
    // AccessKit reports the focused group's active descendant.
    let focused = h.query_by(|n| n.is_focused()).and_then(|n| n.accesskit_node().label());
    assert_eq!(focused.as_deref(), Some("Anchor 2 of 4"), "the release on an anchor focuses the paths");
    assert_eq!(painted(&h, focus), 0, "no ring after the pointer");
    key(&mut h, Modifiers::NONE, Key::PageDown);
    assert_eq!(active(&h).as_deref(), Some("Anchor 3 of 4"));
    assert_eq!(painted(&h, focus), 1);
}

fn descendants(node: egui_kittest::kittest::AccessKitNode<'_>) -> Vec<egui_kittest::kittest::AccessKitNode<'_>> {
    node.children().flat_map(|child| std::iter::once(child).chain(descendants(child))).collect()
}

fn objects(h: &Harness<'_, Host>) -> Vec<(String, String, Option<bool>)> {
    descendants(h.get_by_label("Paths").accesskit_node())
        .into_iter()
        .filter(|n| n.role() == Role::GraphicsObject)
        .map(|n| (n.label().unwrap_or_default(), n.value().unwrap_or_default(), n.data().is_selected()))
        .collect()
}

#[test]
fn anchors_and_shown_handles_are_graphics_objects_until_the_anchor_limit() {
    let mut h = harness(vec![path_a()]);
    h.state_mut().paths.selection = sel(1, &[(0, 2)]);
    h.run();
    let group = h.get_by_label("Paths");
    assert_eq!(group.accesskit_node().role(), Role::Group);
    let o = |name: &str, value: &str, selected: bool| (name.to_owned(), value.to_owned(), Some(selected));
    assert_eq!(
        objects(&h),
        [
            o("Anchor 1 of 4", "60.0 px, 60.0 px", false),
            o("Anchor 2 of 4", "160.0 px, 60.0 px", false),
            o("Anchor 3 of 4", "260.0 px, 60.0 px", true),
            o("In handle", "230.0 px, 20.0 px", false),
            o("Out handle", "290.0 px, 100.0 px", false),
            o("Anchor 4 of 4", "340.0 px, 160.0 px", false),
        ]
    );
    assert_eq!(active(&h).as_deref(), Some("Anchor 3 of 4"), "the first selected anchor stands for the pick");

    let knots = (0..=MAX_ANCHORS).map(|i| Knot::corner(Point::new(i as f64 * 0.001, 100.0))).collect();
    let mut h = harness(vec![EditPath { key: 9, subpaths: vec![EditSubpath { knots, closed: false }], color: None }]);
    h.run();
    assert_eq!(h.get_by_label("Paths").accesskit_node().role(), Role::Group);
    assert!(objects(&h).is_empty(), "over the limit only the group");
}

/// A press on an anchor during a text field's typing run begins after the run's Commit and still
/// drags from the press.
#[test]
fn a_press_during_a_typing_run_begins_after_the_runs_commit() {
    let mut h = two_paths();
    h.state_mut().field.on = true;
    h.step();
    let at = screen(&h, Point::new(60.0, 60.0));
    h.event(Event::PointerMoved(at));
    h.step();
    h.event(button(at, true, Modifiers::NONE));
    h.step();
    assert!(h.state().events.is_empty(), "the press waits: {:?}", h.state().events);
    for i in 1..=6 {
        h.event(Event::PointerMoved(at + vec2(5.0 * i as f32, 0.0)));
        h.step();
    }
    h.event(button(at + vec2(30.0, 0.0), false, Modifiers::NONE));
    h.step();
    h.step();
    let events = &h.state().events;
    assert!(matches!(events.first(), Some(EditEvent::Begin { .. })), "{events:?}");
    assert!(matches!(events.last(), Some(EditEvent::Commit { changed: true, .. })), "{events:?}");
    assert!(h.state().field.commit_came_first());
}

#[test]
fn modifiers_override_replaces_the_input_modifiers_in_the_overlay() {
    let mut h = two_paths();
    let (a1, a2) = (screen(&h, anchor(&h, 0, 0, 0)), screen(&h, anchor(&h, 0, 0, 1)));
    click(&mut h, a1, Modifiers::NONE);
    click(&mut h, a2, Modifiers::SHIFT);
    h.state_mut().modifiers = Some(Modifiers::NONE);
    // The input holds ⇧ after the press, which would keep the move at 45°; the override says none.
    press(&mut h, a2, Modifiers::NONE);
    move_to(&mut h, a2 + vec2(10.0, 0.0));
    h.event(Event::ModifiersChanged(Modifiers::SHIFT));
    move_to(&mut h, a2 + vec2(30.0, 26.0));
    release(&mut h, a2 + vec2(30.0, 26.0));
    let (_, by) = moved_by(last_edit(&h)).expect("a move");
    assert!(near(by, Vec2::new(30.0, 26.0)), "{by:?}");
}

/// The filled squares of the last frame, by width.
fn square_widths(h: &Harness<'_, Host>) -> Vec<f32> {
    fn collect(shape: &Shape, out: &mut Vec<f32>) {
        match shape {
            Shape::Vec(shapes) => shapes.iter().for_each(|s| collect(s, out)),
            Shape::Rect(rect) => out.push(rect.rect.width()),
            _ => {}
        }
    }
    let mut out = Vec::new();
    h.output().shapes.iter().for_each(|c| collect(&c.shape, &mut out));
    out
}

/// The lines of the last frame stroked `width` wide in `color`.
fn lines(h: &Harness<'_, Host>, width: f32, color: Color32) -> usize {
    fn count(shape: &Shape, width: f32, color: Color32) -> usize {
        match shape {
            Shape::Vec(shapes) => shapes.iter().map(|s| count(s, width, color)).sum(),
            Shape::Path(path) => usize::from((path.stroke.width - width).abs() < 1e-3 && path.stroke.color == egui::epaint::ColorMode::Solid(color)),
            _ => 0,
        }
    }
    h.output().shapes.iter().map(|c| count(&c.shape, width, color)).sum()
}

#[test]
fn highlight_draws_the_hover_look_on_the_hit_only() {
    let style = BezierOverlay::style(&current(&two_paths().ctx));
    let enlarged = style.highlight_anchor + 2.0 * style.anchor.halo.width;
    let ring = style.selected_segment + 2.0 * style.highlight_halo;

    let mut h = two_paths();
    assert!(!square_widths(&h).contains(&enlarged) && lines(&h, ring, style.halo) == 0, "nothing highlighted");

    let point = BezierHit::Anchor { path: 1, knot: KnotRef::new(0, 1) };
    h.state_mut().highlight = Some(point);
    h.run();
    let big = square_widths(&h).iter().filter(|w| (**w - enlarged).abs() < 1e-3).count();
    assert_eq!(big, 2, "one enlarged anchor: its body and its outline");
    assert_eq!(lines(&h, ring, style.halo), 0, "no segment ring for a point");

    let segment = BezierHit::Segment { path: 1, segment: KnotRef::new(0, 1), t: 0.5 };
    h.state_mut().highlight = Some(segment);
    h.run();
    assert_eq!(lines(&h, ring, style.halo), 1, "the ring's halo under the segment");
    assert_eq!(lines(&h, style.selected_segment, style.color), 1, "the ring");
    assert!(!square_widths(&h).contains(&enlarged), "no enlarged anchor for a segment");
}
