//! Standard prompts in a harness.

use egui::accesskit::Role;
use egui::os::OperatingSystem;
use egui::{Context, Key};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
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

// Row 168: a vector mask asks "Delete the vector mask?" with OK and Cancel; when the host
// cannot apply, Apply is disabled and neither Enter nor its key answers it.
#[test]
fn the_vector_mask_question_and_a_disabled_apply() {
    let mut harness = harness(StandardPrompt::trash_vector_mask(), OperatingSystem::Mac);
    harness.run();
    harness.get_by_label_contains("Delete the vector mask?");
    harness.get_by_role_and_label(Role::Button, "OK");
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().answers, [PromptChoice::Proceed]);

    let mut harness = self::harness(StandardPrompt::trash_mask().can_apply(false), OperatingSystem::Mac);
    harness.run();
    assert!(harness.get_by_role_and_label(Role::Button, "Apply").accesskit_node().is_disabled());
    harness.key_press(Key::Enter);
    harness.key_press(Key::A);
    harness.run();
    assert!(harness.state().answers.is_empty(), "a disabled Apply answers nothing");
    harness.get_by_role_and_label(Role::Button, "Delete").click();
    harness.run();
    assert_eq!(harness.state().answers, [PromptChoice::Alternate]);
}

// Row 169: the app gives message and buttons; the answer says which was pressed. Details is a
// disclosure that Space opens, and Enter on it does not answer the prompt.
#[test]
fn a_custom_prompt_has_the_apps_buttons_tone_and_details() {
    let rasterize = || {
        StandardPrompt::custom("This type layer must be rasterized.")
            .title("Rasterize")
            .button(PromptChoice::Proceed, "Convert to Smart Object", Some(Key::C))
            .button(PromptChoice::Alternate, "Rasterize", Some(Key::R))
            .button(PromptChoice::Cancel, "Cancel", Some(Key::A))
    };
    let mut harness = harness(rasterize(), OperatingSystem::Mac);
    harness.run();
    harness.get_by_label_contains("must be rasterized");
    harness.get_by_role_and_label(Role::Button, "Rasterize").click();
    harness.run();
    assert_eq!(harness.state().answers, [PromptChoice::Alternate]);
    let mut harness = self::harness(rasterize(), OperatingSystem::Mac);
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().answers, [PromptChoice::Proceed]);

    let gpu = StandardPrompt::custom("The GPU failed.")
        .button(PromptChoice::Proceed, "Keep Using CPU", Some(Key::K))
        .button(PromptChoice::Alternate, "Retry GPU", Some(Key::R))
        .tone(craft_theme::Tone::Warn, "Drawing will be slower.")
        .details("device lost");
    let mut harness = self::harness(gpu, OperatingSystem::Mac);
    harness.run();
    assert!(harness.query_all_by_label_contains("Drawing will be slower.").next().is_some());
    assert!(harness.query_all_by_label_contains("device lost").next().is_none(), "details start closed");
    let toggle = harness.get_by_role_and_label(Role::DisclosureTriangle, "Details");
    toggle.focus();
    harness.run();
    harness.key_press(Key::Space);
    harness.run();
    assert!(harness.query_all_by_label_contains("device lost").next().is_some());
    harness.key_press(Key::Enter);
    harness.run();
    assert!(harness.state().answers.is_empty(), "Enter on Details toggles it and answers nothing");
    // With no Cancel button, Esc answers the first Alternate (the library's prompt rule).
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().answers, [PromptChoice::Alternate]);
}

// Row 170: a damaged-file prompt shows its paragraphs and every warning.
#[test]
fn a_damaged_file_lists_its_warnings() {
    let prompt =
        StandardPrompt::damaged_file("a.psd").message("First paragraph.\n\nSecond paragraph.").warnings(["Layer 1 is short", "Layer 2 is short"]);
    let mut harness = harness(prompt, OperatingSystem::Mac);
    harness.run();
    harness.get_by_label_contains("Second paragraph.");
    harness.get_by_label("Layer 1 is short");
    harness.get_by_label("Layer 2 is short");
}

// A message kept as several catalog texts switches language paragraph by paragraph, the
// retry line with its value filled in.
#[test]
fn paragraphs_follow_the_language_each_on_its_own() {
    let (i18n, problems) = craft_i18n::I18n::new(
        &[("de", "\tThe file is damaged.\tDie Datei ist beschädigt.\n\tRetry in {n} s.\tNeuer Versuch in {n} s.\n", "")],
        &["de"],
    );
    assert!(problems.is_empty(), "{problems:?}");
    let de = i18n.from_code("de").expect("de");
    let prompt = StandardPrompt::custom("Open").message("The file is damaged.").paragraph("Retry in {n} s.").arg("n", "5");
    let mut harness = harness(prompt, OperatingSystem::Mac);
    harness.run();
    harness.get_by_label_contains("The file is damaged.\n\nRetry in 5 s.");
    craft_host::Host::builder(craft_host::AppInfo { id: "test", name: "Test", version: "1.0" })
        .i18n(std::sync::Arc::new(i18n))
        .lang(de)
        .install(&harness.ctx);
    harness.run();
    harness.get_by_label_contains("Die Datei ist beschädigt.\n\nNeuer Versuch in 5 s.");
}

// An app's own prompt texts are catalog keys read at draw time: a prompt left open follows a
// language switch, and an arg is filled in after translation and is not translated itself.
#[test]
fn a_custom_prompt_follows_the_language_while_open() {
    let (i18n, problems) =
        craft_i18n::I18n::new(&[("de", "\tRasterize {name}?\t{name} rastern?\n\tRasterize\tRastern\n\tRadius\tRadius-de\n", "")], &["de"]);
    assert!(problems.is_empty(), "{problems:?}");
    let de = i18n.from_code("de").expect("de");
    let i18n = std::sync::Arc::new(i18n);
    let prompt = StandardPrompt::custom("Rasterize {name}?").button(PromptChoice::Proceed, "Rasterize", Some(Key::R)).arg("name", "Radius");
    let mut harness = harness(prompt, OperatingSystem::Mac);
    harness.run();
    harness.get_by_label_contains("Rasterize ");
    harness.get_by_role_and_label(Role::Button, "Rasterize");
    craft_host::Host::builder(craft_host::AppInfo { id: "test", name: "Test", version: "1.0" }).i18n(i18n).lang(de).install(&harness.ctx);
    harness.run();
    harness.get_by_label_contains("rastern?");
    harness.get_by_label_contains("Radius");
    harness.get_by_role_and_label(Role::Button, "Rastern");
    assert!(harness.query_by_label_contains("Radius-de").is_none(), "the arg is not translated");
}
