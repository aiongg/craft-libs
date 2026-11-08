//! The export dialogs' own logic: file-name rules, the settings a document can take and the JPEG quality scale.

use craft_behavior::PixelSize;
use craft_ui::contract::Bind;
use craft_ui::patterns::form_dialogs::{
    ExportAsDialog, ExportAsResult, ExportFormat, ExportOptions, ExportSize, HostRequests, JpegPreset, PathKind, PathRequest, export_file_name,
    jpeg_quality_percent, jpeg_quality_preset, numbered_paths,
};
use craft_ui::testing::{Fixture, trial};
use egui::Modifiers;
use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// The longest name an export keeps, in characters.
const LONG_NAME: usize = 80;

fn document() -> PixelSize {
    PixelSize::new(1200, 800)
}

/// Reserved characters become `_`, the name is cut to 80 characters and trimmed, and an empty name becomes the
/// fallback.
#[trial]
fn a_file_name_loses_reserved_characters_and_is_trimmed() {
    assert_eq!(export_file_name("a<b>:c\"d/e\\f|g?h*i\u{7}j", "layer"), "a_b__c_d_e_f_g_h_i_j");
    assert_eq!(export_file_name("  ..name.. ", "layer"), "name");
    assert_eq!(export_file_name("...", "layer"), "layer");
    assert_eq!(export_file_name("", "Untitled"), "Untitled");
    assert_eq!(export_file_name(&"x".repeat(LONG_NAME + 20), "layer"), "x".repeat(LONG_NAME));
}

/// Later files of a multi-file export are numbered with their layer names and never collide with a file that exists.
#[trial]
fn numbered_paths_add_layer_names_and_avoid_collisions() {
    let first = Path::new("out/sheet.png");
    let taken = [PathBuf::from("out/sheet-2-sky.png"), PathBuf::from("out/sheet-2-sky-1.png")];
    let paths = numbered_paths(first, &["base", "sky", "a/b"], |path| taken.iter().any(|t| t == path));
    assert_eq!(paths, [PathBuf::from("out/sheet.png"), PathBuf::from("out/sheet-2-sky-2.png"), PathBuf::from("out/sheet-3-a_b.png")]);

    let bare = numbered_paths(Path::new(""), &["one", "two"], |_| false);
    assert_eq!(bare, [PathBuf::from(""), PathBuf::from("layers-2-two.png")]);
}

/// A remembered setting is made to fit the document: an unknown format falls back to the list's first, the quality
/// stays within 1 to 100, a linked pixel width takes its height from this document, an unlinked size stays when it is
/// in range, and a size the document cannot take becomes 100 percent.
#[trial]
fn remembered_options_are_fitted_to_the_document() {
    let formats = ExportFormat::standard();
    let mut remembered = ExportOptions::default();
    remembered.format = "bmp".to_owned();
    remembered.quality = 0;
    let fitted = remembered.fitted(&formats, document());
    assert_eq!((fitted.format.as_str(), fitted.quality), ("png", 1));

    let mut linked = ExportOptions::default();
    linked.size = ExportSize::pixels(600, 999);
    assert_eq!(linked.fitted(&formats, document()).size, ExportSize::pixels(600, 400));

    let mut unlinked = ExportOptions::default();
    unlinked.lock_aspect = false;
    unlinked.size = ExportSize::pixels(600, 999);
    assert_eq!(unlinked.clone().fitted(&formats, document()).size, ExportSize::pixels(600, 999));
    unlinked.size = ExportSize::pixels(0, 10);
    assert_eq!(unlinked.fitted(&formats, document()).size, ExportSize::Scale(100.0));

    let mut huge = ExportOptions::default();
    huge.size = ExportSize::Scale(5000.0);
    assert_eq!(huge.fitted(&formats, document()).size, ExportSize::Scale(100.0));
}

/// The output size follows the scale, rounding each side to at least one pixel, or is the pixel size given.
#[trial]
fn the_output_size_follows_the_scale_or_the_pixels() {
    let mut options = ExportOptions::default();
    options.size = ExportSize::Scale(50.0);
    assert_eq!(options.output_size(PixelSize::new(101, 3)), PixelSize::new(51, 2));
    options.size = ExportSize::pixels(640, 480);
    assert_eq!(options.output_size(document()), PixelSize::new(640, 480));
}

/// The JPEG scale of 0 to 12 maps onto the encoder's 1 to 100, and each quality falls in one of the four presets.
#[trial]
fn jpeg_quality_maps_to_percent_and_preset() {
    assert_eq!([0, 12, 13].map(jpeg_quality_percent), [1, 100, 100]);
    assert_eq!([3, 5, 8].map(jpeg_quality_percent), [26, 42, 67]);
    let presets: Vec<JpegPreset> = (0..=12).map(jpeg_quality_preset).collect();
    let low = [JpegPreset::Low; 5];
    let medium = [JpegPreset::Medium; 3];
    let high = [JpegPreset::High; 2];
    let maximum = [JpegPreset::Maximum; 3];
    assert_eq!(presets, [low.as_slice(), &medium, &high, &maximum].concat());
    assert!(presets.iter().all(|preset| jpeg_quality_preset(preset.quality()) == *preset), "a preset sets a quality inside itself");
}

/// Export files a save request for the file name and the format and keeps the dialog open; the host's path answer then
/// gives the result and closes the dialog.
#[trial]
fn export_then_a_path_answer_gives_the_result_and_closes() {
    let options = Rc::new(RefCell::new(ExportOptions::default()));
    let host = Rc::new(RefCell::new(HostRequests::new()));
    let asked = Rc::new(RefCell::new(None::<PathRequest>));
    let result = Rc::new(RefCell::new(None::<ExportAsResult>));
    let closed = Rc::new(Cell::new(false));
    let (held, requests, request, answer, done) = (Rc::clone(&options), Rc::clone(&host), Rc::clone(&asked), Rc::clone(&result), Rc::clone(&closed));
    let mut fixture = Fixture::new(move |ui| {
        let mut options = held.borrow_mut();
        let answers = requests.borrow_mut().take_answers();
        let mut out = ExportAsDialog::new("Poster", document(), Bind::bound(&mut *options)).answers(answers).show(ui);
        if let Some(filed) = out.requests.take_path() {
            *request.borrow_mut() = Some(filed);
        }
        if out.result.is_some() {
            *answer.borrow_mut() = out.result;
        }
        if out.closed.is_some() {
            done.set(true);
        }
    });
    fixture.settle();
    let export = fixture.by_label("Export…").expect("the Export button");
    fixture.click(&export);
    fixture.settle();
    let filed = asked.borrow().clone().expect("Export files a save request");
    assert_eq!((filed.kind, filed.file_name.as_deref()), (PathKind::SaveFile, Some("Poster.png")));
    assert_eq!(filed.filters.first().map(|filter| filter.extensions.clone()), Some(vec!["png".to_owned()]));
    assert!(!closed.get() && result.borrow().is_none(), "the dialog waits for the host's path");

    host.borrow_mut().answer_path(filed.key, PathBuf::from("out/Poster.png"));
    fixture.settle();
    assert!(closed.get(), "the path answer closes the dialog");
    let done = result.borrow().clone().expect("the answer carries the result");
    assert_eq!((done.path, done.size), (PathBuf::from("out/Poster.png"), document()));
}

/// With Alt held the second button is Reset: it restores the opening settings and keeps the dialog open, as for any
/// form dialog.
#[trial]
fn alt_turns_cancel_into_reset_which_keeps_export_open() {
    let options = Rc::new(RefCell::new(ExportOptions::default()));
    let closed = Rc::new(Cell::new(false));
    let (held, done) = (Rc::clone(&options), Rc::clone(&closed));
    let mut fixture = Fixture::new(move |ui| {
        let mut options = held.borrow_mut();
        let out = ExportAsDialog::new("Poster", document(), Bind::bound(&mut *options)).show(ui);
        if out.closed.is_some() {
            done.set(true);
        }
    });
    fixture.hold(Modifiers::ALT);
    fixture.settle();
    assert!(fixture.by_label("Reset").is_some() && fixture.by_label("Cancel").is_none(), "Reset with Alt");
    let opening = options.borrow().quality;
    options.borrow_mut().quality = opening - 1;
    fixture.idle(3);
    let reset = fixture.by_label("Reset").expect("the Reset button");
    fixture.click(&reset);
    fixture.settle();
    assert_eq!(options.borrow().quality, opening, "Reset restores the opening settings");
    assert!(!closed.get(), "Reset keeps the dialog open");
}
