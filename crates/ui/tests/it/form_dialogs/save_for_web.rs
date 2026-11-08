//! The own logic of Save for Web: which panes the dialog asks the app to encode, and the Save… hand-off.

use craft_behavior::PixelSize;
use craft_ui::contract::Bind;
use craft_ui::patterns::form_dialogs::{
    HostRequests, SAVE_PATH_KEY, SaveForWebDialog, SaveForWebOptions, SaveForWebResult, WantedVariant, WebPane, WebPictures, WebPreset, WebView,
};
use craft_ui::runtime::overlay::CloseReason;
use craft_ui::runtime::requests::PathRequest;
use craft_ui::testing::{Fixture, trial};

use egui::Modifiers;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

const DOCUMENT: PixelSize = PixelSize::new(1200, 800);

/// The opening settings changed by `change`.
fn options(change: impl FnOnce(&mut SaveForWebOptions)) -> SaveForWebOptions {
    let mut options = SaveForWebOptions::default();
    change(&mut options);
    options
}

/// A preset is what the settings show apart from the size.
#[trial]
fn a_preset_matches_the_settings_apart_from_the_size() {
    let preset = WebPreset::new("JPEG Medium", SaveForWebOptions::default());
    assert!(preset.is_set_in(&options(|options| options.percent = 50.0)));
    assert!(!preset.is_set_in(&options(|options| options.quality = 61)));
    assert_eq!(WebView::FourUp.panes().len(), 4);
    assert_eq!(WebView::Optimized.panes(), &[WebPane::Encoded(0)]);
}

struct Rig {
    fixture: Fixture,
    asked: Rc<RefCell<Vec<WantedVariant>>>,
    requested: Rc<RefCell<Option<PathRequest>>>,
    answered: Rc<RefCell<Option<SaveForWebResult>>>,
    closed: Rc<RefCell<Option<CloseReason>>>,
    host: Rc<RefCell<HostRequests>>,
}

fn rig() -> Rig {
    let asked = Rc::new(RefCell::new(Vec::new()));
    let requested = Rc::new(RefCell::new(None));
    let answered = Rc::new(RefCell::new(None));
    let closed = Rc::new(RefCell::new(None));
    let host = Rc::new(RefCell::new(HostRequests::new()));
    let (wanted, request, result, ended, answers) =
        (Rc::clone(&asked), Rc::clone(&requested), Rc::clone(&answered), Rc::clone(&closed), Rc::clone(&host));
    let mut fixture = Fixture::new(move |ui| {
        let mut open = true;
        let answers = answers.borrow_mut().take_answers();
        let mut shown =
            SaveForWebDialog::new("poster.psd", DOCUMENT, WebPictures::new(), Bind::kept()).opened(Bind::bound(&mut open)).answers(answers).show(ui);
        wanted.borrow_mut().extend(shown.wanted_variants.iter().cloned());
        if let Some(path) = shown.requests.take_path() {
            *request.borrow_mut() = Some(path);
        }
        if shown.result.is_some() {
            *result.borrow_mut() = shown.result;
        }
        if shown.closed.is_some() {
            *ended.borrow_mut() = shown.closed;
        }
    });
    fixture.settle();
    Rig { fixture, asked, requested, answered, closed, host }
}

/// The dialog asks for the panes its view shows once per change, with a new revision each time, and the first pass
/// asks for the current settings alone (2-Up).
#[trial]
fn the_dialog_asks_for_the_panes_of_the_view_once_per_change() {
    let mut rig = rig();
    {
        let asked = rig.asked.borrow();
        assert_eq!(asked.len(), 1, "2-Up asks for the current settings only");
        assert_eq!((asked[0].index, asked[0].request.revision), (0, 1));
        assert_eq!(asked[0].request.options, SaveForWebOptions::default());
    }
    rig.fixture.idle(3);
    assert_eq!(rig.asked.borrow().len(), 1, "nothing changed, nothing asked");

    let four_up = rig.fixture.by_label("4-Up").expect("the view tabs are shown");
    rig.fixture.click(&four_up);
    rig.fixture.idle(3);
    let asked = rig.asked.borrow();
    let latest: Vec<_> = asked.iter().filter(|want| want.request.revision == 2).collect();
    assert_eq!(latest.iter().map(|want| want.index).collect::<Vec<_>>(), vec![0, 1, 2], "4-Up asks for three panes");
    assert_eq!(latest[1].request.options.quality, SaveForWebOptions::default().quality / 2);
}

/// Save… files the file dialog request for the document's name with the format's extension and leaves the dialog
/// open; the path the host answers with makes the result and closes the dialog.
#[trial]
fn save_files_the_picker_and_the_path_makes_the_answer() {
    let mut rig = rig();
    let save = rig.fixture.by_label("Save…").expect("the Save button is shown");
    rig.fixture.click(&save);
    rig.fixture.idle(2);
    let request = rig.requested.borrow().clone().expect("the press filed a save request");
    assert_eq!((request.key.as_str(), request.file_name.as_deref()), (SAVE_PATH_KEY, Some("poster.jpg")));
    assert_eq!(request.filters.first().map(|filter| filter.extensions.clone()), Some(vec!["jpg".to_owned()]));
    assert!(rig.answered.borrow().is_none(), "the dialog waits for the host");
    assert!(rig.closed.borrow().is_none(), "Save… does not close the dialog by itself");

    rig.host.borrow_mut().answer_path(request.key, PathBuf::from("/exports/poster.jpg"));
    rig.fixture.idle(2);
    let answered = rig.answered.borrow().clone().expect("the path makes the answer");
    assert_eq!(answered.path, PathBuf::from("/exports/poster.jpg"));
    assert_eq!((answered.width, answered.height), (DOCUMENT.width, DOCUMENT.height));
    assert_eq!(*rig.closed.borrow(), Some(CloseReason::Activated), "the path answer closes the dialog");
}

/// With Alt held the second button is Reset: it restores the opening settings and keeps the dialog open, and Save…
/// stays a button that waits for the host.
#[trial]
fn alt_turns_cancel_into_reset_which_keeps_save_for_web_open() {
    let options = Rc::new(RefCell::new(SaveForWebOptions::default()));
    let closed = Rc::new(RefCell::new(None::<CloseReason>));
    let (held, done) = (Rc::clone(&options), Rc::clone(&closed));
    let mut fixture = Fixture::new(move |ui| {
        let mut options = held.borrow_mut();
        let out = SaveForWebDialog::new("poster.psd", DOCUMENT, WebPictures::new(), Bind::bound(&mut *options)).show(ui);
        if out.closed.is_some() {
            *done.borrow_mut() = out.closed;
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
    assert!(closed.borrow().is_none(), "Reset keeps the dialog open");
}
