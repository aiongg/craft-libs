//! The preview pane: the pan as AccessKit sees it (the scroll actions of the axes it pans on, and the same pan as the
//! arrow keys), the states, fit, the zoom range, zoom about the pointer, the click rule and a view shared by panes.

use craft_behavior::CanvasTransform;
use craft_ui::contract::{Role, Show as _};
use craft_ui::patterns::form_dialogs::{PreviewPane, PreviewState};
use craft_ui::testing::access::Declared;
use craft_ui::testing::focus::AccessActionKind;
use craft_ui::testing::pointer;
use craft_ui::testing::{Fixture, trial};
use egui::load::SizedTexture;
use egui::{ColorImage, Key, Modifiers, Pos2, Rect, TextureHandle, TextureOptions, Vec2, pos2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A picture much wider than the well and much shorter than it: the pane pans along the horizontal axis only.
const WIDE: [usize; 2] = [2000, 8];

struct Rig {
    fixture: Fixture,
    view: Rc<Cell<CanvasTransform>>,
}

fn rig() -> Rig {
    let view = Rc::new(Cell::new(CanvasTransform::new(1.0, Vec2::ZERO)));
    let handle: RefCell<Option<TextureHandle>> = RefCell::new(None);
    let held = Rc::clone(&view);
    let mut fixture = Fixture::new(move |ui| {
        let mut handle = handle.borrow_mut();
        let texture =
            handle.get_or_insert_with(|| ui.ctx().load_texture("preview-pan", ColorImage::filled(WIDE, Default::default()), TextureOptions::LINEAR));
        let sized = SizedTexture::new(texture.id(), Vec2::new(WIDE[0] as f32, WIDE[1] as f32));
        if let Some(next) = PreviewPane::new("Preview", sized, held.get()).show(ui).view {
            held.set(next);
        }
    });
    fixture.settle();
    Rig { fixture, view }
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

/// AccessKit ScrollRight pans the view as ArrowRight does.
#[trial]
fn scroll_right_pans_as_the_right_arrow_does() {
    let mut keyed = rig();
    keyed.fixture.key(Key::Tab, Modifiers::NONE);
    keyed.fixture.idle(2);
    keyed.fixture.key(Key::ArrowRight, Modifiers::NONE);
    keyed.fixture.idle(2);
    let mut scrolled = rig();
    let pane = scrolled.fixture.by_label("Preview").expect("the pane has a node");
    scrolled.fixture.access(AccessActionKind::ScrollRight, &pane);
    scrolled.fixture.idle(2);
    assert!(keyed.view.get().offset.x < 0.0, "the arrow pans");
    assert_eq!(scrolled.view.get(), keyed.view.get(), "the scroll action pans the same way");
}

/// Where the panes below sit, and their well.
const ORIGIN: Pos2 = Pos2::new(10.0, 10.0);
const WELL: Vec2 = Vec2::new(200.0, 100.0);
/// A picture ten times the well, so every pan and zoom below stays inside it.
const BIG: [usize; 2] = [2000, 1000];
const EDGE: f32 = 1e-3;
/// A zoom range wide enough that no fit below is clamped.
const WIDE_RANGE: std::ops::RangeInclusive<f32> = 0.0001..=32.0;

/// What one pane under test reports, kept between passes the way a dialog keeps it.
#[derive(Clone)]
struct Held {
    view: Rc<Cell<CanvasTransform>>,
    fit: Rc<Cell<bool>>,
    shown: Rc<Cell<CanvasTransform>>,
    well: Rc<Cell<Vec2>>,
    reported: Rc<Cell<u32>>,
}

impl Held {
    fn new(well: Vec2) -> Self {
        Held {
            view: Rc::new(Cell::new(CanvasTransform::new(1.0, Vec2::ZERO))),
            fit: Rc::new(Cell::new(true)),
            shown: Rc::new(Cell::new(CanvasTransform::new(1.0, Vec2::ZERO))),
            well: Rc::new(Cell::new(well)),
            reported: Rc::new(Cell::new(0)),
        }
    }
}

/// A fixture with one pane of a `picture`-sized picture at `ORIGIN`; `state` makes its state from the texture and
/// `config` sets its builders from the held fit flag.
fn pane_rig(picture: [usize; 2], held: &Held, state: fn(SizedTexture) -> PreviewState, config: fn(PreviewPane, bool) -> PreviewPane) -> Fixture {
    let handle: RefCell<Option<TextureHandle>> = RefCell::new(None);
    let held = held.clone();
    let mut fixture = Fixture::new(move |ui| {
        let mut handle = handle.borrow_mut();
        let texture = handle
            .get_or_insert_with(|| ui.ctx().load_texture("preview-pane", ColorImage::filled(picture, Default::default()), TextureOptions::LINEAR));
        let sized = SizedTexture::new(texture.id(), Vec2::new(picture[0] as f32, picture[1] as f32));
        let pane = config(PreviewPane::state("Preview", state(sized), held.view.get()), held.fit.get());
        let out = pane.show_at(ui, Rect::from_min_size(ORIGIN, held.well.get()));
        held.shown.set(out.shown);
        if let Some(view) = out.view {
            held.view.set(view);
            held.reported.set(held.reported.get() + 1);
        }
        if let Some(fit) = out.fit {
            held.fit.set(fit);
        }
    });
    fixture.settle();
    fixture
}

fn ready(texture: SizedTexture) -> PreviewState {
    PreviewState::Ready(texture)
}

fn plain(pane: PreviewPane, _: bool) -> PreviewPane {
    pane
}

fn fitting(pane: PreviewPane, fit: bool) -> PreviewPane {
    pane.fit(fit).zoom_range(WIDE_RANGE)
}

fn centre_of(held: &Held) -> Pos2 {
    ORIGIN + held.well.get() / 2.0
}

fn zoom_at(fixture: &mut Fixture, at: Pos2, dy: f32) {
    fixture.frame(vec![pointer::moved(at)]);
    fixture.frame(vec![pointer::zoom_wheel(dy)]);
    fixture.idle(2);
}

fn double_click(fixture: &mut Fixture, at: Pos2) {
    fixture.frame(vec![pointer::moved(at)]);
    for _ in 0..2 {
        fixture.frame(vec![pointer::press(at)]);
        fixture.frame(vec![pointer::release(at)]);
    }
    fixture.idle(2);
}

/// A pane that is pending or failed draws no picture, so there is nothing to pan: a drag reports no view where the
/// same drag over a picture does. A failed pane shows its message, a pending one its progress line, and the pane keeps
/// its name.
#[trial]
fn a_pane_without_a_picture_shows_its_state_and_reports_no_view() {
    let ready_held = Held::new(WELL);
    let mut with_picture = pane_rig(BIG, &ready_held, ready, plain);
    let pane = with_picture.by_label("Preview").expect("the pane has a node");
    with_picture.drag(&pane, Vec2::new(-30.0, 0.0));
    assert!(ready_held.reported.get() > 0, "a drag over a picture pans");

    let failed_held = Held::new(WELL);
    let mut failed = pane_rig(BIG, &failed_held, |_| PreviewState::Failed("The preview failed.".into()), plain);
    let pane = failed.by_label("Preview").expect("a failed pane keeps its name");
    failed.drag(&pane, Vec2::new(-30.0, 0.0));
    assert_eq!(failed_held.reported.get(), 0, "a failed pane reports no view");
    assert!(failed.texts().iter().any(|text| text.text.contains("The preview failed.")), "the message is shown");

    let pending_held = Held::new(WELL);
    let mut pending = pane_rig(BIG, &pending_held, |_| PreviewState::Pending(Some(0.5)), plain);
    let pane = pending.by_label("Preview").expect("a pending pane keeps its name");
    pending.drag(&pane, Vec2::new(-30.0, 0.0));
    assert_eq!(pending_held.reported.get(), 0, "a pending pane reports no view");
    assert!(!pending.by_role(Role::PROGRESS_INDICATOR).is_empty(), "the progress line is shown");
    let tree = pending.tree();
    assert!(!tree.declares_key(pane.node, Declared::ScrollLeft), "a pane with no picture declares no pan");
}

/// The zoom follows the well while the pane fits; a wheel zoom leaves fit and a double click returns to it.
#[trial]
fn a_fitting_pane_follows_its_well_until_the_user_zooms_and_a_double_click_returns() {
    let held = Held::new(WELL);
    let mut fixture = pane_rig(BIG, &held, ready, fitting);
    let expected = |well: Vec2| (well.x / BIG[0] as f32).min(well.y / BIG[1] as f32);
    assert!((held.shown.get().scale - expected(WELL)).abs() < EDGE, "the picture fits the well: {:?}", held.shown.get());
    assert_eq!(held.reported.get(), 0, "fitting reports no view");

    let bigger = Vec2::new(400.0, 300.0);
    held.well.set(bigger);
    fixture.idle(3);
    assert!((held.shown.get().scale - expected(bigger)).abs() < EDGE, "the zoom follows the resized well: {:?}", held.shown.get());
    assert!(held.fit.get(), "resizing the well does not leave fit");

    zoom_at(&mut fixture, centre_of(&held) + Vec2::new(30.0, 10.0), 40.0);
    assert!(!held.fit.get(), "a wheel zoom leaves fit");
    assert!((held.shown.get().scale - expected(bigger)).abs() > EDGE, "the zoom changed");

    double_click(&mut fixture, centre_of(&held));
    assert!(held.fit.get(), "a double click returns to fit");
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

    let held = Held::new(WELL);
    let mut fixture = pane_rig(BIG, &held, ready, |pane, _| pane.zoom_range(0.5..=2.0));
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

/// A pane built with a zoom range zooms about the pointer: the picture point under it stays under it.
#[trial]
fn wheel_zoom_keeps_the_point_under_the_pointer() {
    let held = Held::new(WELL);
    let mut fixture = pane_rig(BIG, &held, ready, |pane, _| pane.zoom_range(WIDE_RANGE));
    let before = held.shown.get();
    let about = Vec2::new(50.0, 20.0);
    zoom_at(&mut fixture, centre_of(&held) + about, 20.0);
    let after = held.shown.get();
    assert!((after.scale - before.scale).abs() > EDGE, "the wheel zoomed");
    let canvas_before = (about - before.offset) / before.scale;
    let canvas_after = (about - after.offset) / after.scale;
    assert!((canvas_before - canvas_after).length() < 0.05, "the point under the pointer moved: {canvas_before:?} -> {canvas_after:?}");
}

/// With `click_centres(false)` a click reports no view; by default it centres the view on the point clicked.
#[trial]
fn click_centres_decides_whether_a_click_moves_the_view() {
    let held = Held::new(WELL);
    let mut fixture = pane_rig(BIG, &held, ready, |pane, _| pane.click_centres(false));
    fixture.click_at(centre_of(&held) + Vec2::new(60.0, 20.0));
    assert_eq!(held.reported.get(), 0, "a click leaves the view");

    let held = Held::new(WELL);
    let mut fixture = pane_rig(BIG, &held, ready, plain);
    fixture.click_at(centre_of(&held) + Vec2::new(60.0, 20.0));
    assert!(held.reported.get() > 0 && held.view.get().offset.x < 0.0, "a click centres the view on the point");
}

/// Two panes given one view show the same part, and a pan on one reports a view that, applied, moves both.
#[trial]
fn panes_given_one_view_show_the_same_part() {
    let view = Rc::new(Cell::new(CanvasTransform::new(1.0, Vec2::ZERO)));
    let shown = [Rc::new(Cell::new(view.get())), Rc::new(Cell::new(view.get()))];
    let (given, seen) = (Rc::clone(&view), shown.clone());
    let handle: RefCell<Option<TextureHandle>> = RefCell::new(None);
    let mut fixture = Fixture::new(move |ui| {
        let mut handle = handle.borrow_mut();
        let texture =
            handle.get_or_insert_with(|| ui.ctx().load_texture("preview-two", ColorImage::filled(BIG, Default::default()), TextureOptions::LINEAR));
        let sized = SizedTexture::new(texture.id(), Vec2::new(BIG[0] as f32, BIG[1] as f32));
        let mut reported = None;
        for (at, (name, seen)) in ["Left", "Right"].into_iter().zip(&seen).enumerate() {
            let rect = Rect::from_min_size(pos2(ORIGIN.x + at as f32 * (WELL.x + 20.0), ORIGIN.y), WELL);
            let out = PreviewPane::new(name, sized, given.get()).show_at(ui, rect);
            seen.set(out.shown);
            reported = reported.or(out.view);
        }
        if let Some(next) = reported {
            given.set(next);
        }
    });
    fixture.settle();
    assert_eq!(shown[0].get(), shown[1].get(), "both panes show the same part");
    let left = fixture.by_label("Left").expect("the left pane has a node");
    fixture.drag(&left, Vec2::new(-40.0, 0.0));
    assert!(view.get().offset.x < 0.0, "the pan reported a view");
    assert_eq!(shown[0].get(), shown[1].get(), "the applied view moves both panes alike");
    assert!(shown[1].get().offset.x < 0.0, "the other pane moved too");
}
