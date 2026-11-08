//! The own logic of Save for Web: the 4-Up variants, the download estimate, the animation clock, which panes the
//! dialog asks the app to encode, and the Save… hand-off.

use craft_behavior::PixelSize;
use craft_ui::contract::Bind;
use craft_ui::patterns::form_dialogs::{
    HostRequests, PathRequest, SAVE_PATH_KEY, SaveForWebDialog, SaveForWebOptions, SaveForWebResult, WantedVariant, WebFormat, WebPane, WebPictures,
    WebPreset, WebView, advance_frame, download_seconds, four_up_variants,
};
use craft_ui::runtime::overlay::CloseReason;
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

#[trial]
fn zz_diag_save_button_per_frame() {
    let mut fixture = Fixture::new(move |ui| {
        let mut open = true;
        SaveForWebDialog::new("poster.psd", DOCUMENT, WebPictures::new(), Bind::kept()).opened(Bind::bound(&mut open)).show(ui);
    });
    let mut frames: Vec<Vec<(craft_ui::testing::access::NodeKey, String, egui::Rect)>> = Vec::new();
    for _ in 0..7 {
        fixture.idle(1);
        let tree = fixture.tree();
        frames.push(
            tree.keys()
                .into_iter()
                .filter_map(|key| Some((key, format!("{:?} {:?}", tree.role_of(key)?, tree.name_of(key).unwrap_or("")), tree.bounds_of(key)?)))
                .collect(),
        );
    }
    let mut out = Vec::new();
    for at in 2..6 {
        out.push(format!("--- frame {} to {}", at + 1, at + 2));
        for (key, name, rect) in &frames[at + 1] {
            let old = frames[at].iter().find(|(other, _, _)| other == key).map(|(_, _, rect)| *rect);
            if old != Some(*rect) {
                out.push(format!("{name}: {old:?} -> {rect:?}"));
            }
        }
    }
    panic!("DIAG\n{}", out.join("\n"));
}
