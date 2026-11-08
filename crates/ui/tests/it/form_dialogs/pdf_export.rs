//! The Save PDF dialog's own logic: the artboard range, the Summary's differences, the standards' rules and the
//! wording of the checks.

use craft_behavior::Mixed;
use craft_pdf::settings::{ColorConversion, OutputSettings, PdfSettings, SettingsError};
use craft_ui::contract::{Bind, Role};
use craft_ui::patterns::form_dialogs::{PdfExportDialog, PdfExportWords};
use craft_ui::runtime::env::WordSet as _;
use craft_ui::testing::{Fixture, trial};
use egui::Modifiers;
use std::cell::RefCell;
use std::rc::Rc;

/// The artboards of the document the ranges are checked against.
const ARTBOARDS: usize = 5;

/// A refused setting is worded from the values the check reports; an unknown rule keeps craft-pdf's message.
#[trial]
fn check_messages_come_from_words_with_their_values() {
    let words = PdfExportWords::english();
    let mut settings = PdfSettings::default();
    settings.compression.color.ppi = 5.0;
    let error = settings.check_values().expect_err("a resolution of 5 ppi is refused");
    assert_eq!(words.error_text(&error), "Compression › Color images › to must be 9–2400 ppi (got 5)");

    let settings = PdfSettings {
        output: OutputSettings { conversion: ColorConversion::Destination, destination: "Nowhere".to_owned(), ..OutputSettings::default() },
        ..PdfSettings::default()
    };
    let error = settings.check(&()).expect_err("no profile is called Nowhere");
    assert_eq!(words.error_text(&error), "Destination: no RGB or CMYK profile is called “Nowhere”");

    let unknown = SettingsError { id: "newRule", message: "a new rule".to_owned(), values: Vec::new() };
    assert_eq!(words.error_text(&unknown), "a new rule");
}

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
