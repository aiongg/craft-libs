//! The Navigator's geometry, its settings, its preview box and its zoom row.

use craft_behavior::kurbo::{Point, Rect as DocRect, Size};
use craft_behavior::{CanvasView, ZoomLadder};
use craft_ui::canvas::{
    Canvas, NavigationOptions, Navigator, NavigatorOptions, NavigatorSettings, NavigatorZoomRow, ProxyColor, preview_rect, proxy_quad,
    view_center_for, zoom_to_document_rect,
};
use craft_ui::contract::{Bind, Role, Show as _, ShowContainer as _};
use craft_ui::testing::focus::AccessActionKind;
use craft_ui::testing::pointer::{moved, press, release, wheel_lines};
use craft_ui::testing::story::EditKind;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Pos2, Rect, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

const DOC: Size = Size::new(2000.0, 1500.0);

/// The preview box and the inset round the fitted document, in points.
const FRAME: Rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(216.0, 166.0));
const INSET: f32 = 8.0;

/// The canvas of the views under test, in points.
const CANVAS: Rect = Rect::from_min_max(Pos2::new(50.0, 20.0), Pos2::new(850.0, 620.0));
const ZOOM_CANVAS: Rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(800.0, 600.0));

/// How far from the middle a click on the middle of the box may land, in document units.
const CENTRE_SLACK: f64 = 100.0;

/// The zoom the row starts from.
const START: f64 = 1.0;
/// The 66.67 % rung of the zoom ladder.
const TWO_THIRDS: f64 = 2.0 / 3.0;

const CENTRE: Point = Point::new(1000.0, 750.0);

/// The slack of a point comparison after f32 maths, in points.
const TOLERANCE: f32 = 1e-3;

/// The proxy of the upright view in the 216 by 166 box: 800 by 600 points at zoom 1 are 80 by 60 preview points round
/// the preview point of the centre, (108, 83).
const UPRIGHT: [Pos2; 4] = [Pos2::new(68.0, 53.0), Pos2::new(148.0, 53.0), Pos2::new(148.0, 113.0), Pos2::new(68.0, 113.0)];

fn preview() -> Rect {
    preview_rect(FRAME, DOC, INSET)
}

fn view() -> CanvasView {
    CanvasView::new(CANVAS, CENTRE, 1.0)
}

fn near(a: Pos2, b: Pos2) -> bool {
    (a - b).length() < TOLERANCE
}

#[trial]
fn the_proxy_follows_a_rotated_or_flipped_view() {
    assert_eq!(preview(), Rect::from_min_max(Pos2::new(8.0, 8.0), Pos2::new(208.0, 158.0)), "the document fits the box inside the inset");
    let upright = proxy_quad(&view(), DOC, preview());
    assert!(upright.iter().zip(UPRIGHT).all(|(a, b)| near(*a, b)), "{upright:?}");

    let wide = proxy_quad(&view().aspect(2.0), DOC, preview());
    assert!(near(wide[0], Pos2::new(88.0, 53.0)) && near(wide[1], Pos2::new(128.0, 53.0)), "a pixel aspect of 2 shows half the columns: {wide:?}");

    let turned = Rect::from_points(&proxy_quad(&view().rotation(90.0), DOC, preview()));
    assert!((turned.width() - 60.0).abs() < TOLERANCE && (turned.height() - 80.0).abs() < TOLERANCE, "a quarter turn swaps the extents: {turned:?}");

    let [a, b, c, d] = proxy_quad(&view().rotation(30.0), DOC, preview());
    assert!(((b - a).length() - 80.0).abs() < TOLERANCE && ((c - b).length() - 60.0).abs() < TOLERANCE, "the edges keep their length");
    assert!((b - a).dot(c - b).abs() < 1e-2 && near(a + (c - b), d), "the corners stay right angles");
    assert!((a.x - b.x).abs() > 1.0 && (a.y - b.y).abs() > 1.0, "a tilted view is not axis-aligned");

    let flipped = proxy_quad(&view().flip_x(true), DOC, preview());
    assert!(near(flipped[0], UPRIGHT[1]) && near(flipped[1], UPRIGHT[0]), "a mirrored view keeps the canvas's corner order: {flipped:?}");
}

#[trial]
fn a_click_in_the_preview_maps_to_the_document_point_and_centres_the_view() {
    {
        let middle = view_center_for(preview(), DOC, Pos2::new(108.0, 83.0));
        assert!((middle.x - CENTRE.x).abs() < 1e-2 && (middle.y - CENTRE.y).abs() < 1e-2, "{middle:?}");
        assert_eq!(
            view_center_for(preview(), DOC, Pos2::new(-40.0, 400.0)),
            Point::new(0.0, DOC.height),
            "a click off the picture stays on the document"
        );
    }
    {
        let view = Rc::new(RefCell::new(CanvasView::new(ZOOM_CANVAS, Point::new(0.0, 0.0), START)));
        let held = Rc::clone(&view);
        let mut fixture = Fixture::new(move |ui| {
            let mut view = held.borrow_mut();
            Navigator::new("Navigator", Bind::bound(&mut *view), DOC).show(ui);
        });
        fixture.settle();
        let preview = fixture.by_label("Navigator").expect("the navigator has a node");
        fixture.click(&preview);
        fixture.settle();
        let centre = view.borrow().center;
        assert!(
            (centre.x - CENTRE.x).abs() < CENTRE_SLACK && (centre.y - CENTRE.y).abs() < CENTRE_SLACK,
            "the middle of the box is the middle of the document: {centre:?}"
        );
    }
}

fn zoom_view(zoom: f64) -> CanvasView {
    CanvasView::new(ZOOM_CANVAS, CENTRE, zoom)
}

#[trial]
fn zooming_to_a_rectangle_fills_the_canvas_with_it() {
    // 800 by 600 points over 50 by 37.5 units: 16 points per unit.
    let mut view = zoom_view(1.0);
    assert!(zoom_to_document_rect(&mut view, DocRect::new(100.0, 100.0, 150.0, 137.5), DOC, 1.0));
    assert!((view.zoom - 16.0).abs() < 1e-9 && view.center == Point::new(125.0, 118.75), "{view:?}");

    let mut view = zoom_view(1.0);
    zoom_to_document_rect(&mut view, DocRect::new(0.0, 0.0, 400.0, 300.0), DOC, 2.0);
    assert!((view.zoom - 2.0).abs() < 1e-9, "the stored zoom is the device zoom over the pixels per point: {view:?}");

    let mut view = zoom_view(1.0).aspect(2.0);
    zoom_to_document_rect(&mut view, DocRect::new(0.0, 0.0, 400.0, 300.0), DOC, 1.0);
    assert!((view.zoom - 1.0).abs() < 1e-9, "a pixel aspect of 2 doubles the columns across the canvas: {view:?}");

    let mut view = zoom_view(1.0);
    assert!(zoom_to_document_rect(&mut view, DocRect::new(-100.0, -100.0, 400.0, 300.0), DOC, 1.0));
    assert!((view.zoom - 2.0).abs() < 1e-9 && view.center == Point::new(200.0, 150.0), "a rectangle past the document is clipped: {view:?}");

    let mut view = zoom_view(1.0);
    assert!(zoom_to_document_rect(&mut view, DocRect::new(100.0, 100.0, 101.0, 150.0), DOC, 1.0));
    assert!((view.zoom - 2.0).abs() < 1e-9 && view.center == Point::new(100.5, 125.0), "a sliver steps one rung in: {view:?}");

    for rect in [DocRect::new(f64::NAN, 0.0, 100.0, 100.0), DocRect::new(3000.0, 0.0, 3100.0, 100.0), DocRect::new(0.0, 0.0, f64::INFINITY, 1.0)] {
        let mut view = zoom_view(1.0);
        assert!(!zoom_to_document_rect(&mut view, rect, DOC, 1.0), "{rect:?}");
        assert_eq!(view, zoom_view(1.0), "{rect:?} changes nothing");
    }
}

#[trial]
fn the_navigator_settings_round_trip() {
    let red = toml::Value::try_from(NavigatorSettings::new(NavigatorOptions::new(ProxyColor::Red))).expect("settings serialise");
    assert_eq!(red.get("proxyColor").and_then(toml::Value::as_str), Some("Red"), "the options are flattened into the section");
    let custom = NavigatorSettings::new(NavigatorOptions::new(ProxyColor::Custom([1, 2, 3])));
    let written = toml::Value::try_from(custom).expect("settings serialise");
    assert_eq!(written.try_into::<NavigatorSettings>().expect("settings read back"), custom);
    assert_eq!(toml::from_str::<NavigatorSettings>("other = 1").expect("unknown fields are ignored"), NavigatorSettings::default());
}

/// Where the canvas beside the zoom row shows, below the row.
const ROW_CANVAS: Rect = Rect::from_min_max(Pos2::new(0.0, 100.0), Pos2::new(800.0, 700.0));

/// The view of the canvas beside the zoom row, at `zoom` about `CENTRE`.
fn row_view(zoom: f64) -> CanvasView {
    CanvasView::new(ROW_CANVAS, CENTRE, zoom)
}

/// A zoom row over `view` and the canvas that shows it below the row. The app copies the view the canvas reports
/// into `view`.
fn zoom_row(view: &Rc<RefCell<CanvasView>>, animated: bool) -> Fixture {
    let held = Rc::clone(view);
    Fixture::new(move |ui| {
        let mut options = NavigationOptions::default();
        options.animated_zoom = animated;
        NavigatorZoomRow::new(Bind::bound(&mut *held.borrow_mut()), DOC).show(ui);
        let shown = *held.borrow();
        let canvas = Canvas::new("Document").view(shown).document(DOC).navigation(options).show_at(ui, ROW_CANVAS, |_, _| {});
        if let Some(next) = canvas.output.view {
            *held.borrow_mut() = next;
        }
    })
}

#[trial]
fn the_percent_field_steps_the_zoom_ladder_and_shows_two_decimals() {
    let view = Rc::new(RefCell::new(row_view(TWO_THIRDS)));
    let mut fixture = zoom_row(&view, false);
    fixture.settle();
    let field = fixture.by_role(Role::SPIN_BUTTON).into_iter().next().expect("the row has a percent field");
    assert!(fixture.texts().iter().any(|text| text.text == "66.67"), "a ladder rung shows its two decimals");
    fixture.click(&field);
    fixture.settle();
    fixture.key(Key::ArrowUp, Modifiers::NONE);
    fixture.settle();
    let after = view.borrow().zoom;
    let next_rung = ZoomLadder::step(TWO_THIRDS, 1, DOC);
    assert!(next_rung > TWO_THIRDS && (after - next_rung).abs() < 1e-9, "up from 66.67 % goes to the next rung: {after}");
}

// l4e-navprev: the preview box's behaviour.

/// How far a trial drags, in points: past the click slop, and short enough to stay on the document.
const DRAG: f32 = 20.0;

/// Where a drag grips the proxy, from its middle in points: off the middle, so a jump to the pointer would show.
const GRIP: f32 = 10.0;

/// The share of the visible span an arrow key and an AccessKit scroll move the view by.
const ARROW_SHARE: f64 = 0.1;

/// The slack of a document point comparison after f32 maths, in document units.
const DOC_SLACK: f64 = 1e-3;

fn near_doc(a: Point, b: Point) -> bool {
    (a - b).hypot() < DOC_SLACK
}

/// A navigator shown in a fixture, with the bound view and the kinds of the edits it reported.
struct Shown {
    fixture: Fixture,
    view: Rc<RefCell<CanvasView>>,
    edits: Rc<RefCell<Vec<EditKind>>>,
}

impl Shown {
    fn new(start: CanvasView, animated: bool) -> Self {
        let view = Rc::new(RefCell::new(start));
        let edits = Rc::new(RefCell::new(Vec::new()));
        let (held, kinds) = (Rc::clone(&view), Rc::clone(&edits));
        let mut fixture = Fixture::new(move |ui| {
            let mut navigation = NavigationOptions::default();
            navigation.animated_zoom = animated;
            let mut view = held.borrow_mut();
            let out = Navigator::new("Navigator", Bind::bound(&mut *view), DOC).navigation(navigation).show(ui);
            kinds.borrow_mut().extend(out.view.event.as_ref().and_then(EditKind::of));
        });
        fixture.settle();
        Shown { fixture, view, edits }
    }

    /// The navigator focused by Tab.
    fn focused(start: CanvasView, animated: bool) -> Self {
        let mut shown = Shown::new(start, animated);
        shown.fixture.key(Key::Tab, Modifiers::NONE);
        shown
    }

    fn frame(&self) -> Rect {
        self.fixture.by_label("Navigator").expect("the navigator has a node").rect
    }

    fn view(&self) -> CanvasView {
        *self.view.borrow()
    }

    fn ends(&self) -> usize {
        self.edits.borrow().iter().filter(|kind| matches!(kind, EditKind::Commit | EditKind::Set)).count()
    }

    /// Press at `from` and move by `by` in four steps; the button stays down.
    fn press_and_move(&mut self, from: Pos2, by: Vec2) {
        self.fixture.frame(vec![moved(from)]);
        self.fixture.frame(vec![press(from)]);
        for step in 1..=4 {
            self.fixture.frame(vec![moved(from + by * (step as f32 / 4.0))]);
        }
    }

    fn drag(&mut self, from: Pos2, by: Vec2) {
        self.press_and_move(from, by);
        self.fixture.frame(vec![release(from + by)]);
    }
}

#[trial]
fn a_drag_of_the_proxy_moves_the_view_by_the_distance_and_escape_puts_it_back() {
    {
        let moved_by = |by: Vec2| {
            let mut shown = Shown::new(zoom_view(START), false);
            let from = shown.frame().center() + Vec2::new(GRIP, 0.0);
            shown.drag(from, by);
            assert_eq!(shown.ends(), 1, "the drag is one commit: {:?}", shown.edits.borrow());
            shown.view().center - CENTRE
        };
        let once = moved_by(Vec2::new(DRAG, 0.0));
        let twice = moved_by(Vec2::new(2.0 * DRAG, 0.0));
        assert!(once.x > 0.0 && once.y.abs() < 1e-6, "a drag to the right moves the view right: {once:?}");
        assert!((twice.x - 2.0 * once.x).abs() < 1e-3 * twice.x, "the view moves by the distance dragged, not to the pointer: {once:?}, {twice:?}");
    }
    {
        let mut shown = Shown::new(zoom_view(START), false);
        let from = shown.frame().center();
        shown.press_and_move(from, Vec2::new(DRAG, DRAG));
        shown.fixture.key(Key::Escape, Modifiers::NONE);
        shown.fixture.frame(vec![release(from + Vec2::new(DRAG, DRAG))]);
        assert!(shown.edits.borrow().contains(&EditKind::Update), "the drag ran: {:?}", shown.edits.borrow());
        assert_eq!(shown.view(), zoom_view(START), "Escape restores the view of the press");
        assert_eq!(shown.ends(), 0, "a cancelled drag commits nothing: {:?}", shown.edits.borrow());
    }
}

#[trial]
fn the_wheel_during_a_drag_changes_nothing_and_the_release_commits() {
    let dragged = |wheel: bool| {
        let mut shown = Shown::new(zoom_view(START), false);
        let (from, by) = (shown.frame().center(), Vec2::new(DRAG, 0.0));
        shown.press_and_move(from, by);
        if wheel {
            shown.fixture.frame(vec![wheel_lines(1.0)]);
        }
        shown.fixture.frame(vec![release(from + by)]);
        assert_eq!(shown.ends(), 1, "one commit: {:?}", shown.edits.borrow());
        shown.view()
    };
    assert_eq!(dragged(true), dragged(false), "the view is the drag's");
}

#[trial]
fn a_cmd_drag_zooms_the_bound_view_to_the_rectangle_with_or_without_animated_zoom() {
    for animated in [true, false] {
        let mut shown = Shown::new(zoom_view(START), animated);
        let frame = shown.frame();
        shown.fixture.hold(Modifiers::COMMAND);
        shown.drag(frame.lerp_inside(Vec2::new(0.3, 0.3)), frame.size() * 0.2);
        shown.fixture.hold(Modifiers::NONE);
        let bound = shown.view();
        assert!(bound.zoom > START, "the rectangle zooms the bound view in (animated: {animated}): {bound:?}");
    }
}

#[trial]
fn an_arrow_key_pans_by_a_share_of_the_span_and_end_shows_the_document_end() {
    let mut shown = Shown::focused(zoom_view(START), false);
    shown.fixture.key(Key::ArrowRight, Modifiers::NONE);
    let span = ZOOM_CANVAS.size();
    let step = f64::from(span.x) * ARROW_SHARE / START;
    assert!(near_doc(shown.view().center, Point::new(CENTRE.x + step, CENTRE.y)), "one tenth of the visible span: {:?}", shown.view());

    shown.fixture.key(Key::End, Modifiers::NONE);
    let end = Point::new(DOC.width - f64::from(span.x) / 2.0 / START, DOC.height - f64::from(span.y) / 2.0 / START);
    assert!(near_doc(shown.view().center, end), "End shows the document's bottom-right corner: {:?}", shown.view());
}

#[trial]
fn zoom_in_and_the_keys_step_the_ladder_and_a_step_past_the_least_zoom_makes_no_edit() {
    {
        let mut shown = Shown::focused(zoom_view(START), false);
        shown.fixture.key(Key::Equals, Modifiers::NONE);
        assert!(shown.view().zoom > START && shown.view().center == CENTRE, "= steps in about the centre: {:?}", shown.view());

        let least = zoom_view(ZoomLadder::min(DOC));
        let mut shown = Shown::focused(least, true);
        shown.fixture.key(Key::Minus, Modifiers::NONE);
        assert_eq!(shown.view(), least, "the least zoom settles");
        assert!(shown.edits.borrow().is_empty(), "and reports no edit: {:?}", shown.edits.borrow());
    }
    {
        let view = Rc::new(RefCell::new(row_view(START)));
        let mut fixture = zoom_row(&view, false);
        fixture.settle();
        let zoom_in = fixture.by_label("Zoom In").expect("the row has a Zoom In button");
        fixture.click(&zoom_in).settle();
        let after = *view.borrow();
        assert!(after.zoom > START && after.zoom <= ZoomLadder::MAX, "one rung in: {after:?}");
        assert_eq!(after.center, CENTRE, "the centre stays");
    }
}

#[trial]
fn the_wheel_zooms_about_the_view_centre() {
    let mut shown = Shown::new(zoom_view(START), false);
    let at = shown.frame().center();
    shown.fixture.frame(vec![moved(at)]);
    shown.fixture.frame(vec![wheel_lines(1.0)]);
    let after = shown.view();
    assert!(after.zoom != START && after.center == CENTRE, "a notch zooms and keeps the centre: {after:?}");
}

#[trial]
fn accesskit_scrolls_and_the_scroll_bars_move_the_view_by_the_arrow_step() {
    let mut shown = Shown::new(zoom_view(START), false);
    let span = ZOOM_CANVAS.size();
    let target = shown.fixture.by_label("Navigator").expect("the navigator has a node");
    shown.fixture.access(AccessActionKind::ScrollDown, &target);
    let down = f64::from(span.y) * ARROW_SHARE / START;
    assert!(near_doc(shown.view().center, Point::new(CENTRE.x, CENTRE.y + down)), "ScrollDown moves the view as Down does: {:?}", shown.view());

    let bar = shown.fixture.by_label("Visible area horizontal position").expect("the horizontal scroll bar has a node");
    shown.fixture.access(AccessActionKind::Increment, &bar);
    let right = f64::from(span.x) * ARROW_SHARE / START;
    let stepped = Point::new(CENTRE.x + right, CENTRE.y + down);
    assert!(near_doc(shown.view().center, stepped), "Increment moves the view by the arrow step: {:?}", shown.view());
}
// End of l4e-navprev.

// l4e-navpanel

#[trial]
fn the_preview_has_an_image_node_named_after_the_document_preview() {
    let shown = Shown::new(zoom_view(START), false);
    let picture = shown.fixture.by_label("Document preview").expect("the picture has a node");
    assert!(shown.fixture.by_role(Role::IMAGE).iter().any(|image| image.node == picture.node), "the picture's node is an image");
}

// End of l4e-navpanel.
