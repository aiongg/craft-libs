//! The Save PDF dialog's own logic: the artboard range, the Summary's differences, the standards' rules and the
//! wording of the checks.

use craft_behavior::Mixed;
use craft_pdf::settings::{ColorConversion, Compatibility, OutputSettings, PdfSettings, SettingsError, Standard};
use craft_ui::contract::{Bind, Role};
use craft_ui::patterns::form_dialogs::{PdfExportDialog, PdfExportWords};
use craft_ui::runtime::env::WordSet as _;
use craft_ui::testing::{Fixture, trial};
use egui::{Modifiers, Vec2};
use std::cell::RefCell;
use std::rc::Rc;

/// The artboards of the document the ranges are checked against.
const ARTBOARDS: usize = 5;

/// Alt-Reset restores what the dialog keeps beside the settings: the artboard choice and the password switch.
#[trial]
fn alt_reset_restores_the_artboard_choice_and_the_password_switch() {
    let settings = Rc::new(RefCell::new(PdfSettings::default()));
    let held = Rc::clone(&settings);
    let mut fixture = Fixture::new(move |ui| {
        let mut open = true;
        let mut settings = held.borrow_mut();
        PdfExportDialog::new("Test", Bind::bound(&mut *settings)).opened(Bind::bound(&mut open)).artboard_count(ARTBOARDS).show(ui);
    });
    fixture.settle();
    let radios = fixture.by_role(Role::RADIO_BUTTON);
    assert_eq!(radios.len(), 2, "All and Range");
    fixture.click(&radios[1]);
    fixture.settle();
    let fast = fixture.by_label("Optimize for fast web view").expect("the General page is shown");
    fixture.click(&fast);
    fixture.settle();
    let security = fixture.by_label("Security").expect("the section list is shown");
    fixture.click(&security);
    fixture.settle();
    let require = fixture.by_label("Require a password to open the document").expect("the Security page is shown");
    fixture.click(&require);
    fixture.settle();
    let toggled = |fixture: &Fixture, label: &str| {
        let target = fixture.by_label(label).expect("the control is shown");
        fixture.tree().toggled_of(target.node)
    };
    assert_eq!(toggled(&fixture, "Require a password to open the document"), Some(Mixed::Same(true)));

    fixture.hold(Modifiers::ALT);
    fixture.settle();
    let reset = fixture.by_label("Reset").expect("Alt turns Cancel into Reset");
    fixture.click(&reset);
    fixture.settle();
    fixture.hold(Modifiers::NONE);
    fixture.settle();
    assert_eq!(toggled(&fixture, "Require a password to open the document"), Some(Mixed::Same(false)), "Reset cleared the switch");
    let general = fixture.by_label("General").expect("the section list is shown");
    fixture.click(&general);
    fixture.settle();
    let radios = fixture.by_role(Role::RADIO_BUTTON);
    assert_eq!(fixture.tree().toggled_of(radios[0].node), Some(Mixed::Same(true)), "Reset chose All again");
}

/// A fixture over the dialog on `settings`, in a room that shows a whole page of the dialog.
fn open_on(settings: PdfSettings) -> (Fixture, Rc<RefCell<PdfSettings>>) {
    let settings = Rc::new(RefCell::new(settings));
    let held = Rc::clone(&settings);
    let mut fixture = Fixture::new(move |ui| {
        let mut open = true;
        let mut settings = held.borrow_mut();
        PdfExportDialog::new("Test", Bind::bound(&mut *settings)).opened(Bind::bound(&mut open)).artboard_count(ARTBOARDS).show(ui);
    });
    fixture.room(Vec2::new(1100.0, 900.0));
    fixture.settle();
    (fixture, settings)
}

fn go_to(fixture: &mut Fixture, section: &str) {
    let target = fixture.by_label(section).unwrap_or_else(|| panic!("the section list shows {section}"));
    fixture.click(&target);
    fixture.settle();
}

fn shows(fixture: &Fixture, text: &str) -> bool {
    fixture.texts().iter().any(|run| run.text == text)
}

/// A typed range names artboards the document has or the dialog says why not, in the words of the check that failed.
#[trial]
fn an_artboard_range_the_document_does_not_have_shows_its_error() {
    let (mut fixture, _) = open_on(PdfSettings::default());
    let radios = fixture.by_role(Role::RADIO_BUTTON);
    fixture.click(&radios[1]);
    fixture.settle();
    let range = |fixture: &Fixture| {
        fixture.by_role(Role::TEXT_INPUT).into_iter().find(|field| fixture.tree().name_of(field.node) == Some("Range")).expect("the range field")
    };
    let type_range = |fixture: &mut Fixture, text: &str| {
        let field = range(fixture);
        fixture.click(&field);
        fixture.key(egui::Key::A, Modifiers::COMMAND);
        fixture.type_text(text);
        fixture.settle();
    };
    type_range(&mut fixture, "1-3, 5");
    assert!(!fixture.texts().iter().any(|run| run.text.starts_with("Range `")), "a range inside the document shows no error");
    type_range(&mut fixture, "9");
    assert!(shows(&fixture, "Range `9`: `9` is not a number from 1 to 5"), "out of bounds");
    type_range(&mut fixture, "4-2");
    assert!(shows(&fixture, "Range `4-2`: `4-2` runs backwards"), "backwards");
}

