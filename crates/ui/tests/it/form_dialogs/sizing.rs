//! The size dialogs' own logic: the answer at OK, the refusal of an empty size and the method list.

use craft_behavior::PixelSize;
use craft_ui::contract::Bind;
use craft_ui::patterns::form_dialogs::{CanvasExtension, CanvasSize, ImageSize, Resample, SizingWords};
use craft_ui::pickers::Anchor9;
use craft_ui::runtime::env::UnitKind;
use craft_ui::runtime::env::WordSet as _;
use craft_ui::testing::{Fixture, trial};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const PICTURE: PixelSize = PixelSize::new(1600, 900);
const RESOLUTION: f64 = 300.0;

/// A fixture with a dialog built by `build`, which keeps the answer and whether the dialog is still open.
fn rig<R: 'static>(
    build: impl for<'a> Fn(Bind<'a, bool>, &mut egui::Ui) -> Option<R> + 'static,
) -> (Fixture, Rc<RefCell<Option<R>>>, Rc<Cell<bool>>) {
    let answer = Rc::new(RefCell::new(None));
    let open = Rc::new(Cell::new(true));
    let (kept, state) = (Rc::clone(&answer), Rc::clone(&open));
    let mut fixture = Fixture::new(move |ui| {
        let mut now = state.get();
        if let Some(result) = build(Bind::bound(&mut now), ui) {
            *kept.borrow_mut() = Some(result);
        }
        state.set(now);
    });
    fixture.settle();
    (fixture, answer, open)
}

fn press_ok(fixture: &mut Fixture) {
    let ok = fixture.by_label("OK").expect("the OK button is shown");
    fixture.click(&ok);
    fixture.idle(3);
}

/// OK answers the values the dialog opened with: the size, the resolution, the units and the method.
#[trial]
fn image_size_answers_the_opening_values() {
    let (mut fixture, answer, open) =
        rig(|open, ui| ImageSize::new(PICTURE).pixels_per_inch(RESOLUTION).method(Resample::Lanczos).opened(open).show(ui).result);
    press_ok(&mut fixture);
    let expected = answer.borrow_mut().take().expect("OK answers");
    assert_eq!(expected.size, PICTURE);
    assert_eq!(expected.pixels_per_inch, RESOLUTION);
    assert_eq!((expected.unit, expected.resolution_unit), (UnitKind::Pixels, UnitKind::PixelsPerInch));
    assert_eq!(expected.resample, Some(Resample::Lanczos));
    assert!(!open.get(), "OK closes the dialog");
}

/// With Resample off the answer names no method.
#[trial]
fn image_size_with_resample_off_answers_no_method() {
    let (mut fixture, answer, _) = rig(|open, ui| ImageSize::new(PICTURE).resampling(false).opened(open).show(ui).result);
    press_ok(&mut fixture);
    assert_eq!(answer.borrow().as_ref().map(|result| result.resample), Some(None));
}

/// An empty size cannot be answered: OK leaves the dialog open and answers nothing.
#[trial]
fn an_empty_size_refuses_ok() {
    let (mut fixture, answer, open) = rig(|open, ui| ImageSize::new(PixelSize::new(0, 900)).opened(open).show(ui).result);
    press_ok(&mut fixture);
    assert!(answer.borrow().is_none(), "no answer for an empty size");
    assert!(open.get(), "the dialog stays open");
}

/// A document with no background answers Transparent whatever extension the dialog was given.
#[trial]
fn canvas_size_without_a_background_answers_transparent() {
    let (mut fixture, answer, _) = rig(|open, ui| {
        CanvasSize::new(PICTURE).extension(CanvasExtension::White).has_background(false).anchor(Anchor9::TopLeft).opened(open).show(ui).result
    });
    press_ok(&mut fixture);
    let result = answer.borrow_mut().take().expect("OK answers");
    assert_eq!(result.extension, CanvasExtension::Transparent);
    assert_eq!(result.anchor, Anchor9::TopLeft);
    assert_eq!(result.size, PICTURE);
}

/// The method list keeps the order of the full list whatever order the app names the methods in.
#[trial]
fn the_method_list_follows_the_full_list() {
    let words = SizingWords::english();
    let listed = |entries: Vec<craft_ui::contract::Entry<Resample>>| entries.iter().filter_map(|entry| entry.value().copied()).collect::<Vec<_>>();
    assert_eq!(listed(Resample::entries(&words, &[Resample::Lanczos, Resample::Automatic])), [Resample::Automatic, Resample::Lanczos]);
    assert_eq!(listed(Resample::entries(&words, &[])), Resample::ALL);
}
