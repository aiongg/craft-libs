//! Save PDF in a harness.
#![allow(clippy::unwrap_used, reason = "tests: a missing dialog is a failed test")]

use std::path::PathBuf;
use std::sync::Arc;

use craft_host::{AppInfo, Host, Picked};
use craft_pdf::settings::{ColorMode, Compatibility, PdfSettings, ProfileCatalog, ProfileKind, SecuritySettings, Standard, builtin_preset};
use egui::accesskit::Role;
use egui::vec2;
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::super::web_tests::StubFiles;
use super::{PdfArtboards, PdfExportDialog, PdfExportResult};
use crate::dialog::{DialogHost, DialogResult};
use crate::theme::{Base, install};

struct Profiles;

impl ProfileCatalog for Profiles {
    fn kind(&self, name: &str) -> Option<ProfileKind> {
        match name {
            "sRGB" => Some(ProfileKind::Rgb),
            "Coated CMYK" => Some(ProfileKind::Cmyk),
            _ => None,
        }
    }
    fn rgb(&self) -> &str {
        "sRGB"
    }
    fn cmyk(&self) -> &str {
        "Coated CMYK"
    }
}

struct App {
    host: DialogHost,
    answers: Vec<DialogResult<PdfExportResult>>,
}

fn dialog() -> PdfExportDialog {
    PdfExportDialog::new("Test", "poster").profiles(vec!["sRGB".to_owned(), "Coated CMYK".to_owned()], Arc::new(Profiles)).color_mode(ColorMode::Rgb)
}

fn harness(dialog: PdfExportDialog) -> (Harness<'static, App>, Arc<StubFiles>) {
    let files = Arc::new(StubFiles::default());
    let stub = files.clone();
    let mut harness = HarnessBuilder::default()
        .with_size(vec2(1000.0, 900.0))
        .with_context_setup(move |ctx| {
            install(ctx, &Base::suite_default());
            Host::builder(AppInfo { id: "test", name: "Test", version: "1.0" }).files(stub).install(ctx);
        })
        .build_ui_state(
            |ui, app: &mut App| {
                app.host.show(ui.ctx());
                if let Some(answer) = app.host.take::<PdfExportDialog>() {
                    app.answers.push(answer);
                }
            },
            App { host: DialogHost::default(), answers: Vec::new() },
        );
    harness.state_mut().host.open(dialog);
    harness.run_steps(4);
    (harness, files)
}

fn pdf<'a>(harness: &'a mut Harness<'static, App>) -> &'a mut PdfExportDialog {
    harness.state_mut().host.get_mut::<PdfExportDialog>().unwrap()
}

fn choose(harness: &mut Harness<'static, App>, list: &str, option: &str) {
    harness.get_by_role_and_label(Role::ComboBox, list).click();
    harness.run_steps(4);
    harness.get_by_label(option).click();
    harness.run_steps(4);
}

fn section(harness: &mut Harness<'static, App>, name: &str) {
    harness.get_by_label(name).click();
    harness.run_steps(4);
}

fn save(harness: &mut Harness<'static, App>) {
    harness.get_by_role_and_label(Role::Button, "Save PDF").click();
    harness.run_steps(4);
}

// Choosing a preset fills every row with its settings.
#[test]
fn a_preset_choice_fills_the_rows() {
    let (mut harness, _) = harness(dialog());
    assert!(pdf(&mut harness).settings().preserve_editing, "the app default keeps the document editable");
    choose(&mut harness, "Preset", "Smallest File Size");
    let smallest = builtin_preset("Smallest File Size", "Test").unwrap().settings;
    assert_eq!(pdf(&mut harness).settings(), &smallest);
    assert!(harness.query_by_role_and_label(Role::ComboBox, "Compatibility").is_some());
    section(&mut harness, "Compression");
    assert_eq!(harness.query_all_by_role_and_label(Role::SpinButton, "to").count(), 3, "colour, grey and monochrome resolutions");
}

// A PDF/X standard sets its version and turns off editing data, PDF layers and passwords.
#[test]
fn a_pdf_x_standard_forces_its_rows() {
    let settings = PdfSettings {
        preserve_editing: true,
        create_layers: true,
        security: SecuritySettings { open_password: "secret".to_owned(), ..SecuritySettings::default() },
        ..PdfSettings::default()
    };
    let (mut harness, _) = harness(dialog().initial(settings));
    assert_eq!(pdf(&mut harness).settings().compatibility, Compatibility::Pdf17);
    choose(&mut harness, "Standard", "PDF/X-1a:2001");
    let settings = pdf(&mut harness).settings();
    assert_eq!(settings.standard, Standard::PdfX1a);
    assert_eq!(settings.compatibility, Compatibility::Pdf13);
    assert!(!settings.preserve_editing && !settings.create_layers);
    assert!(settings.security.open_password.is_empty());
    assert!(harness.query_by_label("Preserve editing capabilities").is_some_and(|node| node.accesskit_node().is_disabled()));
}

// A failing check shows its message and disables Save PDF, so no picker opens.
#[test]
fn a_failing_check_blocks_save_with_its_message() {
    let settings = PdfSettings {
        security: SecuritySettings { open_password: "same".to_owned(), permissions_password: "same".to_owned(), ..SecuritySettings::default() },
        ..PdfSettings::default()
    };
    let (mut harness, files) = harness(dialog().initial(settings));
    assert!(harness.query_by_label("The open and permissions passwords must differ").is_some());
    assert!(harness.get_by_role_and_label(Role::Button, "Save PDF").accesskit_node().is_disabled());
    save(&mut harness);
    assert!(files.requests().is_empty());
}

// The password rows enable with their check box, and Save PDF returns what was typed, the
// artboard range and the picked path.
#[test]
fn security_rows_follow_the_password_box_and_the_result_carries_the_settings() {
    let (mut harness, files) = harness(dialog().artboards(PdfArtboards::Range("1-3, 5".to_owned())));
    section(&mut harness, "Security");
    let field = |harness: &Harness<'static, App>| harness.get_by_role_and_label(Role::PasswordInput, "Open password").accesskit_node().is_disabled();
    assert!(field(&harness), "no password asked for");
    harness.get_by_label("Require a password to open the document").click();
    harness.run_steps(4);
    assert!(!field(&harness));
    harness.get_by_role_and_label(Role::PasswordInput, "Open password").click();
    harness.run_steps(2);
    harness.get_by_role_and_label(Role::PasswordInput, "Open password").type_text("secret");
    harness.run_steps(4);
    assert_eq!(pdf(&mut harness).settings().security.open_password, "secret");

    save(&mut harness);
    let requests = files.requests();
    assert_eq!((requests[0].purpose.as_str(), requests[0].file_name.as_deref()), ("pdf", Some("poster.pdf")));
    files.answer(1, Picked::Paths(vec![PathBuf::from("/out/poster.pdf")]));
    harness.run_steps(4);
    let [DialogResult::Ok(result)] = harness.state().answers.as_slice() else { panic!("one answer expected") };
    assert_eq!(result.path, PathBuf::from("/out/poster.pdf"));
    assert_eq!(result.options.security.open_password, "secret");
    assert_eq!(result.artboards, PdfArtboards::Range("1-3, 5".to_owned()));
}

// Save Preset… names the settings; the preset reaches the app without the passwords.
#[test]
fn save_preset_reports_the_named_preset_without_passwords() {
    let settings =
        PdfSettings { security: SecuritySettings { open_password: "secret".to_owned(), ..SecuritySettings::default() }, ..PdfSettings::default() };
    let (mut harness, _) = harness(dialog().initial(settings));
    harness.get_by_label("Save Preset…").click();
    harness.run_steps(4);
    harness.get_by_role_and_label(Role::TextInput, "Save as preset").click();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.get_by_role_and_label(Role::TextInput, "Save as preset").type_text("Proof");
    harness.run_steps(4);
    harness.get_by_role_and_label(Role::Button, "Save").click();
    harness.run_steps(4);
    let preset = pdf(&mut harness).take_preset().expect("the preset is reported");
    assert_eq!(preset.name, "Proof");
    assert!(preset.settings.security.open_password.is_empty());
    assert!(pdf(&mut harness).take_preset().is_none(), "taken once");
}

// Every refused setting is worded by its id, with the values it carries; a translator's words
// replace the English, and craft-pdf's message stays only for an unknown id.
#[test]
fn check_messages_come_from_words_with_their_values() {
    use craft_pdf::settings::SettingsError;

    use super::words::PdfExportLabels;

    let labels = PdfExportLabels::default();
    let mut settings = PdfSettings::default();
    settings.compression.color.ppi = 5.0;
    let error = settings.check_values().unwrap_err();
    assert_eq!(labels.error_text(&error, &settings), "Compression › Color images › to must be 9–2400 ppi (got 5)");

    let settings = PdfSettings {
        output: craft_pdf::settings::OutputSettings {
            conversion: craft_pdf::settings::ColorConversion::Destination,
            destination: "Nowhere".to_owned(),
            ..Default::default()
        },
        ..PdfSettings::default()
    };
    let error = settings.check(&Profiles).unwrap_err();
    assert_eq!(labels.error_text(&error, &settings), "Destination: no RGB or CMYK profile is called “Nowhere”");

    let settings = PdfSettings {
        compatibility: Compatibility::Pdf14,
        security: SecuritySettings { open_password: "é".repeat(20), ..SecuritySettings::default() },
        ..PdfSettings::default()
    };
    let error = settings.check_values().unwrap_err();
    assert_eq!(labels.error_text(&error, &settings), "PDF 1.4 passwords can have up to 32 ASCII characters (PDF 1.7 and later take any)");

    let translated = PdfExportLabels::default().localized(|key| (key == "pdfExport.checkPasswordLegacy").then(|| "{version}: {limit}".to_owned()));
    assert_eq!(translated.error_text(&error, &settings), "PDF 1.4: 32");
    let unknown = SettingsError { id: "newRule", message: "a new rule".to_owned() };
    assert_eq!(labels.error_text(&unknown, &settings), "a new rule");
}

// The Summary lists the options that differ from the chosen preset, and says so when none do.
#[test]
fn the_summary_lists_options_that_differ_from_the_preset() {
    use super::summary::changes;
    use super::words::PdfExportLabels;

    let labels = PdfExportLabels::default();
    let base = builtin_preset("Test Default", "Test").unwrap().settings;
    let smallest = builtin_preset("Smallest File Size", "Test").unwrap().settings;
    let lines = changes(&labels, &smallest, &base);
    assert!(lines.contains(&"General › Optimize for fast web view: On".to_owned()), "{lines:?}");
    assert!(lines.contains(&"Compression › Color images › Downsampling: Bicubic".to_owned()), "{lines:?}");
    assert!(lines.contains(&"General › Preserve editing capabilities: Off".to_owned()), "{lines:?}");
    assert!(changes(&labels, &base, &base).is_empty());

    let (mut harness, _) = harness(dialog());
    section(&mut harness, "Summary");
    assert!(harness.query_by_label("Every option matches Test Default.").is_some());
    pdf(&mut harness).settings_mut_for_test().thumbnails = true;
    harness.run_steps(4);
    assert!(harness.query_by_label("General › Embed page thumbnails: On").is_some());
}

// A range outside the artboards disables Save PDF with VectorCraft's message.
#[test]
fn a_bad_artboard_range_blocks_save_with_its_message() {
    use super::summary::{RangeError, check_range};

    assert_eq!(check_range("1-3, 5", 5), Ok(()));
    assert_eq!(check_range("3-", 5), Ok(()));
    assert_eq!(check_range("All", 5), Ok(()));
    assert_eq!(check_range("6", 5), Err(RangeError::NotANumber { part: "6".to_owned() }));
    assert_eq!(check_range("+2", 5), Err(RangeError::NotANumber { part: "+2".to_owned() }));
    assert_eq!(check_range("4-2", 5), Err(RangeError::Backwards { part: "4-2".to_owned() }));
    assert_eq!(check_range("-", 5), Err(RangeError::NoNumber { part: "-".to_owned() }));
    assert_eq!(check_range(" , ", 5), Err(RangeError::Empty));

    let (mut harness, files) = harness(dialog().artboard_count(3).artboards(PdfArtboards::Range("1-9".to_owned())));
    assert!(harness.query_by_label("Range `1-9`: `9` is not a number from 1 to 3").is_some());
    assert!(harness.get_by_role_and_label(Role::Button, "Save PDF").accesskit_node().is_disabled());
    save(&mut harness);
    assert!(files.requests().is_empty());
}
