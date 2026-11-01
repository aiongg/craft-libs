//! Tests of the anchor tools (task PE-B1) and the Curvature tool (task PE-C1).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{Event, Modifiers, PointerButton, Pos2, Sense, Shape, vec2};
use egui_kittest::Harness;
use kurbo::Point;

use super::{BezierNames, BezierOverlay, BezierTool, EditPath, EditSubpath, Knot, KnotRef, PathSelection};
use craft_ui::canvas::{CanvasView, RulerUnits};
use craft_ui::localized::LocaleNumbers;
use craft_ui::theme::{Base, current, install};

/// The key of the open path the tests pass last, as the host passes the Pen draft.
const DRAFT: u64 = 9;

struct Host {
    paths: Vec<EditPath>,
    selection: PathSelection,
    tool: BezierTool,
    view: Option<CanvasView>,
    inserts: Vec<(u64, KnotRef, f64)>,
    deletes: Vec<(u64, Vec<KnotRef>)>,
    selects: usize,
    claimed: bool,
}

fn open(key: u64, y: f64) -> EditPath {
    let knots = [60.0, 160.0, 260.0].map(|x| Knot::corner(Point::new(x, y))).to_vec();
    EditPath { key, subpaths: vec![EditSubpath { knots, closed: false }], color: None }
}

fn square(key: u64) -> EditPath {
    let knots = [(60.0, 200.0), (140.0, 200.0), (140.0, 260.0), (60.0, 260.0)].map(|(x, y)| Knot::corner(Point::new(x, y))).to_vec();
    EditPath { key, subpaths: vec![EditSubpath { knots, closed: true }], color: None }
}

/// An open path (key 1) at y 60, a closed square (key 2) and a draft (key 9) at y 150, in that
/// order; the host applies reported inserts and deletes to the path of the reported key.
fn harness(tool: BezierTool) -> Harness<'static, Host> {
    let host = Host {
        paths: vec![open(1, 60.0), square(2), open(DRAFT, 150.0)],
        selection: PathSelection::default(),
        tool,
        view: None,
        inserts: Vec::new(),
        deletes: Vec::new(),
        selects: 0,
        claimed: false,
    };
    let mut h = Harness::<Host>::builder().with_size([480.0, 400.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            let (rect, canvas) = ui.allocate_exact_size(vec2(400.0, 300.0), Sense::click_and_drag() - Sense::FOCUSABLE);
            let view = CanvasView::new(rect, Point::new(200.0, 150.0), 1.0);
            host.view = Some(view);
            let px = RulerUnits::pixels(LocaleNumbers::from_tag("en"));
            let names = BezierNames::new("Paths", "In handle", "Out handle", "Selection", |i, n| format!("Anchor {i} of {n}"));
            let response = BezierOverlay::new("paths", &host.paths, &host.selection, &px, &px, names).tool(host.tool).show(ui, &canvas, &view);
            host.claimed |= response.claimed;
            host.selects += usize::from(response.select.is_some());
            if let Some((key, segment, t)) = response.insert {
                host.inserts.push((key, segment, t));
                if let Some(path) = host.paths.iter_mut().find(|p| p.key == key) {
                    let _ = path.subpaths[segment.subpath].insert_at(segment.knot, t);
                }
            }
            if let Some((key, knots)) = response.delete {
                host.deletes.push((key, knots.clone()));
                if let Some(path) = host.paths.iter_mut().find(|p| p.key == key) {
                    for k in knots {
                        path.subpaths[k.subpath].remove_knot(k.knot);
                    }
                }
            }
        },
        host,
    );
    h.run();
    h
}

fn screen(h: &Harness<'_, Host>, x: f64, y: f64) -> Pos2 {
    let Some(view) = h.state().view else { panic!("drawn") };
    view.to_screen(Point::new(x, y))
}

/// A click at document point (x, y); returns whether the overlay claimed the press.
fn click(h: &mut Harness<'_, Host>, x: f64, y: f64, modifiers: Modifiers) -> bool {
    let at = screen(h, x, y);
    h.state_mut().claimed = false;
    h.event(Event::ModifiersChanged(modifiers));
    h.event(Event::PointerMoved(at));
    h.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: true, modifiers });
    h.run();
    let claimed = h.state().claimed;
    h.event(Event::PointerButton { pos: at, button: PointerButton::Primary, pressed: false, modifiers });
    h.event(Event::ModifiersChanged(Modifiers::NONE));
    h.run();
    claimed
}

fn reports(h: &Harness<'_, Host>) -> (usize, usize) {
    (h.state().inserts.len(), h.state().deletes.len())
}

// PE-B1. Add Anchor: a click on a segment reports `insert` at the hit's t; a click on an anchor
// is claimed and reports nothing; with ⌥ the click on an anchor reports `delete`.
#[test]
fn add_anchor_tool_reports_insert_on_a_segment_click_and_alt_swaps() {
    let mut h = harness(BezierTool::AddAnchor);
    assert!(click(&mut h, 110.0, 60.0, Modifiers::NONE), "a segment is claimed");
    let [(path, segment, t)] = h.state().inserts[..] else { panic!("one insert: {:?}", h.state().inserts) };
    assert_eq!((path, segment), (1, KnotRef::new(0, 0)));
    assert!(t > 0.0 && t < 1.0);
    assert_eq!(h.state().selects, 0, "no selection change");

    assert!(click(&mut h, 160.0, 60.0, Modifiers::NONE), "an anchor is claimed");
    assert!(click(&mut h, 60.0, 60.0, Modifiers::NONE));
    assert_eq!(reports(&h), (1, 0), "nothing reported for an anchor");

    click(&mut h, 60.0, 60.0, Modifiers::ALT);
    assert_eq!(h.state().deletes, vec![(1, vec![KnotRef::new(0, 0)])]);
}

// PE-B1. Delete Anchor: a click on an anchor reports `delete` of that anchor; ⌥ on a segment
// reports `insert`.
#[test]
fn delete_anchor_tool_reports_delete_on_an_anchor_click() {
    let mut h = harness(BezierTool::DeleteAnchor);
    assert!(click(&mut h, 260.0, 60.0, Modifiers::NONE));
    assert_eq!(h.state().deletes, vec![(1, vec![KnotRef::new(0, 2)])]);
    assert!(click(&mut h, 110.0, 60.0, Modifiers::NONE), "a segment is claimed");
    assert_eq!(reports(&h), (0, 1), "nothing reported for a segment");
    click(&mut h, 110.0, 60.0, Modifiers::ALT);
    assert!(matches!(h.state().inserts[..], [(1, segment, _)] if segment == KnotRef::new(0, 0)), "{:?}", h.state().inserts);
}

// PE-B1. Auto Add/Delete: the first and last anchors of an open subpath and empty canvas are not
// claimed (the pen draft takes them); a closed subpath's every anchor is.
#[test]
fn auto_add_delete_leaves_open_ends_to_the_pen() {
    let mut h = harness(BezierTool::AutoAddDelete);
    assert!(!click(&mut h, 60.0, 60.0, Modifiers::NONE), "the first anchor closes the draft");
    assert!(!click(&mut h, 260.0, 60.0, Modifiers::NONE), "the last anchor retracts");
    assert!(!click(&mut h, 360.0, 20.0, Modifiers::NONE), "empty canvas starts a path");
    assert_eq!(reports(&h), (0, 0));

    assert!(click(&mut h, 160.0, 60.0, Modifiers::NONE), "an inner anchor");
    assert_eq!(h.state().deletes, vec![(1, vec![KnotRef::new(0, 1)])]);
    assert!(click(&mut h, 60.0, 200.0, Modifiers::NONE), "a closed subpath's first anchor");
    assert_eq!(h.state().deletes.len(), 2);
    assert!(click(&mut h, 140.0, 230.0, Modifiers::NONE), "a segment");
    assert_eq!(h.state().inserts.len(), 1);
}

// PE-B1. With an open pen draft passed as one more `EditPath` (its own key, last), Add on the
// draft's segment and Delete on its anchor report `insert` and `delete` with the draft's key, and
// applying them with `EditSubpath::{insert_at, remove_knot}` leaves the draft open from its last
// anchor (PhotoCraft `anchor_tools.rs:116-128`).
#[test]
fn anchor_tools_edit_the_pen_draft_passed_as_a_path() {
    let mut h = harness(BezierTool::AddAnchor);
    click(&mut h, 110.0, 150.0, Modifiers::NONE);
    assert!(matches!(h.state().inserts[..], [(DRAFT, segment, _)] if segment == KnotRef::new(0, 0)), "{:?}", h.state().inserts);
    let last = Point::new(260.0, 150.0);
    let draft = |h: &Harness<'_, Host>| h.state().paths[2].subpaths[0].clone();
    assert_eq!(draft(&h).knots.len(), 4);

    h.state_mut().tool = BezierTool::DeleteAnchor;
    h.run();
    click(&mut h, 160.0, 150.0, Modifiers::NONE);
    assert_eq!(h.state().deletes, vec![(DRAFT, vec![KnotRef::new(0, 2)])]);
    let after = draft(&h);
    assert_eq!(after.knots.len(), 3);
    assert!(!after.closed, "the draft stays open");
    assert!(after.knots.last().is_some_and(|k| k.anchor == last), "drawing goes on from the last anchor");
}

// PE-B1. Hovering a segment with Add draws the segment mark and a hollow anchor at t; hovering an
// anchor with Delete draws the enlarged anchor.
#[test]
fn anchor_tools_draw_the_hover_mark_of_what_a_press_takes() {
    fn widths(h: &Harness<'_, Host>) -> Vec<f32> {
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
    let hover = |h: &mut Harness<'_, Host>, x: f64, y: f64| {
        let at = screen(h, x, y);
        h.event(Event::PointerMoved(at));
        h.run();
    };
    let mut h = harness(BezierTool::AddAnchor);
    let style = BezierOverlay::style(&current(&h.ctx));
    let enlarged = style.highlight_anchor + 2.0 * style.anchor.halo.width;
    let count = |h: &Harness<'_, Host>, w: f32| widths(h).iter().filter(|x| (**x - w).abs() < 1e-3).count();
    let plain = count(&h, style.anchor.size);

    hover(&mut h, 110.0, 60.0);
    assert!(count(&h, style.anchor.size) > plain, "a hollow anchor at t");
    hover(&mut h, 160.0, 60.0);
    assert_eq!(count(&h, enlarged), 0, "Add edits nothing on an anchor");
    assert_eq!(count(&h, style.anchor.size), plain);

    h.state_mut().tool = BezierTool::DeleteAnchor;
    hover(&mut h, 160.0, 60.0);
    assert_eq!(count(&h, enlarged), 2, "the enlarged anchor: body and outline");
    hover(&mut h, 110.0, 60.0);
    assert_eq!(count(&h, enlarged), 0, "Delete edits nothing on a segment");
}

// PE-C1. A drag from an anchor reports MoveCurvature Begin past the click slop, Update, Commit;
// Esc cancels and the rest of the press does nothing.
#[test]
#[ignore = "skeleton"]
fn curvature_tool_drags_an_anchor_and_reflows() {
    todo!()
}

// PE-C1. A drag from a segment is one InsertCurvature gesture with t clamped to 0.01..=0.99.
#[test]
#[ignore = "skeleton"]
fn curvature_tool_insert_and_drag_is_one_gesture() {
    todo!()
}

// PE-C1. A click without drag on a segment reports one Set of InsertCurvature with `by` zero.
#[test]
#[ignore = "skeleton"]
fn curvature_tool_click_on_a_segment_inserts_only() {
    todo!()
}

// PE-C1. The second click of a double-click on an anchor reports a ToggleCurvature Set.
#[test]
#[ignore = "skeleton"]
fn curvature_tool_double_click_toggles_an_anchor() {
    todo!()
}

// PE-C1. A press on empty canvas is not claimed, so the pen draft shown after starts a path.
#[test]
#[ignore = "skeleton"]
fn curvature_tool_does_not_claim_empty_canvas() {
    todo!()
}

// PE-C1. With focus, the arrows report a MoveCurvature Set of the selected anchors (⇧ 10) and
// Enter a ToggleCurvature Set of the picked anchor.
#[test]
#[ignore = "skeleton"]
fn curvature_tool_keys_move_and_toggle() {
    todo!()
}
