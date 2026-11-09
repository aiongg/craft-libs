//! The export dialogs' own logic: the settings a document can take and what Export As answers.

use craft_behavior::PixelSize;
use craft_ui::contract::Bind;
use craft_ui::patterns::form_dialogs::{
    ExportAsDialog, ExportAsResult, ExportFormat, ExportOptions, ExportOptionsKind, ExportSize, FormatOptions, FormatOptionsDialog, HostRequests,
    JpegOptions,
};
use craft_ui::runtime::requests::{PathKind, PathRequest};
use craft_ui::testing::{Fixture, trial};

use egui::Modifiers;
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::Rc;

fn document() -> PixelSize {
    PixelSize::new(1200, 800)
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

/// The file name Export asks the host to save under for a document called `name`.
fn requested_file(name: &str) -> String {
    let name = name.to_owned();
    let asked = Rc::new(RefCell::new(None::<PathRequest>));
    let request = Rc::clone(&asked);
    let mut options = ExportOptions::default();
    let mut fixture = Fixture::new(move |ui| {
        let mut out = ExportAsDialog::new(name.clone(), document(), Bind::bound(&mut options)).show(ui);
        if let Some(filed) = out.requests.take_path() {
            *request.borrow_mut() = Some(filed);
        }
    });
    fixture.settle();
    let export = fixture.by_label("Export…").expect("the Export button");
    fixture.click(&export);
    fixture.settle();
    let filed = asked.borrow().clone().expect("Export files a save request");
    filed.file_name.expect("the request names a file")
}

/// The default file name is the document's, made safe on every platform: reserved characters become underscores, dots and
/// spaces at the ends go, and a name with nothing left is Untitled.
#[trial]
fn the_default_file_name_is_made_safe() {
    assert_eq!(requested_file("Poster"), "Poster.png");
    assert_eq!(requested_file("  ..My: poster/v2?.. "), "My_ poster_v2_.png");
    assert_eq!(requested_file(" . "), "Untitled.png");
    let long = "x".repeat(100);
    assert_eq!(requested_file(&long).len(), 80 + ".png".len(), "at most 80 characters");
}

/// The JPEG options dialog opened on `quality`, in a room that shows its rows.
fn jpeg_dialog(quality: u8, answer: Rc<RefCell<Option<FormatOptions>>>) -> Fixture {
    let mut fixture = Fixture::new(move |ui| {
        let shown = FormatOptionsDialog::new(ExportOptionsKind::Jpeg).initial(FormatOptions::Jpeg(JpegOptions::new(quality))).show(ui);
        if shown.result.is_some() {
            *answer.borrow_mut() = shown.result;
        }
    });
    fixture.settle();
    fixture
}

