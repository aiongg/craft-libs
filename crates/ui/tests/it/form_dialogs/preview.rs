//! The preview pane: the pan as AccessKit sees it (the scroll actions of the axes it pans on, and the same pan as the
//! arrow keys), the states, fit, the zoom range, zoom about the pointer, the wheel routing, the click rule and a view
//! shared by panes.

use craft_behavior::{CanvasTransform, WheelTaker};
use craft_ui::contract::Show as _;
use craft_ui::patterns::form_dialogs::{PreviewPane, PreviewState, PreviewView};
use craft_ui::testing::access::Declared;
use craft_ui::testing::pointer;
use craft_ui::testing::wheel::{register, take};
use craft_ui::testing::{Fixture, trial};
use egui::load::SizedTexture;
use egui::{ColorImage, Id, Pos2, Rect, TextureHandle, TextureOptions, Vec2, pos2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A picture much wider than the well and much shorter than it: the pane pans along the horizontal axis only.
const WIDE: [usize; 2] = [2000, 8];

struct Rig {
    fixture: Fixture,
}

fn rig() -> Rig {
    let view = Rc::new(RefCell::new(PreviewView::default()));
    let handle: RefCell<Option<TextureHandle>> = RefCell::new(None);
    let held = Rc::clone(&view);
    let mut fixture = Fixture::new(move |ui| {
        let mut handle = handle.borrow_mut();
        let texture =
            handle.get_or_insert_with(|| ui.ctx().load_texture("preview-pan", ColorImage::filled(WIDE, Default::default()), TextureOptions::LINEAR));
        let sized = SizedTexture::new(texture.id(), Vec2::new(WIDE[0] as f32, WIDE[1] as f32));
        let next = PreviewPane::new("Preview", sized, &held.borrow()).show(ui).view;
        if let Some(next) = next {
            *held.borrow_mut() = next;
        }
    });
    fixture.settle();
    Rig { fixture }
}

/// A picture wider than the well declares ScrollLeft and ScrollRight on the pane, and no vertical scroll action.
#[trial]
fn a_pane_declares_the_scroll_actions_of_the_axes_it_pans_on() {
    let rig = rig();
    let pane = rig.fixture.by_label("Preview").expect("the pane has a node");
    let tree = rig.fixture.tree();
    assert!(
        tree.declares_key(pane.node, Declared::ScrollLeft) && tree.declares_key(pane.node, Declared::ScrollRight),
        "the horizontal pan is declared"
    );
    assert!(
        !tree.declares_key(pane.node, Declared::ScrollUp) && !tree.declares_key(pane.node, Declared::ScrollDown),
        "a picture that fits vertically does not pan vertically"
    );
}

/// Where the panes below sit, and their well.
const ORIGIN: Pos2 = Pos2::new(10.0, 10.0);
const WELL: Vec2 = Vec2::new(200.0, 100.0);
/// A picture ten times the well, so every pan and zoom below stays inside it.
const BIG: [usize; 2] = [2000, 1000];
const EDGE: f32 = 1e-3;
/// A zoom range wide enough that no fit below is clamped.
const WIDE_RANGE: std::ops::RangeInclusive<f32> = 0.0001..=32.0;
/// The scroll region around the panes below.
const BODY: Vec2 = Vec2::new(600.0, 400.0);
/// More frames than a wheel gesture takes to settle.
const STILL_LIMIT: usize = 120;

/// What one pane under test reports, kept between passes the way a dialog keeps it.
#[derive(Clone)]
struct Held {
    view: Rc<RefCell<PreviewView>>,
    shown: Rc<Cell<CanvasTransform>>,
    well: Rc<Cell<Vec2>>,
    reported: Rc<Cell<u32>>,
    /// The scroll an enclosing scroll region (a dialog body) under the pane was given.
    body: Rc<Cell<Vec2>>,
}

impl Held {
    fn new(well: Vec2) -> Self {
        Held::with(well, PreviewView::default())
    }

    fn with(well: Vec2, view: PreviewView) -> Self {
        Held {
            view: Rc::new(RefCell::new(view)),
            shown: Rc::new(Cell::new(CanvasTransform::new(1.0, Vec2::ZERO))),
            well: Rc::new(Cell::new(well)),
            reported: Rc::new(Cell::new(0)),
            body: Rc::new(Cell::new(Vec2::ZERO)),
        }
    }

    fn fit(&self) -> bool {
        self.view.borrow().fit
    }

    fn offset(&self) -> Vec2 {
        self.view.borrow().transform.offset
    }
}

/// A fixture with one pane of a `picture`-sized picture at `ORIGIN` inside a scroll region that takes both scroll
/// axes; `state` makes the pane's state from the texture and `config` sets its builders.
fn pane_rig(picture: [usize; 2], held: &Held, state: fn(SizedTexture) -> PreviewState, config: fn(PreviewPane) -> PreviewPane) -> Fixture {
    let handle: RefCell<Option<TextureHandle>> = RefCell::new(None);
    let held = held.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut handle = handle.borrow_mut();
        let texture = handle
            .get_or_insert_with(|| ui.ctx().load_texture("preview-pane", ColorImage::filled(picture, Default::default()), TextureOptions::LINEAR));
        let sized = SizedTexture::new(texture.id(), Vec2::new(picture[0] as f32, picture[1] as f32));
        let body = Id::unique("preview-body");
        register(ui.ctx(), WheelTaker::new(body, Rect::from_min_size(ORIGIN, BODY)).vertical(true).horizontal(true));
        let pane = config(PreviewPane::state("Preview", state(sized), &held.view.borrow()));
        let out = pane.show_at(ui, Rect::from_min_size(ORIGIN, held.well.get()));
        held.shown.set(out.shown.transform);
        if let Some(view) = out.view {
            *held.view.borrow_mut() = view;
            held.reported.set(held.reported.get() + 1);
        }
        if let Some(share) = take(ui.ctx(), body) {
            held.body.set(held.body.get() + share);
        }
    });
    fixture.settle();
    fixture
}

fn ready(texture: SizedTexture) -> PreviewState {
    PreviewState::Ready(texture)
}

fn plain(pane: PreviewPane) -> PreviewPane {
    pane
}

fn fitting() -> PreviewView {
    PreviewView::fitting().zoom_range(WIDE_RANGE)
}

fn centre_of(held: &Held) -> Pos2 {
    ORIGIN + held.well.get() / 2.0
}

fn zoom_at(fixture: &mut Fixture, at: Pos2, dy: f32) {
    fixture.frame(vec![pointer::moved(at)]);
    fixture.frame(vec![pointer::zoom_wheel(dy)]);
    fixture.idle(2);
}

/// Frames until the pane reports nothing new: egui spreads a wheel zoom over several frames, and the rest of it is
/// still a wheel zoom when it arrives.
fn until_still(fixture: &mut Fixture, held: &Held) {
    let mut seen = u32::MAX;
    for _ in 0..STILL_LIMIT {
        fixture.idle(1);
        if held.reported.get() == seen {
            return;
        }
        seen = held.reported.get();
    }
    panic!("the pane keeps reporting a new view");
}

fn double_click(fixture: &mut Fixture, at: Pos2) {
    fixture.frame(vec![pointer::moved(at)]);
    for _ in 0..2 {
        fixture.frame(vec![pointer::press(at)]);
        fixture.frame(vec![pointer::release(at)]);
    }
    fixture.idle(2);
}

/// The zoom follows the well while the pane fits; a wheel zoom leaves fit and a double click returns to it.
#[trial]
fn a_fitting_pane_follows_its_well_until_the_user_zooms_and_a_double_click_returns() {
    let held = Held::with(WELL, fitting());
    let mut fixture = pane_rig(BIG, &held, ready, plain);
    let expected = |well: Vec2| (well.x / BIG[0] as f32).min(well.y / BIG[1] as f32);
    assert!((held.shown.get().scale - expected(WELL)).abs() < EDGE, "the picture fits the well: {:?}", held.shown.get());
    assert_eq!(held.reported.get(), 0, "fitting reports no view");

    let bigger = Vec2::new(400.0, 300.0);
    held.well.set(bigger);
    fixture.idle(3);
    assert!((held.shown.get().scale - expected(bigger)).abs() < EDGE, "the zoom follows the resized well: {:?}", held.shown.get());
    assert!(held.fit(), "resizing the well does not leave fit");

    zoom_at(&mut fixture, centre_of(&held) + Vec2::new(30.0, 10.0), 40.0);
    assert!(!held.fit(), "a wheel zoom leaves fit");
    assert!((held.shown.get().scale - expected(bigger)).abs() > EDGE, "the zoom changed");
    until_still(&mut fixture, &held);

    double_click(&mut fixture, centre_of(&held));
    assert!(
        held.fit(),
        "a double click returns to fit: {:?} reported {} times, shown {:?}",
        held.view.borrow(),
        held.reported.get(),
        held.shown.get()
    );
    assert!((held.shown.get().scale - expected(bigger)).abs() < EDGE, "the view is the fit view again: {:?}", held.shown.get());
}

/// The wheel and the zoom buttons stop at the ends of `zoom_range`.
#[trial]
fn zoom_range_bounds_the_wheel_and_the_buttons() {
    let range = 0.5..=2.0;
    let mut view = CanvasTransform::new(1.0, Vec2::ZERO);
    for _ in 0..20 {
        view = PreviewPane::zoom_in_within(view, &range);
    }
    assert_eq!(view.scale, 2.0, "zoom in stops at the end of the range");
    assert!(!PreviewPane::can_zoom_in_within(view, &range) && PreviewPane::can_zoom_out_within(view, &range));
    for _ in 0..20 {
        view = PreviewPane::zoom_out_within(view, &range);
    }
    assert_eq!(view.scale, 0.5, "zoom out stops at the start of the range");
    assert!(PreviewPane::can_zoom_in_within(view, &range) && !PreviewPane::can_zoom_out_within(view, &range));

    let held = Held::with(WELL, PreviewView::default().zoom_range(0.5..=2.0));
    let mut fixture = pane_rig(BIG, &held, ready, plain);
    for _ in 0..6 {
        zoom_at(&mut fixture, centre_of(&held), 60.0);
    }
    let first = held.shown.get().scale;
    for _ in 0..12 {
        zoom_at(&mut fixture, centre_of(&held), -60.0);
    }
    let second = held.shown.get().scale;
    assert!((first.max(second) - 2.0).abs() < EDGE, "the wheel reaches the end of the range and stops there: {first} {second}");
    assert!((first.min(second) - 0.5).abs() < EDGE, "the wheel reaches the start of the range and stops there: {first} {second}");
}

/// Every pane zooms about the pointer: the picture point under it stays under it.
#[trial]
fn wheel_zoom_keeps_the_point_under_the_pointer() {
    let held = Held::new(WELL);
    let mut fixture = pane_rig(BIG, &held, ready, plain);
    let before = held.shown.get();
    let about = Vec2::new(50.0, 20.0);
    zoom_at(&mut fixture, centre_of(&held) + about, 20.0);
    let after = held.shown.get();
    assert!((after.scale - before.scale).abs() > EDGE, "the wheel zoomed");
    let canvas_before = (about - before.offset) / before.scale;
    let canvas_after = (about - after.offset) / after.scale;
    assert!((canvas_before - canvas_after).length() < 0.05, "the point under the pointer moved: {canvas_before:?} -> {canvas_after:?}");
}

/// The wheel goes to the scroll region around a pane while the picture fits the well, pans the picture where it
/// overflows, and zooms in both; a fitting pane never swallows the vertical scroll.
#[trial]
fn the_wheel_scrolls_the_body_over_a_fitting_pane_and_pans_an_overflowing_one() {
    let scroll = |fixture: &mut Fixture, at: Pos2| {
        fixture.frame(vec![pointer::moved(at)]);
        fixture.frame(vec![pointer::wheel(-40.0)]);
        fixture.idle(3);
    };

    let fitting_held = Held::with(WELL, fitting());
    let mut fitting_pane = pane_rig(BIG, &fitting_held, ready, plain);
    scroll(&mut fitting_pane, centre_of(&fitting_held));
    assert!(fitting_held.body.get().y != 0.0, "the scroll region under a fitting pane takes the wheel");
    assert_eq!(fitting_held.reported.get(), 0, "a fitting pane pans nothing");
    let before = fitting_held.body.get();
    zoom_at(&mut fitting_pane, centre_of(&fitting_held), 40.0);
    assert_eq!(fitting_held.body.get(), before, "zoom over a fitting pane stays with the pane");
    assert!(!fitting_held.fit(), "the pane took the zoom");

    let overflowing = Held::new(WELL);
    let mut overflowing_pane = pane_rig(BIG, &overflowing, ready, plain);
    scroll(&mut overflowing_pane, centre_of(&overflowing));
    assert_eq!(overflowing.body.get(), Vec2::ZERO, "the pane takes the wheel while the picture overflows the well");
    assert!(overflowing.offset().y != 0.0, "the wheel pans the picture");
    let zoomed = overflowing.shown.get().scale;
    zoom_at(&mut overflowing_pane, centre_of(&overflowing), 40.0);
    assert!((overflowing.shown.get().scale - zoomed).abs() > EDGE, "zoom works over an overflowing pane");
}

/// A fixed pane (`PreviewView::fixed`) sizes from the room it is given: in a tile smaller
/// than the plot token it stays inside the tile.
#[trial]
fn a_fitted_fixed_pane_stays_inside_a_tile_smaller_than_the_plot_token() {
    const TILE: Vec2 = Vec2::new(133.0, 75.0);
    let seen = Rc::new(Cell::new((Rect::NOTHING, Rect::NOTHING)));
    let out = Rc::clone(&seen);
    let moved_seen = Rc::new(Cell::new(false));
    let moved = Rc::clone(&moved_seen);
    let handle: RefCell<Option<TextureHandle>> = RefCell::new(None);
    let mut fixture = Fixture::new(move |ui| {
        let mut handle = handle.borrow_mut();
        let texture = handle.get_or_insert_with(|| {
            ui.ctx().load_texture("preview-fixed", ColorImage::filled([240, 160], Default::default()), TextureOptions::LINEAR)
        });
        let sized = SizedTexture::new(texture.id(), Vec2::new(240.0, 160.0));
        let view = PreviewView::fixed();
        ui.set_max_size(TILE);
        let tile = ui.max_rect();
        let shown = PreviewPane::new("Tile", sized, &view).show(ui);
        out.set((tile, shown.response.rect));
        moved.set(moved.get() || shown.view.is_some());
    });
    fixture.settle();
    fixture.frame(vec![pointer::moved(pos2(60.0, 40.0))]);
    fixture.frame(vec![pointer::zoom_wheel(3.0)]);
    fixture.idle(2);
    assert!(!moved_seen.get(), "the wheel leaves a fixed view unchanged");
    let (tile, rect) = seen.get();
    assert!(tile.contains_rect(rect), "the pane is {rect:?} in the tile {tile:?}");
}
