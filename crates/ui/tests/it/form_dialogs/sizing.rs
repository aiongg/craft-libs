//! The size dialogs' own logic: the answer at OK, the refusal of an empty size, Alt-Reset, the pixel fields with
//! Resample off and the method list.

use craft_behavior::PixelSize;
use craft_ui::contract::Bind;
use craft_ui::contract::Role;
use craft_ui::dialog::{DialogHost, DialogResult};
use craft_ui::patterns::form_dialogs::{
    CanvasExtension, CanvasSize, CanvasSizeValues, HostedImageSize, ImageSize, ImageSizeValues, Resample, SizingWords,
};
use craft_ui::pickers::Anchor9;
use craft_ui::runtime::env::UnitKind;
use craft_ui::runtime::env::WordSet as _;
use craft_ui::testing::{Fixture, trial};
use egui::{Key, Modifiers};
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

/// A fixture showing Image Size over `values`, which the test keeps.
fn image_fixture(values: &Rc<RefCell<ImageSizeValues>>) -> Fixture {
    let held = Rc::clone(values);
    let mut fixture = Fixture::new(move |ui| {
        let mut values = held.borrow_mut();
        ImageSize::new(Bind::bound(&mut *values)).opened(Bind::given(true)).show(ui);
    });
    fixture.settle();
    fixture
}

/// OK answers the values the dialog opened with: the size, the resolution, the units and the method.
#[trial]
fn image_size_answers_the_opening_values() {
    let values = Rc::new(RefCell::new(ImageSizeValues::new(PICTURE).pixels_per_inch(RESOLUTION).method(Resample::Lanczos)));
    let held = Rc::clone(&values);
    let (mut fixture, answer, open) = rig(move |open, ui| {
        let mut values = held.borrow_mut();
        ImageSize::new(Bind::bound(&mut *values)).opened(open).show(ui).result
    });
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
    let values = Rc::new(RefCell::new(ImageSizeValues::new(PICTURE).resampling(false)));
    let (mut fixture, answer, _) = rig(move |open, ui| {
        let mut values = values.borrow_mut();
        ImageSize::new(Bind::bound(&mut *values)).opened(open).show(ui).result
    });
    press_ok(&mut fixture);
    assert_eq!(answer.borrow().as_ref().map(|result| result.resample), Some(None));
}

/// An empty size cannot be answered: OK leaves the dialog open and answers nothing.
#[trial]
fn an_empty_size_refuses_ok() {
    let values = Rc::new(RefCell::new(ImageSizeValues::new(PixelSize::new(0, 900))));
    let (mut fixture, answer, open) = rig(move |open, ui| {
        let mut values = values.borrow_mut();
        ImageSize::new(Bind::bound(&mut *values)).opened(open).show(ui).result
    });
    press_ok(&mut fixture);
    assert!(answer.borrow().is_none(), "no answer for an empty size");
    assert!(open.get(), "the dialog stays open");
}

/// A document with no background answers Transparent whatever extension the dialog was given.
#[trial]
fn canvas_size_without_a_background_answers_transparent() {
    let values =
        Rc::new(RefCell::new(CanvasSizeValues::new(PICTURE).extension(CanvasExtension::White).has_background(false).anchor(Anchor9::TopLeft)));
    let (mut fixture, answer, _) = rig(move |open, ui| {
        let mut values = values.borrow_mut();
        CanvasSize::new(Bind::bound(&mut *values)).opened(open).show(ui).result
    });
    press_ok(&mut fixture);
    let result = answer.borrow_mut().take().expect("OK answers");
    assert_eq!(result.extension, CanvasExtension::Transparent);
    assert_eq!(result.anchor, Anchor9::TopLeft);
    assert_eq!(result.size, PICTURE);
}

/// Alt turns Cancel into Reset, which puts back the values the dialog opened with.
#[trial]
fn alt_reset_in_image_size_restores_the_opening_values() {
    let opening = ImageSizeValues::new(PICTURE);
    let values = Rc::new(RefCell::new(opening.clone()));
    let mut fixture = image_fixture(&values);
    let resample = fixture.by_label("Resample").expect("the Resample box is shown");
    fixture.click(&resample);
    fixture.settle();
    assert_ne!(*values.borrow(), opening, "the edit wrote through");
    fixture.hold(Modifiers::ALT);
    fixture.settle();
    let reset = fixture.by_label("Reset").expect("Alt turns Cancel into Reset");
    fixture.click(&reset);
    fixture.settle();
    assert_eq!(*values.borrow(), opening, "Reset wrote the opening values back");
}

/// Typing a width changes the size with Resample on and does nothing with Resample off, where the pixel count is
/// fixed.
#[trial]
fn resample_off_disables_the_pixel_fields() {
    let typed = |resampling: bool| {
        let opening = ImageSizeValues::new(PICTURE).resampling(resampling);
        let values = Rc::new(RefCell::new(opening.clone()));
        let mut fixture = image_fixture(&values);
        let width = fixture.by_role(Role::SPIN_BUTTON).into_iter().next().expect("the width field is shown");
        fixture.click(&width);
        fixture.key(Key::A, Modifiers::COMMAND);
        fixture.type_text("200");
        fixture.key(Key::Enter, Modifiers::NONE);
        fixture.settle();
        let changed = *values.borrow() != opening;
        changed
    };
    assert!(typed(true), "with Resample on the width takes the typed number");
    assert!(!typed(false), "with Resample off the width field takes no input");
}

/// The method list keeps the order of the full list whatever order the app names the methods in.
#[trial]
fn the_method_list_follows_the_full_list() {
    let words = SizingWords::english();
    let listed = |entries: Vec<craft_ui::contract::Entry<Resample>>| entries.iter().filter_map(|entry| entry.value().copied()).collect::<Vec<_>>();
    assert_eq!(listed(Resample::entries(&words, &[Resample::Lanczos, Resample::Automatic])), [Resample::Automatic, Resample::Lanczos]);
    assert_eq!(listed(Resample::entries(&words, &[])), Resample::ALL);
}

/// A hosted Image Size opens on the document's size, and OK hands the edited size back through the host.
#[trial]
fn a_hosted_image_size_shows_the_document_size_and_answers_through_the_host() {
    const DOCUMENT: PixelSize = PixelSize::new(1200, 800);
    let host = Rc::new(RefCell::new(DialogHost::new()));
    host.borrow_mut().open(HostedImageSize::new(ImageSizeValues::new(DOCUMENT), |values| ImageSize::new(values)));
    let shown = Rc::clone(&host);
    let mut fixture = Fixture::new(move |ui| shown.borrow_mut().run(ui));
    fixture.settle();
    assert!(fixture.texts().iter().any(|run| run.text.contains("1200")), "the width shows the document's width");
    assert!(fixture.texts().iter().any(|run| run.text.contains("800")), "the height shows the document's height");

    let width = fixture.by_label("Width").expect("the width field is shown");
    fixture.click(&width);
    fixture.key(Key::A, Modifiers::COMMAND);
    fixture.type_text("600");
    press_ok(&mut fixture);
    let Some(DialogResult::Ok(answer)) = host.borrow_mut().take::<HostedImageSize>() else { panic!("OK answers through the host") };
    assert_eq!(answer.size.width, 600, "the answer carries the edited width");
}
