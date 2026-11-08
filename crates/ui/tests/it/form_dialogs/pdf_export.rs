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

/// The Summary lists each option that differs from the preset, as "Section › Setting: value".
#[trial]
fn the_summary_lists_the_differences_from_the_preset() {
    let (mut fixture, _) = open_on(PdfSettings { preserve_editing: true, ..PdfSettings::default() });
    go_to(&mut fixture, "Summary");
    assert!(fixture.texts().iter().any(|run| run.text.starts_with("Every option matches")), "the settings are the preset's");
    go_to(&mut fixture, "General");
    let fast = fixture.by_label("Optimize for fast web view").expect("the General page");
    fixture.click(&fast);
    fixture.settle();
    go_to(&mut fixture, "Summary");
    assert!(!fixture.texts().iter().any(|run| run.text.starts_with("Every option matches")), "something differs now");
    assert!(
        fixture.texts().iter().any(|run| run.text.starts_with("General › Optimize for fast web view: ")),
        "the changed option is listed: {:?}",
        fixture.texts().iter().map(|run| run.text.as_str()).collect::<Vec<_>>()
    );
}

/// Choosing PDF/X-1a moves the version to one it allows and turns off what its files can't have, which the Summary
/// then shows: the layers the settings opened with no longer differ from the preset, and editing data is off.
#[trial]
fn choosing_a_standard_applies_its_rules() {
    let settings = PdfSettings { preserve_editing: true, create_layers: true, compatibility: Compatibility::Pdf17, ..PdfSettings::default() };
    let (mut fixture, _) = open_on(settings);
    let lines = |fixture: &Fixture| fixture.texts().iter().map(|run| run.text.clone()).collect::<Vec<_>>();
    go_to(&mut fixture, "Summary");
    assert!(lines(&fixture).iter().any(|text| text.starts_with("General › Create PDF layers")), "the layers differ from the preset at first");
    go_to(&mut fixture, "General");
    let standard = fixture.control("Standard").expect("the Standard select");
    fixture.click(&standard);
    fixture.settle();
    let x1a = fixture.by_label(Standard::PdfX1a.label()).expect("the list offers PDF/X-1a");
    fixture.click(&x1a);
    fixture.settle();
    go_to(&mut fixture, "Summary");
    let lines = lines(&fixture);
    assert!(lines.iter().any(|text| text == Standard::PdfX1a.label()), "the standard shows in the Summary");
    assert!(lines.iter().any(|text| text == Standard::PdfX1a.version().label()), "the version moved to the one the standard allows");
    assert!(lines.iter().any(|text| text.starts_with("General › Preserve editing") && text.ends_with("Off")), "editing data is off: {lines:?}");
    assert!(!lines.iter().any(|text| text.starts_with("General › Create PDF layers")), "layers are off, as the preset has them: {lines:?}");
}
