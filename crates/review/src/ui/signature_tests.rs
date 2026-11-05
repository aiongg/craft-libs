//! The signature pad.

#![cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]

use std::io::Cursor;
use std::path::Path;
use std::sync::{Arc, Mutex};

use craft_host::{AppInfo, FileError, Files, Host, ManualJobs, PickId, PickRequest, Picked};
use craft_ui::dialog::{DialogHost, DialogResult};
use craft_ui::testing::symbols;
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::signature::{SignaturePadDialog, SignaturePadResult};
use crate::signature::{SavedSignature, SignatureDraft, SignatureTab};

fn harness(draft: SignatureDraft) -> Harness<'static, DialogHost> {
    with_files(draft, None).0
}

/// A file service whose picker answers `answer` at once.
struct Scripted {
    answer: Picked,
    waiting: Mutex<Vec<PickId>>,
}

impl Files for Scripted {
    fn pick(&self, _: PickRequest) -> PickId {
        let mut waiting = self.waiting.lock().unwrap();
        let id = PickId(waiting.len() as u64 + 1);
        waiting.push(id);
        id
    }

    fn take_answer(&self, id: PickId) -> Option<Picked> {
        let mut waiting = self.waiting.lock().unwrap();
        let at = waiting.iter().position(|w| *w == id)?;
        waiting.remove(at);
        Some(self.answer.clone())
    }

    fn forget(&self, id: PickId) {
        self.waiting.lock().unwrap().retain(|w| *w != id);
    }

    fn pick_pending(&self) -> bool {
        false
    }

    fn read(&self, path: &Path) -> Result<Vec<u8>, FileError> {
        Ok(std::fs::read(path)?)
    }

    fn write(&self, _: &Path, _: &[u8]) -> Result<(), FileError> {
        Err(FileError::Unsupported)
    }

    fn last_folder(&self, _: &str) -> Option<std::path::PathBuf> {
        None
    }
}

/// The pad on a host whose picker answers with `pick`; decode jobs wait for `ManualJobs`.
fn with_files(draft: SignatureDraft, pick: Option<Picked>) -> (Harness<'static, DialogHost>, Arc<ManualJobs>) {
    let jobs = Arc::new(ManualJobs::new());
    let mut host = DialogHost::new();
    host.open(SignaturePadDialog::new(draft));
    let setup_jobs = jobs.clone();
    let mut harness = Harness::<DialogHost>::builder()
        .with_size([640.0, 480.0])
        .with_context_setup(move |ctx| {
            install(ctx, &Base::suite_default());
            symbols::install(ctx);
            if let Some(answer) = &pick {
                let files = Arc::new(Scripted { answer: answer.clone(), waiting: Mutex::new(Vec::new()) });
                Host::builder(AppInfo { id: "test", name: "Test", version: "1" }).files(files).jobs(setup_jobs.clone()).build().install(ctx);
            }
        })
        .build_ui_state(|ui, host: &mut DialogHost| host.show(ui.ctx()), host);
    harness.run();
    (harness, jobs)
}

// Apply returns the drawn strokes on the Draw tab.
#[test]
fn apply_returns_the_current_tab() {
    let strokes = vec![vec![[0.1, 0.5], [0.4, 0.2], [0.9, 0.6]]];
    let draft = SignatureDraft { tab: SignatureTab::Draw, strokes: strokes.clone(), ..SignatureDraft::new(false, "Ana") };
    let mut harness = harness(draft);
    harness.get_by_role_and_label(Role::Button, "Apply").click();
    harness.run();
    let answer = harness.state_mut().take::<SignaturePadDialog>();
    assert!(
        matches!(answer, Some(DialogResult::Ok(SignaturePadResult { signature: SavedSignature::Drawn(drawn), initials: false })) if drawn == strokes)
    );
}

// Apply is disabled for an empty draft.
#[test]
fn apply_waits_for_content() {
    let mut harness = harness(SignatureDraft::new(false, "  "));
    assert!(harness.get_by_role_and_label(Role::Button, "Apply").accesskit_node().is_disabled());
    harness.get_by_role_and_label(Role::Button, "Clear").click();
    harness.run();
    assert!(harness.state_mut().take::<SignaturePadDialog>().is_none());

    let harness = self::harness(SignatureDraft::new(false, "Ana"));
    assert!(!harness.get_by_role_and_label(Role::Button, "Apply").accesskit_node().is_disabled());
}

fn png(width: u32, height: u32) -> Vec<u8> {
    let mut bytes = Cursor::new(Vec::new());
    image::RgbaImage::from_pixel(width, height, image::Rgba([10, 20, 30, 255])).write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    bytes.into_inner()
}

/// Presses Browse… on the Image tab, lets the decode job run and shows the result.
fn browse(contents: Vec<u8>) -> Harness<'static, DialogHost> {
    let draft = SignatureDraft { tab: SignatureTab::Image, ..SignatureDraft::new(false, "Ana") };
    let (mut harness, jobs) = with_files(draft, Some(Picked::Contents(vec![("signature.png".into(), contents)])));
    harness.get_by_role_and_label(Role::Button, "Browse…").click();
    harness.run();
    assert_eq!(jobs.run_all(), 1, "the decode runs on the host's jobs");
    harness.run();
    harness
}

// A picked PNG is decoded off the pass and Apply returns it.
#[test]
fn picked_image_is_decoded_on_the_jobs_and_applied() {
    let mut harness = browse(png(3, 2));
    harness.get_by_role_and_label(Role::Button, "Apply").click();
    harness.run();
    let answer = harness.state_mut().take::<SignaturePadDialog>();
    let Some(DialogResult::Ok(SignaturePadResult { signature: SavedSignature::Image(image), initials: false })) = answer else {
        panic!("expected an image, got {answer:?}");
    };
    assert_eq!((image.width(), image.height(), image.rgba().len()), (3, 2, 24));
}

// An image past the side limit or the byte cap shows the error word and leaves Apply disabled.
#[test]
fn oversize_pick_shows_an_error() {
    for contents in [png(crate::signature::MAX_IMAGE_SIDE + 1, 1), vec![0; crate::signature::MAX_IMAGE_BYTES + 1]] {
        let harness = browse(contents);
        harness.get_by_label_contains("too large");
        assert!(harness.get_by_role_and_label(Role::Button, "Apply").accesskit_node().is_disabled());
    }
}

// Bytes that are no image show the other error word.
#[test]
fn unreadable_pick_shows_an_error() {
    let harness = browse(b"not an image".to_vec());
    harness.get_by_label_contains("could not be read");
}

// The Draw pad is named for screen readers.
#[test]
fn draw_pad_has_a_name() {
    let harness = harness(SignatureDraft { tab: SignatureTab::Draw, ..SignatureDraft::default() });
    harness.get_by_role_and_label(Role::Image, "Draw");
}
