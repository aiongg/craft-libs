//! `CropOverlay`: the frame's gestures and keys through real input.

use craft_behavior::geometry::CropBox;
use craft_behavior::kurbo::Rect;
use craft_ui::canvas::{Canvas, CropOverlay, ToolPath};
use craft_ui::contract::{Bind, Show as _, ShowContainer as _};
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers, Vec2};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const FRAME: Rect = Rect::new(40.0, 30.0, 200.0, 140.0);

struct Shown {
    crop: Rc<RefCell<CropBox>>,
    applied: Rc<Cell<bool>>,
    discarded: Rc<Cell<bool>>,
    fixture: Fixture,
}

fn shown(configure: impl Fn(CropOverlay<'_>) -> CropOverlay<'_> + 'static) -> Shown {
    let crop = Rc::new(RefCell::new(CropBox::upright(FRAME)));
    let (applied, discarded) = (Rc::new(Cell::new(false)), Rc::new(Cell::new(false)));
    let (c, a, d) = (Rc::clone(&crop), Rc::clone(&applied), Rc::clone(&discarded));
    let mut fixture = Fixture::new(move |ui| {
        let _ = Canvas::new("Document", ToolPath::Command("the Crop To... command")).show(ui, |ui, body| {
            let out = configure(CropOverlay::new("Crop", Bind::bound(&mut *c.borrow_mut()))).show_at(ui, body.rect);
            a.set(a.get() || out.output.apply);
            d.set(d.get() || out.output.discard);
        });
    });
    fixture.settle();
    Shown { crop, applied, discarded, fixture }
}

fn travel(fixture: &Fixture) -> f32 {
    4.0 * craft_ui::testing::input::max_click_dist(fixture.ctx())
}

#[trial]
fn a_corner_drag_resizes_and_keeps_the_opposite_corner() {
    let mut s = shown(|c| c);
    let corner = s.fixture.by_label("Crop corner, bottom right").expect("corner");
    let by = travel(&s.fixture);
    s.fixture.drag(&corner, Vec2::new(by, by));
    let rect = s.crop.borrow().rect;
    assert_eq!((rect.x0, rect.y0), (FRAME.x0, FRAME.y0));
    assert!(rect.x1 > FRAME.x1 && rect.y1 > FRAME.y1, "{rect:?}");
}

#[trial]
fn a_held_ratio_is_kept_by_a_corner_drag() {
    let mut s = shown(|c| c.ratio(2.0));
    let corner = s.fixture.by_label("Crop corner, bottom right").expect("corner");
    let by = travel(&s.fixture);
    s.fixture.drag(&corner, Vec2::new(by, 0.0));
    let rect = s.crop.borrow().rect;
    assert!((rect.width() / rect.height() - 2.0).abs() < 1e-6, "{rect:?}");
}

#[trial]
fn a_drag_inside_moves_the_frame_and_a_drag_inside_the_default_frame_draws() {
    let mut s = shown(|c| c);
    let body = s.fixture.by_label("Whole frame").expect("whole frame");
    let by = travel(&s.fixture);
    s.fixture.drag(&body, Vec2::new(by, 0.0));
    let moved = s.crop.borrow().rect;
    assert!(moved.x0 > FRAME.x0 && (moved.width() - FRAME.width()).abs() < 1e-9, "{moved:?}");

    let mut s = shown(|c| c.default_frame());
    let body = s.fixture.by_label("Whole frame").expect("whole frame");
    s.fixture.drag(&body, Vec2::new(by, by));
    let drawn = s.crop.borrow().rect;
    assert!(drawn.width() < FRAME.width() && drawn.width() > 0.0, "{drawn:?}");
}

#[trial]
fn enter_applies_and_escape_discards_on_a_focused_part() {
    let mut s = shown(|c| c);
    let body = s.fixture.by_label("Whole frame").expect("whole frame");
    s.fixture.click(&body);
    s.fixture.key(Key::Enter, Modifiers::NONE);
    assert!(s.applied.get());
    s.fixture.key(Key::Escape, Modifiers::NONE);
    assert!(s.discarded.get());
}

#[trial]
fn an_arrow_moves_the_whole_frame_and_x_swaps_it() {
    let mut s = shown(|c| c);
    let body = s.fixture.by_label("Whole frame").expect("whole frame");
    s.fixture.click(&body);
    s.fixture.key(Key::ArrowRight, Modifiers::NONE);
    assert!(s.crop.borrow().rect.x0 > FRAME.x0);
    let before = s.crop.borrow().rect;
    s.fixture.key(Key::X, Modifiers::NONE);
    let after = s.crop.borrow().rect;
    assert!((after.width() - before.height()).abs() < 1e-9 && (after.height() - before.width()).abs() < 1e-9, "{after:?}");
}

#[trial]
fn a_side_under_one_unit_cancels_the_drag() {
    let mut s = shown(|c| c);
    let corner = s.fixture.by_label("Crop corner, top left").expect("corner");
    s.fixture.drag(&corner, Vec2::new(FRAME.width() as f32 + 0.2, 0.0));
    assert_eq!(s.crop.borrow().rect, FRAME, "the frame returns to what it was");
}

#[trial]
fn every_part_has_a_name() {
    let s = shown(|c| c);
    for name in [
        "Whole frame",
        "Crop corner, top left",
        "Crop corner, top right",
        "Crop corner, bottom right",
        "Crop corner, bottom left",
        "Crop edge, top",
        "Crop edge, right",
        "Crop edge, bottom",
        "Crop edge, left",
    ] {
        assert!(s.fixture.by_label(name).is_some(), "{name}");
    }
}
