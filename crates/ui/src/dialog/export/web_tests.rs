//! Save for Web and Export As in a harness.
#![allow(clippy::unwrap_used, reason = "tests: a poisoned stub or a missing dialog is a failed test")]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use craft_host::{AppInfo, FileError, Files, Host, PickAnswer, PickId, PickRequest, Picked};
use egui::accesskit::Role;
use egui::{ColorImage, TextureHandle, TextureOptions, vec2};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, HarnessBuilder};

use super::web::{SaveForWebDialog, SaveForWebOptions, WebVariant};
use super::*;
use crate::dialog::DialogHost;
use crate::theme::{Base, install};

/// A file service that records the requests and answers when the test says so.
#[derive(Default)]
struct StubFiles {
    state: Mutex<StubState>,
}

#[derive(Default)]
struct StubState {
    requests: Vec<PickRequest>,
    answered: Vec<PickAnswer>,
}

impl StubFiles {
    fn answer(&self, ticket: usize, picked: Picked) {
        let mut state = self.state.lock().unwrap();
        let request = state.requests[ticket - 1].clone();
        state.answered.push(PickAnswer { id: PickId(ticket as u64), request, picked });
    }

    fn requests(&self) -> Vec<PickRequest> {
        self.state.lock().unwrap().requests.clone()
    }

    /// Answers that came and were neither taken nor forgotten.
    fn unclaimed(&self) -> usize {
        self.state.lock().unwrap().answered.len()
    }
}

impl Files for StubFiles {
    fn pick(&self, request: PickRequest) -> PickId {
        let mut state = self.state.lock().unwrap();
        state.requests.push(request);
        PickId(state.requests.len() as u64)
    }

    fn take_answer(&self, id: PickId) -> Option<Picked> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let at = state.answered.iter().position(|a| a.id == id)?;
        Some(state.answered.remove(at).picked)
    }

    fn forget(&self, id: PickId) {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).answered.retain(|a| a.id != id);
    }

    fn pick_pending(&self) -> bool {
        false
    }

    fn read(&self, _path: &Path) -> Result<Vec<u8>, FileError> {
        Err(FileError::Unsupported)
    }

    fn write(&self, _path: &Path, _bytes: &[u8]) -> Result<(), FileError> {
        Err(FileError::Unsupported)
    }

    fn last_folder(&self, _purpose: &str) -> Option<PathBuf> {
        None
    }
}

struct App {
    host: DialogHost,
    answers: Vec<DialogResult<SaveForWebResult>>,
}

fn harness() -> (Harness<'static, App>, Arc<StubFiles>) {
    let files = Arc::new(StubFiles::default());
    let stub = files.clone();
    let mut harness = HarnessBuilder::default()
        .with_size(vec2(1100.0, 900.0))
        .with_context_setup(move |ctx| {
            install(ctx, &Base::suite_default());
            Host::builder(AppInfo { id: "test", name: "Test", version: "1.0" }).files(stub).install(ctx);
        })
        .build_ui_state(
            |ui, app: &mut App| {
                app.host.show(ui.ctx());
                if let Some(answer) = app.host.take::<SaveForWebDialog>() {
                    app.answers.push(answer);
                }
            },
            App { host: DialogHost::default(), answers: Vec::new() },
        );
    open(&mut harness);
    (harness, files)
}

fn texture(harness: &Harness<'static, App>) -> TextureHandle {
    harness.ctx.load_texture("web-test", ColorImage::filled([4, 4], egui::Color32::WHITE), TextureOptions::LINEAR)
}

fn open(harness: &mut Harness<'static, App>) {
    let dialog = SaveForWebDialog::new("poster.psd", 1200, 800, texture(harness));
    harness.state_mut().host.open(dialog);
    harness.run();
}

fn web_dialog<'a>(harness: &'a mut Harness<'static, App>) -> &'a mut SaveForWebDialog {
    harness.state_mut().host.get_mut::<SaveForWebDialog>().unwrap()
}

fn wanted(harness: &mut Harness<'static, App>) -> Vec<(usize, u64, SaveForWebOptions)> {
    harness.state_mut().host.get_mut::<SaveForWebDialog>().unwrap().wanted_variants()
}

fn variant(harness: &Harness<'static, App>, index: usize, revision: u64, bytes: u64, colors: Option<u16>) -> WebVariant {
    WebVariant { index, revision, texture: texture(harness), bytes, colors, palette: Vec::new(), proxy: false }
}

fn tab(harness: &mut Harness<'static, App>, name: &str) {
    harness.get_by_role_and_label(Role::RadioButton, name).click();
    harness.run();
}

// Save for Web opens on 2-Up, then on the view used last; 4-Up asks the app for three panes.
#[test]
fn save_for_web_remembers_the_view_and_four_up_asks_for_three_panes() {
    let (mut harness, _) = harness();
    let first = wanted(&mut harness);
    assert_eq!(first.iter().map(|(index, ..)| *index).collect::<Vec<_>>(), [0], "2-Up encodes the current settings only");
    assert!(wanted(&mut harness).is_empty(), "nothing changed since the last call");

    tab(&mut harness, "4-Up");
    let four = wanted(&mut harness);
    assert_eq!(four.iter().map(|(index, ..)| *index).collect::<Vec<_>>(), [0, 1, 2]);
    assert!(four.iter().all(|(_, revision, _)| *revision > first[0].1), "a view change is a new revision");
    let qualities: Vec<u8> = four.iter().map(|(_, _, settings)| settings.quality).collect();
    assert_eq!(qualities, [60, 30, 15]);

    harness.get_by_label("Cancel").click();
    harness.run();
    assert_eq!(harness.state().answers, [DialogResult::Cancel]);

    open(&mut harness);
    assert_eq!(wanted(&mut harness).len(), 3, "the next Save for Web opens on 4-Up");
    tab(&mut harness, "Original");
    assert!(wanted(&mut harness).is_empty(), "Original needs no encode");
    assert_eq!(harness.state_mut().host.get_mut::<SaveForWebDialog>().map(|dialog| dialog.frame()), Some(0));
}

// The Matte swatch asks the app for a colour once; the answer sets the matte and is a new revision.
#[test]
fn the_matte_swatch_asks_for_a_colour_and_the_answer_sets_it() {
    let (mut harness, _) = harness();
    let first = wanted(&mut harness);
    harness.get_by_role_and_label(Role::ColorWell, "Matte").click();
    harness.run();
    let request = web_dialog(&mut harness).pick_color().expect("the swatch asks for a colour");
    assert_eq!(request.key, MATTE_COLOR_KEY);
    assert_eq!(request.color, craft_theme::Srgba::WHITE, "the default matte is white");
    assert!(web_dialog(&mut harness).pick_color().is_none(), "taken once");

    web_dialog(&mut harness).set_color(&request.key, craft_theme::Srgba::rgb(1.0, 0.0, 0.0));
    harness.run();
    let changed = wanted(&mut harness);
    assert_eq!(changed[0].2.matte, [255, 0, 0]);
    assert!(changed[0].1 > first[0].1);
}

// An encoded pane for an older revision is dropped; the caption shows size, download time and
// colours of the current one.
#[test]
fn stale_variants_are_dropped_and_captions_follow_the_current_one() {
    let (mut harness, _) = harness();
    let old = wanted(&mut harness)[0].1;
    tab(&mut harness, "Optimized");
    let current = wanted(&mut harness)[0].1;
    assert!(current > old);

    let give = |harness: &mut Harness<'static, App>, variant| harness.state_mut().host.get_mut::<SaveForWebDialog>().unwrap().set_variant(variant);
    let stale = variant(&harness, 0, old, 1_000, None);
    let fresh = variant(&harness, 0, current, 48_200, Some(64));
    give(&mut harness, stale);
    harness.run();
    assert!(harness.query_by_label("JPG  1K").is_none(), "the older revision is dropped");
    give(&mut harness, fresh);
    harness.run();
    assert!(harness.query_by_label("JPG  47K").is_some());
    assert!(harness.query_by_label("7 sec @ 56.6 Kbps  ·  64 colors").is_some());
}

// Save… queues a save picker; a cancelled pick keeps the dialog open, a path finishes it.
#[test]
fn save_waits_for_the_picker_and_a_cancelled_pick_keeps_the_dialog() {
    let (mut harness, files) = harness();
    harness.get_by_label("Save…").click();
    harness.run();
    let requests = files.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        (requests[0].purpose.as_str(), requests[0].file_name.as_deref(), requests[0].suggested_extension.as_deref()),
        ("export", Some("poster.jpg"), Some("jpg"))
    );
    assert!(harness.state().answers.is_empty(), "the dialog waits for the picker");

    files.answer(1, Picked::Cancelled);
    harness.run();
    assert!(harness.state().answers.is_empty());
    assert!(harness.state_mut().host.is_open::<SaveForWebDialog>(), "a cancelled pick keeps the dialog");

    harness.get_by_label("Save…").click();
    harness.run();
    assert_eq!(files.requests().len(), 2);
    files.answer(2, Picked::Paths(vec![PathBuf::from("/out/poster.jpg")]));
    harness.run();
    let [DialogResult::Ok(result)] = harness.state().answers.as_slice() else { panic!("one answer expected") };
    assert_eq!((result.path.as_path(), result.width, result.height), (Path::new("/out/poster.jpg"), 1200, 800));
    assert_eq!(result.options, SaveForWebOptions::default());
}

// A host that closes the dialog while its picker is open (cancel, or another Save for Web opened
// over it) drops the ticket, so the late answer is not left in the service.
#[test]
fn a_dialog_closed_by_the_host_forgets_its_pick() {
    let (mut harness, files) = harness();
    harness.get_by_label("Save…").click();
    harness.run();
    files.answer(1, Picked::Paths(vec![PathBuf::from("/out/poster.jpg")]));
    assert_eq!(files.unclaimed(), 1);
    let ctx = harness.ctx.clone();
    assert!(harness.state_mut().host.cancel::<SaveForWebDialog>(&ctx));
    assert_eq!(files.unclaimed(), 0, "the cancelled dialog's answer is dropped");
    assert!(harness.state().answers.is_empty());
}

// The spinner of a pending preview repaints for ever, so these tests run a fixed number of steps.
struct ExportApp {
    host: DialogHost,
    answers: Vec<DialogResult<ExportAsResult>>,
}

fn export_harness() -> (Harness<'static, ExportApp>, Arc<StubFiles>) {
    let files = Arc::new(StubFiles::default());
    let stub = files.clone();
    let mut harness = HarnessBuilder::default()
        .with_size(vec2(1100.0, 900.0))
        .with_context_setup(move |ctx| {
            install(ctx, &Base::suite_default());
            Host::builder(AppInfo { id: "test", name: "Test", version: "1.0" }).files(stub).install(ctx);
        })
        .build_ui_state(
            |ui, app: &mut ExportApp| {
                app.host.show(ui.ctx());
                if let Some(answer) = app.host.take::<ExportAsDialog>() {
                    app.answers.push(answer);
                }
            },
            ExportApp { host: DialogHost::default(), answers: Vec::new() },
        );
    harness.state_mut().host.open(ExportAsDialog::new("poster", 1200, 800));
    harness.run_steps(6);
    (harness, files)
}

fn export_dialog<'a>(harness: &'a mut Harness<'static, ExportApp>) -> &'a mut ExportAsDialog {
    harness.state_mut().host.get_mut::<ExportAsDialog>().unwrap()
}

fn choose(harness: &mut Harness<'static, ExportApp>, list: &str, option: &str) {
    harness.get_by_role_and_label(Role::ComboBox, list).click();
    harness.run_steps(6);
    harness.get_by_label(option).click();
    harness.run_steps(6);
}

// Export As shows Quality only for JPG and lossy WebP, and the ICO limit under the size.
#[test]
fn export_as_shows_the_options_of_the_chosen_format() {
    let (mut harness, files) = export_harness();
    let quality = |harness: &Harness<'static, ExportApp>| harness.query_by_role_and_label(Role::SpinButton, "Quality").is_some();
    assert!(!quality(&harness), "PNG has no quality");
    assert!(harness.query_by_label("Transparency").is_some());
    assert!(harness.query_by_label("ICO is limited to 256 × 256 pixels").is_none());

    choose(&mut harness, "Format", "JPG");
    assert!(quality(&harness));
    assert!(harness.query_by_label("Transparency").is_none(), "JPG has no alpha");

    choose(&mut harness, "Format", "WebP");
    assert!(harness.query_by_label("WEBP  ≈ —").is_some(), "no estimate yet");
    assert!(!quality(&harness), "WebP opens lossless (the Export preference), which hides Quality");
    harness.get_by_label("Lossless").click();
    harness.run_steps(6);
    assert!(quality(&harness));

    choose(&mut harness, "Format", "TGA");
    assert!(harness.query_by_role_and_label(Role::ComboBox, "Bits/Pixel").is_some());

    choose(&mut harness, "Format", "ICO");
    assert!(harness.query_by_role_and_label(Role::ComboBox, "Bits/Pixel").is_none());
    assert!(harness.query_by_label("ICO is limited to 256 × 256 pixels").is_some(), "1200 × 800 is over the limit");

    harness.get_by_label("Export…").click();
    harness.run_steps(6);
    let requests = files.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!((requests[0].purpose.as_str(), requests[0].file_name.as_deref()), ("export", Some("poster.ico")));
    files.answer(1, Picked::Cancelled);
    harness.run_steps(6);
    assert!(harness.state().answers.is_empty(), "a cancelled pick keeps the dialog");
    harness.get_by_label("Export…").click();
    harness.run_steps(6);
    files.answer(2, Picked::Paths(vec![PathBuf::from("/out/poster.ico")]));
    harness.run_steps(6);
    let [DialogResult::Ok(result)] = harness.state().answers.as_slice() else { panic!("one answer expected") };
    assert_eq!(
        (result.path.as_path(), result.options.format.as_str(), result.width, result.height),
        (Path::new("/out/poster.ico"), "ico", 1200, 800)
    );
}

// Same for Export As: a cancel from the host drops the ticket of its open picker.
#[test]
fn export_as_closed_by_the_host_forgets_its_pick() {
    let (mut harness, files) = export_harness();
    harness.get_by_label("Export…").click();
    harness.run_steps(6);
    assert_eq!(files.requests().len(), 1);
    files.answer(1, Picked::Paths(vec![PathBuf::from("/out/poster.png")]));
    let ctx = harness.ctx.clone();
    assert!(harness.state_mut().host.cancel::<ExportAsDialog>(&ctx));
    assert_eq!(files.unclaimed(), 0);
}

// Export As asks for a new preview revision on each change and shows "≈" until the exact size.
#[test]
fn export_as_preview_revisions_and_estimate() {
    let (mut harness, _) = export_harness();
    let (first, settings) = export_dialog(&mut harness).wanted_preview().unwrap();
    assert_eq!(settings, ExportOptions::default());
    assert!(export_dialog(&mut harness).wanted_preview().is_none(), "nothing changed since the last call");

    choose(&mut harness, "Format", "JPG");
    let (second, settings) = export_dialog(&mut harness).wanted_preview().unwrap();
    assert!(second > first);
    assert_eq!(settings.format, "jpg");

    let preview = |harness: &Harness<'static, ExportApp>, revision, estimate| ExportPreview { revision, texture: texture_of(&harness.ctx), estimate };
    let stale = preview(&harness, first, SizeEstimate::Proxy(1_000));
    export_dialog(&mut harness).set_preview(stale);
    harness.run_steps(6);
    assert!(harness.query_by_label("JPG  ≈ 1K").is_none(), "the older revision is dropped");

    let current = preview(&harness, second, SizeEstimate::Proxy(1_258_291));
    export_dialog(&mut harness).set_preview(current);
    harness.run_steps(6);
    assert!(harness.query_by_label("JPG  ≈ 1.2M").is_some());

    export_dialog(&mut harness).set_exact_estimate(first, 5_000_000);
    harness.run_steps(6);
    assert!(harness.query_by_label("JPG  ≈ 1.2M").is_some(), "an exact size for an older revision is ignored");
    export_dialog(&mut harness).set_exact_estimate(second, 1_100_000);
    harness.run_steps(6);
    assert!(harness.query_by_label("JPG  1.0M").is_some());
}

fn texture_of(ctx: &egui::Context) -> TextureHandle {
    ctx.load_texture("export-test", ColorImage::filled([4, 4], egui::Color32::WHITE), TextureOptions::LINEAR)
}
