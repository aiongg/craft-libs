//! The own logic of Save for Web: which panes the dialog asks the app to encode, and the Save… hand-off.

use craft_behavior::PixelSize;
use craft_ui::contract::Bind;
use craft_ui::patterns::form_dialogs::{
    Animation, HostRequests, PathRequest, SAVE_PATH_KEY, SaveForWebDialog, SaveForWebOptions, SaveForWebResult, WantedVariant, WebFormat, WebPane,
    WebPictures, WebPreset, WebVariant, WebView,
};
use craft_ui::runtime::overlay::CloseReason;
use craft_ui::testing::{Fixture, trial};
use egui::load::SizedTexture;
use egui::{ColorImage, Modifiers, TextureHandle, TextureOptions, Vec2};
use std::cell::{Cell, RefCell};
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

struct Pictured {
    fixture: Fixture,
    /// The byte sizes the app answers for panes 0, 1 and 2, read on every frame.
    bytes: Rc<RefCell<[u64; 3]>>,
    /// The frame the dialog reports.
    /// How many panes the dialog has asked the app to encode in all.
    asked: Rc<Cell<usize>>,
}

/// A dialog whose app answers all three encoded panes, over the settings `change` makes, for an animation of
/// `animation` when given.
fn pictured(change: impl FnOnce(&mut SaveForWebOptions), animation: Option<Animation>) -> Pictured {
    let bytes = Rc::new(RefCell::new([10_000, 20_000, 30_000]));
    let asked = Rc::new(Cell::new(0));
    let options = Rc::new(RefCell::new(options(change)));
    let handle: RefCell<Option<TextureHandle>> = RefCell::new(None);
    let (sizes, counted) = (Rc::clone(&bytes), Rc::clone(&asked));
    let mut fixture = Fixture::new(move |ui| {
        let mut handle = handle.borrow_mut();
        let texture =
            handle.get_or_insert_with(|| ui.ctx().load_texture("web-pane", ColorImage::filled([8, 8], Default::default()), TextureOptions::LINEAR));
        let sized = SizedTexture::new(texture.id(), Vec2::new(8.0, 8.0));
        let mut pictures = WebPictures::new().original(sized);
        for (index, bytes) in sizes.borrow().iter().enumerate() {
            pictures.set(WebVariant::new(index, 0, sized, *bytes));
        }
        let mut options = options.borrow_mut();
        let mut dialog = SaveForWebDialog::new("poster.psd", DOCUMENT, pictures, Bind::bound(&mut *options));
        if let Some(animation) = animation {
            dialog = dialog.animation(animation);
        }
        counted.set(counted.get() + dialog.show(ui).wanted_variants.len());
    });
    fixture.settle();
    Pictured { fixture, bytes, asked }
}

/// The painted texts that start with `prefix`.
fn texts_starting(fixture: &Fixture, prefix: &str) -> Vec<String> {
    fixture.texts().iter().map(|text| text.text.clone()).filter(|text| text.starts_with(prefix)).collect()
}

/// 4-Up shows the original and three encodes: the settings' format, then the variants the format rule gives
/// (PNG-24 is followed by PNG-8 and GIF).
#[trial]
fn four_up_shows_the_original_and_the_three_variants() {
    let mut rig = pictured(|options| options.format = WebFormat::Png24, None);
    let four_up = rig.fixture.by_label("4-Up").expect("the view tabs are shown");
    rig.fixture.click(&four_up);
    rig.fixture.idle(3);
    assert_eq!(texts_starting(&rig.fixture, "PNG  ").len(), 2, "the settings and the half variant");
    assert_eq!(texts_starting(&rig.fixture, "GIF  ").len(), 1, "the quarter variant");
    let panes = ["Original", "Optimized", "Optimized 2", "Optimized 3"];
    for pane in panes {
        let found = rig.fixture.tree().keys().into_iter().any(|key| rig.fixture.tree().name_of(key).is_some_and(|name| name.starts_with(pane)));
        assert!(found, "the pane {pane} is shown");
    }
}

/// The download line of an encoded pane follows the size the app answered, at the modem speed the caption names.
#[trial]
fn the_download_estimate_follows_the_encoded_size() {
    let mut rig = pictured(|_| {}, None);
    rig.bytes.borrow_mut()[0] = 35_000;
    rig.fixture.idle(2);
    assert_eq!(texts_starting(&rig.fixture, "5 sec @ 56.6 Kbps").len(), 1);
    rig.bytes.borrow_mut()[0] = 70_000;
    rig.fixture.idle(2);
    assert_eq!(texts_starting(&rig.fixture, "10 sec @ 56.6 Kbps").len(), 1, "a larger file takes longer");
    assert!(texts_starting(&rig.fixture, "5 sec @ 56.6 Kbps").is_empty());
}

/// Play steps the frames by the clock; Pause keeps the frame.
#[trial]
fn the_animation_advances_while_playing_and_stops_on_pause() {
    let mut rig = pictured(|_| {}, Some(Animation::new(12, 24.0)));
    assert_eq!(texts_starting(&rig.fixture, "1 / 12").len(), 1);
    let asked = rig.asked.get();
    let play = rig.fixture.by_label("Play").expect("the play row is shown");
    rig.fixture.click(&play);
    rig.fixture.idle(2);
    rig.fixture.advance(0.26);
    rig.fixture.idle(2);
    assert_eq!(texts_starting(&rig.fixture, "7 / 12").len(), 1, "six frames passed in 0.26 s at 24 fps");
    assert!(rig.asked.get() > asked, "a new frame asks the app to encode it");
    let pause = rig.fixture.by_label("Pause").expect("Play became Pause");
    rig.fixture.click(&pause);
    rig.fixture.idle(2);
    rig.fixture.advance(0.5);
    rig.fixture.idle(2);
    assert_eq!(texts_starting(&rig.fixture, "7 / 12").len(), 1, "a paused animation keeps its frame");
}
