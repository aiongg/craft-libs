//! A document canvas owns its view in a pass: this pass's rect inside the rulers, the first fit, the wheel, a
//! middle-button pan, the Overscroll clamp and the zoom ease. The app only copies the view it reports.

use craft_behavior::CanvasView;
use craft_behavior::kurbo::{Point, Size};
use craft_ui::canvas::{Canvas, HandTool, NavigationOptions, ToolHost, canvas_space};
use craft_ui::contract::{Bind, Role, ShowContainer as _};
use craft_ui::patterns::canvas::NavigatorPanel;
use craft_ui::testing::pointer::{middle, moved, zoom_wheel};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Pos2, Rect, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const DOC: Size = Size::new(2000.0, 1500.0);
const CANVAS: Rect = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(800.0, 600.0));
/// The canvas beside a Navigator panel: right of the panel's column.
const BESIDE: Rect = Rect::from_min_max(Pos2::new(500.0, 0.0), Pos2::new(1300.0, 600.0));
const ROOM: Vec2 = Vec2::new(1400.0, 800.0);

/// Screen points two positions may differ by after the view's maths.
const SLACK: f32 = 0.01;
/// Zooms closer than this are the same, in points per document unit.
const SAME_ZOOM: f64 = 1e-9;

/// A view over the middle of `DOC` at half size: the document covers the canvas.
fn covering() -> CanvasView {
    CanvasView::new(CANVAS, Point::new(DOC.width / 2.0, DOC.height / 2.0), 0.5)
}

#[derive(Clone, Copy, Default)]
struct Options {
    animated: bool,
    hand: bool,
    navigator: bool,
}

/// A canvas over `DOC` whose app copies the one view the host reports, and what its body saw in every pass.
struct Document {
    fixture: Fixture,
    view: Rc<RefCell<CanvasView>>,
    rect: Rc<Cell<Rect>>,
    rulers: Rc<Cell<bool>>,
    /// The body's rect and the view in its `CanvasSpace`, one entry per pass.
    seen: Rc<RefCell<Vec<(Rect, CanvasView)>>>,
    /// The passes whose output carried a tool press.
    presses: Rc<Cell<usize>>,
}

impl Document {
    fn new(start: CanvasView, rect: Rect, options: Options) -> Self {
        let view = Rc::new(RefCell::new(start));
        let rect = Rc::new(Cell::new(rect));
        let rulers = Rc::new(Cell::new(false));
        let seen: Rc<RefCell<Vec<(Rect, CanvasView)>>> = Rc::default();
        let presses = Rc::new(Cell::new(0));
        let (held, at, ruled, saw, pressed) = (Rc::clone(&view), Rc::clone(&rect), Rc::clone(&rulers), Rc::clone(&seen), Rc::clone(&presses));
        let mut navigation = NavigationOptions::default();
        navigation.animated_zoom = options.animated;
        let mut hand = HandTool::new(navigation);
        let mut fixture = Fixture::new(move |ui| {
            if options.navigator {
                NavigatorPanel::new("Navigator", Bind::bound(&mut *held.borrow_mut()), DOC).navigation(navigation).show(ui);
            }
            let given = *held.borrow();
            let shown =
                Canvas::new("Document").view(given).document(DOC).navigation(navigation).show_rulers(ruled.get()).show_at(ui, at.get(), |_, body| {
                    if let Some(space) = canvas_space(&body.env) {
                        saw.borrow_mut().push((body.rect, space.view));
                    }
                });
            if shown.output.press.phase.is_press() {
                pressed.set(pressed.get() + 1);
            }
            let next = if options.hand { ToolHost::new().run(ui, &shown.output, &mut hand).view } else { shown.output.view };
            if let Some(next) = next {
                *held.borrow_mut() = next;
            }
        });
        fixture.room(ROOM);
        Document { fixture, view, rect, rulers, seen, presses }
    }

    fn view(&self) -> CanvasView {
        *self.view.borrow()
    }

    fn last_seen(&self) -> (Rect, CanvasView) {
        *self.seen.borrow().last().expect("the body ran")
    }

    /// The zooms the body showed since the last call.
    fn shown_zooms(&self) -> Vec<f64> {
        self.seen.borrow_mut().drain(..).map(|(_, view)| view.zoom).collect()
    }
}

/// `view.rect` lies in `body` past the rulers along the top and left edges, out to the body's far corner.
fn inside_the_rulers(body: Rect, view: CanvasView) -> bool {
    body.contains_rect(view.rect) && view.rect.min.x > body.min.x && view.rect.min.y > body.min.y && view.rect.max == body.max
}

/// The document's corners on screen through `view`.
fn page_on_screen(view: &CanvasView) -> Rect {
    let (a, b) = (view.to_screen(Point::ZERO), view.to_screen(Point::new(DOC.width, DOC.height)));
    Rect::from_two_pos(a, b)
}

#[trial]
fn the_body_reads_this_passes_rect_inside_the_rulers_while_they_show() {
    let start = CanvasView::new(Rect::from_min_size(Pos2::ZERO, Vec2::splat(10.0)), Point::new(1000.0, 750.0), 0.5);
    let mut document = Document::new(start, CANVAS, Options::default());
    document.rulers.set(true);
    document.fixture.frame(Vec::new());
    let (body, first) = *document.seen.borrow().first().expect("the body ran in the first pass");
    assert!(inside_the_rulers(body, first), "the first pass's view lies inside the rulers: {:?} in {body:?}", first.rect);

    let wider = Rect::from_min_max(CANVAS.min, CANVAS.max + Vec2::new(200.0, 100.0));
    document.rect.set(wider);
    document.fixture.frame(Vec::new());
    let (body, after) = document.last_seen();
    assert_eq!(body.max, wider.max, "the canvas took its new rect");
    assert!(inside_the_rulers(body, after), "the pass after a resize carries the new rect: {:?} in {body:?}", after.rect);

    document.rulers.set(false);
    document.fixture.frame(Vec::new());
    let (body, plain) = document.last_seen();
    assert_eq!(plain.rect, body, "with the rulers off the view takes the canvas's whole rect");
}

#[trial]
fn a_view_never_shown_opens_fitted_and_a_canvas_with_no_area_leaves_the_view_alone() {
    let hidden = Rect::from_min_size(CANVAS.min, Vec2::ZERO);
    let mut document = Document::new(CanvasView::new(Rect::NOTHING, Point::ZERO, 1.0), hidden, Options::default());
    document.fixture.settle();
    let rect = document.view().rect;
    assert!(!(rect.width() > 0.0 && rect.height() > 0.0), "a canvas with no area writes no rect: {rect:?}");

    document.rect.set(CANVAS);
    document.fixture.frame(Vec::new());
    for (when, view) in [("in the first pass with area", document.last_seen().1), ("in the view the app copied", document.view())] {
        let page = page_on_screen(&view);
        assert!(CANVAS.expand(SLACK).contains_rect(page), "{when}, the document fits the canvas: {page:?}");
        assert!((page.center() - CANVAS.center()).length() < SLACK, "{when}, centred: {page:?}");
    }

    // The user's own zoom survives a minimize and restore: a shown view is never refitted.
    let mut zoomed = document.view();
    zoomed.zoom = 1.0;
    *document.view.borrow_mut() = zoomed;
    document.rect.set(hidden);
    document.fixture.settle();
    document.rect.set(CANVAS);
    document.fixture.settle();
    assert_eq!((document.view().zoom, document.view().center), (zoomed.zoom, zoomed.center), "the restored canvas keeps the user's view");
}

#[trial]
fn ctrl_wheel_zooms_about_the_pointer() {
    let mut document = Document::new(covering(), CANVAS, Options::default());
    document.fixture.settle();
    let pointer = CANVAS.center() + Vec2::new(120.0, -70.0);
    document.fixture.frame(vec![moved(pointer)]);
    let before = document.view();
    let under = before.to_doc(pointer);
    document.fixture.frame(vec![zoom_wheel(60.0)]);
    let zoomed = document.view();
    assert!(zoomed.zoom > before.zoom, "Ctrl+wheel up zooms in: {} -> {}", before.zoom, zoomed.zoom);
    assert!((zoomed.to_screen(under) - pointer).length() < SLACK, "the point under the pointer stays: {:?}", zoomed.to_screen(under));
}

#[trial]
fn a_middle_drag_pans_by_the_pointers_travel_and_is_no_tool_press() {
    let mut document = Document::new(covering(), CANVAS, Options::default());
    document.fixture.settle();
    let from = CANVAS.center() + Vec2::new(120.0, -70.0);
    document.fixture.frame(vec![moved(from)]);
    let before = document.view();
    let under = before.to_doc(from);
    let fixture = &mut document.fixture;
    fixture.frame(vec![middle(from, true)]);
    for step in 1..=4 {
        fixture.frame(vec![moved(from + Vec2::new(8.0 * step as f32, 0.0))]);
    }
    fixture.frame(vec![middle(from + Vec2::new(32.0, 0.0), false)]);
    let panned = document.view();
    let travel = panned.to_screen(under) - before.to_screen(under);
    assert!((travel - Vec2::new(32.0, 0.0)).length() < SLACK, "the document follows the pointer, slop included: {travel:?}");
    assert_eq!(document.presses.get(), 0, "a middle press is never a tool press");
}

#[trial]
fn with_overscroll_off_a_hand_drag_cannot_move_the_page_out_of_view() {
    let mut document = Document::new(covering(), CANVAS, Options { hand: true, ..Options::default() });
    document.fixture.settle();
    let canvas = document.fixture.by_role(Role::CANVAS).into_iter().next().expect("the canvas has a node");
    document.fixture.drag(&canvas, Vec2::new(900.0, 450.0));
    let page = page_on_screen(&document.view());
    assert!(page.expand(SLACK).contains_rect(CANVAS), "the page still covers the canvas: {page:?}");
}

/// Shows the ease of a zoom `start` asks for: the app's view jumps to the target, and the body's view runs there over
/// the following passes and lands on it.
fn eases(document: &mut Document, start: impl FnOnce(&mut Fixture)) {
    let before = document.view();
    document.shown_zooms();
    start(&mut document.fixture);
    document.fixture.frame(Vec::new());
    document.fixture.advance(0.05).frame(Vec::new());
    let target = document.view();
    assert!(target.zoom > before.zoom, "the app's view takes the target at once: {} -> {}", before.zoom, target.zoom);
    let early = document.shown_zooms();
    assert!(
        early.iter().any(|zoom| *zoom > before.zoom + SAME_ZOOM && *zoom < target.zoom - SAME_ZOOM),
        "the canvas shows a view on the way: {early:?} from {} to {}",
        before.zoom,
        target.zoom
    );
    document.fixture.advance(1.0).frame(Vec::new());
    let landed = document.last_seen().1;
    assert!((landed.zoom - target.zoom).abs() < SAME_ZOOM, "the ease lands on the target: {landed:?}");
    assert_eq!(document.view().zoom, target.zoom, "and the app's view stays there");
}

#[trial]
fn a_zoom_command_under_animated_zoom_eases_on_the_canvas() {
    let mut document = Document::new(covering(), CANVAS, Options { animated: true, ..Options::default() });
    document.fixture.settle();
    document.fixture.click_at(CANVAS.center()).settle();
    eases(&mut document, |fixture| {
        fixture.key(Key::Equals, Modifiers::COMMAND);
    });
}

#[trial]
fn a_navigator_zoom_under_animated_zoom_eases_on_the_canvas() {
    let start = CanvasView::new(BESIDE, Point::new(DOC.width / 2.0, DOC.height / 2.0), 0.5);
    let mut document = Document::new(start, BESIDE, Options { animated: true, navigator: true, ..Options::default() });
    document.fixture.settle();
    // The first Tab focuses the preview box, whose `=` steps the zoom in.
    document.fixture.key(Key::Tab, Modifiers::NONE).settle();
    eases(&mut document, |fixture| {
        fixture.key(Key::Equals, Modifiers::NONE);
    });
}
