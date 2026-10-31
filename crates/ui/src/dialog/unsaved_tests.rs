//! The unsaved-changes question in a harness.

use craft_quit::{CloseAnswer, CloseScope};
use egui::Key;
use egui::os::OperatingSystem;
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::theme::{Base, install};

struct App {
    host: DialogHost,
    answers: Vec<UnsavedAnswer>,
}

fn harness(dialog: UnsavedChangesDialog, os: OperatingSystem) -> Harness<'static, App> {
    let mut host = DialogHost::default();
    host.open(dialog);
    HarnessBuilder::default()
        .with_size(egui::vec2(640.0, 480.0))
        .with_os(os)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, app: &mut App| {
                app.host.show(ui.ctx());
                if let Some(DialogResult::Ok(answer)) = app.host.take::<UnsavedChangesDialog>() {
                    app.answers.push(answer);
                }
            },
            App { host, answers: Vec::new() },
        )
}

fn dialog() -> UnsavedChangesDialog {
    UnsavedChangesDialog::new(CloseScope::Close, "a.psd")
}

fn answer(answer: CloseAnswer) -> UnsavedAnswer {
    UnsavedAnswer { answer, apply_to_all: false }
}

// macOS shows Save / Don't Save / Cancel in Cancel, Don't Save, Save order; Windows shows Yes /
// No / Cancel with the letters in the labels.
#[test]
fn the_buttons_follow_the_platform_words_and_order() {
    let mut harness = harness(dialog(), OperatingSystem::Mac);
    harness.run();
    let x = |harness: &Harness<'_, App>, label: &str| harness.get_by_label(label).rect().center().x;
    let (cancel, dont_save, save) = (x(&harness, "Cancel"), x(&harness, "Don't Save"), x(&harness, "Save"));
    assert!(dont_save < cancel && cancel < save);
    harness.get_by_label_contains("“a.psd”");

    let mut harness = self::harness(dialog(), OperatingSystem::Windows);
    harness.run();
    let (yes, no, cancel) = (x(&harness, "(Y)es"), x(&harness, "(N)o"), x(&harness, "(C)ancel"));
    assert!(yes < no && no < cancel);
}

// With `remaining(2)` the "Apply to All" box shows; ticked, the answer carries `apply_to_all`.
#[test]
fn apply_to_all_shows_for_several_documents() {
    let mut single = harness(dialog(), OperatingSystem::Mac);
    single.run();
    assert!(single.query_by_label("Apply to All").is_none());

    let mut harness = harness(dialog().remaining(2), OperatingSystem::Mac);
    harness.run();
    harness.get_by_label("Apply to All").click();
    harness.run();
    harness.get_by_label("Don't Save").click();
    harness.run();
    assert_eq!(harness.state().answers, [UnsavedAnswer { answer: CloseAnswer::DontSave, apply_to_all: true }]);
}

// Enter saves, Esc cancels, D answers Don't Save without a modifier.
#[test]
fn enter_esc_and_the_letter_keys_answer() {
    for (key, expected) in [(Key::Enter, CloseAnswer::Save), (Key::Escape, CloseAnswer::Cancel), (Key::D, CloseAnswer::DontSave)] {
        let mut harness = harness(dialog(), OperatingSystem::Mac);
        harness.run();
        harness.key_press(key);
        harness.run();
        assert_eq!(harness.state().answers, [answer(expected)], "{key:?}");
    }
}
