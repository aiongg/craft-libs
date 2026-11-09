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
