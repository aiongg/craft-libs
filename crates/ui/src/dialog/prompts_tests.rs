//! Standard prompts in a harness.

use egui::accesskit::Role;
use egui::os::OperatingSystem;
use egui::{Context, Key};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::theme::{Base, install};

struct App {
    host: DialogHost,
    answers: Vec<PromptChoice>,
    tags: Vec<Option<(u32, Vec<u8>)>>,
}

fn harness(prompt: StandardPrompt, os: OperatingSystem) -> Harness<'static, App> {
    let mut host = DialogHost::default();
    host.open(prompt);
    HarnessBuilder::default()
        .with_size(egui::vec2(640.0, 480.0))
        .with_os(os)
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            craft_host::Host::builder(craft_host::AppInfo { id: "test", name: "Test", version: "1.0" }).install(ctx);
        })
        .build_ui_state(
            |ui, app: &mut App| {
                app.host.show(ui.ctx());
                if let Some(DialogResult::Ok(mut answer)) = app.host.take::<StandardPrompt>() {
                    app.answers.push(answer.choice);
                    app.tags.push(answer.take_tag());
                }
            },
            App { host, answers: Vec::new(), tags: Vec::new() },
        )
}

fn delete() -> StandardPrompt {
    StandardPrompt::confirm_delete(["Background"], 1).dont_show_again("layer.delete")
}

fn skipped(ctx: &Context) -> bool {
    StandardPrompt::skipped(ctx, "layer.delete")
}

// Each kind shows its title, message and buttons; Enter proceeds, Esc cancels.
#[test]
fn each_kind_asks_its_question_with_the_right_buttons() {
    let kinds: [(StandardPrompt, &str, &[&str]); 5] = [
        (StandardPrompt::confirm_delete(["Background"], 1), "Delete Background?", &["Delete", "Cancel"]),
        (StandardPrompt::damaged_file("a.psd"), "“a.psd” has damaged data. Open it anyway?", &["Open Anyway", "Cancel"]),
        (StandardPrompt::changed_on_disk("a.psd"), "“a.psd” was changed by another app after it was opened or saved here.", &["Replace", "Cancel"]),
        (StandardPrompt::replace_file("a.psd", None), "“a.psd” already exists. Replace it?", &["Replace", "Cancel"]),
        (StandardPrompt::trash_mask(), "Apply the mask to the layer before removing it?", &["Apply", "Delete", "Cancel"]),
    ];
    for (prompt, message, buttons) in kinds {
        for (key, expected) in [(Key::Enter, PromptChoice::Proceed), (Key::Escape, PromptChoice::Cancel)] {
            let mut harness = self::harness(prompt_like(&prompt), OperatingSystem::Mac);
            harness.run();
            harness.get_by_label_contains(message);
            for button in buttons {
                harness.get_by_role_and_label(Role::Button, button);
            }
            harness.key_press(key);
            harness.run();
            assert_eq!(harness.state().answers, [expected]);
        }
    }
}

fn prompt_like(prompt: &StandardPrompt) -> StandardPrompt {
    StandardPrompt::new(prompt.kind().clone())
}

// Ticking "Don't show again" and proceeding stores the skip; `skipped` is then true and the app
// does not open the prompt.
#[test]
fn dont_show_again_is_remembered() {
    let mut harness = harness(delete(), OperatingSystem::Mac);
    harness.run();
    assert!(!skipped(&harness.ctx));
    harness.get_by_label("Don't show again").click();
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Delete").click();
    harness.run();
    assert_eq!(harness.state().answers, [PromptChoice::Proceed]);
    assert!(skipped(&harness.ctx));
}

// Cancel with the box ticked stores nothing.
#[test]
fn cancel_does_not_store_the_skip() {
    let mut harness = harness(delete(), OperatingSystem::Mac);
    harness.run();
    harness.get_by_label("Don't show again").click();
    harness.run();
    harness.get_by_label("Cancel").click();
    harness.run();
    assert_eq!(harness.state().answers, [PromptChoice::Cancel]);
    assert!(!skipped(&harness.ctx));
}

// The tag the app gave the prompt comes back with the choice.
#[test]
fn the_tag_returns_with_the_choice() {
    let mut harness = harness(StandardPrompt::damaged_file("a.psd").tag((7_u32, vec![1_u8, 2])), OperatingSystem::Mac);
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().answers, [PromptChoice::Proceed]);
    assert_eq!(harness.state().tags, [Some((7, vec![1, 2]))]);
}

// The app supplies the noun of the delete question; the plural rules pick its form.
#[test]
fn the_delete_prompt_uses_the_apps_noun() {
    let prompt = || StandardPrompt::confirm_delete(["A", "B"], 2);
    let mut harness = harness(prompt().noun("{n} layer", "{n} layers"), OperatingSystem::Mac);
    harness.run();
    harness.get_by_label_contains("2 layers");
    let mut harness = self::harness(prompt(), OperatingSystem::Mac);
    harness.run();
    harness.get_by_label_contains("2 items");
}
