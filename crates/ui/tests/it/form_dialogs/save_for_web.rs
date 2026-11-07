//! The own logic of Save for Web: the 4-Up variants, the download estimate, the animation clock, which panes the
//! dialog asks the app to encode, and the Save… hand-off.

use craft_behavior::PixelSize;
use craft_ui::contract::Bind;
use craft_ui::patterns::form_dialogs::{
    SaveForWebDialog, SaveForWebOptions, SaveForWebResult, SaveRequest, WantedVariant, WebFormat, WebPane, WebPictures, WebPreset, WebView,
    advance_frame, download_seconds, four_up_variants,
};
use craft_ui::testing::{Fixture, trial};
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

const FRAMES: u32 = 12;
const FPS: f32 = 24.0;
const DOCUMENT: PixelSize = PixelSize::new(1200, 800);

/// The opening settings changed by `change`.
fn options(change: impl FnOnce(&mut SaveForWebOptions)) -> SaveForWebOptions {
    let mut options = SaveForWebOptions::default();
    change(&mut options);
    options
}

fn with_format(format: WebFormat) -> SaveForWebOptions {
    options(|options| options.format = format)
}

/// 4-Up's two extra panes halve and quarter the quality or the colours; other formats become PNG-8 and GIF.
#[trial]
fn four_up_variants_halve_and_quarter_what_the_format_has() {
    let jpeg = four_up_variants(&options(|options| options.quality = 60));
    assert_eq!((jpeg[0].quality, jpeg[1].quality), (30, 15));
    assert!(jpeg.iter().all(|variant| variant.format == WebFormat::Jpeg));

    let gif = four_up_variants(&options(|options| (options.format, options.colors) = (WebFormat::Gif, 128)));
    assert_eq!((gif[0].colors, gif[1].colors), (64, 32));

    let lossless = four_up_variants(&options(|options| (options.format, options.lossless) = (WebFormat::WebP, true)));
    assert_eq!((lossless[0].format, lossless[1].format), (WebFormat::Png8, WebFormat::Gif), "lossless WebP has no quality to halve");
    assert_eq!((lossless[0].colors, lossless[1].colors), (128, 64));

    let png = four_up_variants(&with_format(WebFormat::Png24));
    assert_eq!((png[0].format, png[1].format), (WebFormat::Png8, WebFormat::Gif));
}

/// The estimate rounds up at 56.6 kilobits per second.
#[trial]
fn the_download_estimate_rounds_up() {
    assert_eq!(download_seconds(0, 56.6), 0);
    assert_eq!(download_seconds(7075, 56.6), 1);
    assert_eq!(download_seconds(7076, 56.6), 2);
    assert_eq!(download_seconds(50_000, 56.6), 8);
}

/// The clock steps whole frames, keeps the part of a frame that has not passed, wraps, and survives bad times.
#[trial]
fn the_animation_clock_steps_whole_frames_and_wraps() {
    assert_eq!(advance_frame(3, FRAMES, FPS, 10.0, 10.0).0, 3, "no time, no step");
    let (frame, clock) = advance_frame(3, FRAMES, FPS, 10.0, 10.0 + 2.5 / f64::from(FPS));
    assert_eq!(frame, 5);
    assert!((clock - (10.0 + 2.0 / f64::from(FPS))).abs() < 1e-9, "the half frame carries over");
    assert_eq!(advance_frame(FRAMES - 1, FRAMES, FPS, 0.0, 1.5 / f64::from(FPS)).0, 0, "after the last frame comes the first");
    assert_eq!(advance_frame(40, FRAMES, FPS, 0.0, 0.0).0, FRAMES - 1, "a frame past the end is clamped");
    assert_eq!(advance_frame(2, FRAMES, FPS, 5.0, 1.0), (2, 1.0), "a clock that went back restarts");
    assert_eq!(advance_frame(2, FRAMES, 0.0, 0.0, 1.0).0, (2 + 30) % FRAMES, "no frame rate plays at 30");
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
    requested: Rc<RefCell<Option<SaveRequest>>>,
    answered: Rc<RefCell<Option<SaveForWebResult>>>,
    path: Rc<RefCell<Option<PathBuf>>>,
}

fn rig() -> Rig {
    let asked = Rc::new(RefCell::new(Vec::new()));
    let requested = Rc::new(RefCell::new(None));
    let answered = Rc::new(RefCell::new(None));
    let path: Rc<RefCell<Option<PathBuf>>> = Rc::new(RefCell::new(None));
    let (wanted, request, result, answer) = (Rc::clone(&asked), Rc::clone(&requested), Rc::clone(&answered), Rc::clone(&path));
    let mut fixture = Fixture::new(move |ui| {
        let mut open = true;
        let mut dialog = SaveForWebDialog::new("poster.psd", DOCUMENT, WebPictures::new()).opened(Bind::bound(&mut open));
        if let Some(path) = answer.borrow_mut().take() {
            dialog = dialog.saved_to(path);
        }
        let shown = dialog.show(ui);
        wanted.borrow_mut().extend(shown.wanted_variants.iter().cloned());
        if shown.save_as.is_some() {
            *request.borrow_mut() = shown.save_as;
        }
        if shown.result.is_some() {
            *result.borrow_mut() = shown.result;
        }
    });
    fixture.settle();
    Rig { fixture, asked, requested, answered, path }
}

/// The dialog asks for the panes its view shows once per change, with a new revision each time, and the first pass
/// asks for the current settings alone (2-Up).
#[trial]
fn the_dialog_asks_for_the_panes_of_the_view_once_per_change() {
    let mut rig = rig();
    {
        let asked = rig.asked.borrow();
        assert_eq!(asked.len(), 1, "2-Up asks for the current settings only");
        assert_eq!((asked[0].index, asked[0].revision), (0, 1));
        assert_eq!(asked[0].options, SaveForWebOptions::default());
    }
    rig.fixture.idle(3);
    assert_eq!(rig.asked.borrow().len(), 1, "nothing changed, nothing asked");

    let four_up = rig.fixture.by_label("4-Up").expect("the view tabs are shown");
    rig.fixture.click(&four_up);
    rig.fixture.idle(3);
    let asked = rig.asked.borrow();
    let latest: Vec<_> = asked.iter().filter(|want| want.revision == 2).collect();
    assert_eq!(latest.iter().map(|want| want.index).collect::<Vec<_>>(), vec![0, 1, 2], "4-Up asks for three panes");
    assert_eq!(latest[1].options.quality, SaveForWebOptions::default().quality / 2);
}

/// Save… files the save picker for the document's name with the format's extension; the path the picker returns
/// makes the answer, with the size.
#[trial]
fn save_files_the_picker_and_the_path_makes_the_answer() {
    let mut rig = rig();
    let save = rig.fixture.by_label("Save…").expect("the Save button is shown");
    rig.fixture.click(&save);
    rig.fixture.idle(2);
    let request = rig.requested.borrow().clone().expect("the press filed a save request");
    assert_eq!((request.file_name.as_str(), request.extension), ("poster.jpg", "jpg"));
    assert!(rig.answered.borrow().is_none(), "the dialog waits for the picker");

    *rig.path.borrow_mut() = Some(PathBuf::from("/exports/poster.jpg"));
    rig.fixture.idle(2);
    let answered = rig.answered.borrow().clone().expect("the path makes the answer");
    assert_eq!(answered.path, PathBuf::from("/exports/poster.jpg"));
    assert_eq!((answered.width, answered.height), (DOCUMENT.width, DOCUMENT.height));
}
