//! Save for Web and Export As in a harness.
#![allow(clippy::unwrap_used, reason = "tests: a poisoned stub or a missing dialog is a failed test")]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use craft_host::{AppInfo, FileError, Files, Host, PickAnswer, PickId, PickRequest, Picked};
use egui::accesskit::Role;
use egui::{ColorImage, TextureHandle, TextureOptions, vec2};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, HarnessBuilder};

use super::web::{SaveForWebDialog, WebSettings, WebVariant};
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
}

impl Files for StubFiles {
    fn pick(&self, request: PickRequest) -> PickId {
        let mut state = self.state.lock().unwrap();
        state.requests.push(request);
        PickId(state.requests.len() as u64)
    }

    fn take_pick(&self) -> Option<PickAnswer> {
        let mut state = self.state.lock().unwrap();
        if state.answered.is_empty() { None } else { Some(state.answered.remove(0)) }
    }

    fn return_pick(&self, answer: PickAnswer) {
        self.state.lock().unwrap().answered.insert(0, answer);
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

fn wanted(harness: &mut Harness<'static, App>) -> Vec<(usize, u64, WebSettings)> {
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
    assert_eq!(result.settings, WebSettings::default());
}

// Export As shows Quality only for JPG and lossy WebP, and the ICO limit under the size.
#[test]
#[ignore = "skeleton"]
fn export_as_shows_the_options_of_the_chosen_format() {
    todo!()
}

// Export As asks for a new preview revision on each change and shows "≈" until the exact size.
#[test]
#[ignore = "skeleton"]
fn export_as_preview_revisions_and_estimate() {
    todo!()
}
