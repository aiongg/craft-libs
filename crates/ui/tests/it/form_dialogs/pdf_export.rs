//! The Save PDF dialog's own logic: the artboard range, the Summary's differences, the standards' rules and the
//! wording of the checks.

use craft_pdf::settings::{ColorConversion, Compatibility, OutputSettings, PdfSettings, SecuritySettings, SettingsError, Standard, builtin_preset};
use craft_ui::patterns::form_dialogs::{PdfExportWords, RangeError, apply_standard, check_range, differences};
use craft_ui::runtime::env::WordSet as _;
use craft_ui::testing::trial;

/// The artboards of the document the ranges are checked against.
const ARTBOARDS: usize = 5;

fn refused(text: &str) -> Option<RangeError> {
    check_range(text, ARTBOARDS).err()
}

/// A range is numbers and spans of numbers inside the document; open ends run to the first or last artboard.
#[trial]
fn a_range_names_artboards_of_the_document() {
    for accepted in ["1-3, 5", "3-", "All"] {
        assert_eq!(refused(accepted), None, "{accepted}");
    }
    assert!(matches!(refused("6"), Some(RangeError::NotANumber { part, .. }) if part == "6"));
    assert!(matches!(refused("+2"), Some(RangeError::NotANumber { part, .. }) if part == "+2"));
    assert!(matches!(refused("4-2"), Some(RangeError::Backwards { part, .. }) if part == "4-2"));
    assert!(matches!(refused("-"), Some(RangeError::NoNumber { part, .. }) if part == "-"));
    assert_eq!(refused(" , "), Some(RangeError::Empty));
}

/// The Summary lists every option that differs from the preset, in the words of its row, and nothing for equal
/// settings.
#[trial]
fn the_summary_lists_options_that_differ_from_the_preset() {
    let words = PdfExportWords::english();
    let (Some(base), Some(smallest)) = (builtin_preset("Test Default", "Test"), builtin_preset("Smallest File Size", "Test")) else {
        panic!("the built-in presets exist");
    };
    let lines = differences(&words, &smallest.settings, &base.settings);
    for expected in [
        "General › Optimize for fast web view: On",
        "Compression › Color images › Downsampling: Bicubic",
        "General › Preserve editing capabilities: Off",
    ] {
        assert!(lines.iter().any(|line| line == expected), "{expected} in {lines:?}");
    }
    assert!(differences(&words, &base.settings, &base.settings).is_empty());

    let mut typed = base.settings.clone();
    typed.security.open_password = "secret".to_owned();
    assert!(differences(&words, &typed, &base.settings).is_empty(), "passwords are not compared");
}

/// A standard sets the version it allows and turns off what its files cannot have.
#[trial]
fn a_pdf_x_standard_forces_its_rows() {
    let mut settings = PdfSettings {
        standard: Standard::PdfX1a,
        preserve_editing: true,
        create_layers: true,
        security: SecuritySettings { open_password: "secret".to_owned(), ..SecuritySettings::default() },
        ..PdfSettings::default()
    };
    assert_eq!(settings.compatibility, Compatibility::Pdf17);
    apply_standard(&mut settings);
    assert_eq!(settings.compatibility, Compatibility::Pdf13);
    assert!(!settings.preserve_editing && !settings.create_layers);
    assert!(settings.security.open_password.is_empty());
}

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
