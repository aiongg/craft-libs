//! Prompt and Alert in a harness.

use egui::accesskit::Role;
use egui::os::OperatingSystem;
use egui::{Event, Key, Modifiers};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::theme::{Base, install};

struct App {
    prompt: bool,
    alert: bool,
    stack: DialogStack,
    answers: Vec<ButtonRole>,
    alerts: usize,
}

impl App {
    fn prompt() -> Self {
        Self { prompt: true, alert: false, stack: DialogStack::default(), answers: Vec::new(), alerts: 0 }
    }

    fn alert() -> Self {
        Self { prompt: false, alert: true, ..Self::prompt() }
    }
}

fn harness(app: App, os: OperatingSystem) -> Harness<'static, App> {
    HarnessBuilder::default()
        .with_size(egui::vec2(640.0, 480.0))
        .with_os(os)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, app: &mut App| {
                let ctx = &ui.ctx().clone();
                if app.prompt {
                    let answer = Prompt::new("unsaved", "Unsaved changes", "Do you want to save the changes?")
                        .answer(ButtonRole::Default, "Yes", Some(Key::Y))
                        .answer(ButtonRole::Alternate, "No", Some(Key::N))
                        .answer(ButtonRole::Cancel, "Cancel", Some(Key::C))
                        .stack(&app.stack)
                        .show(ctx);
                    if let Some(answer) = answer {
                        app.answers.push(answer);
                        app.prompt = false;
                    }
                }
                if app.alert && Alert::new("about", "About", "Version 1.0").stack(&app.stack).show(ctx) {
                    app.alerts += 1;
                    app.alert = false;
                }
            },
            app,
        )
}

#[test]
fn keys_answer_a_prompt_and_windows_labels_show_them() {
    for (key, role) in [
        (Key::Y, ButtonRole::Default),
        (Key::N, ButtonRole::Alternate),
        (Key::C, ButtonRole::Cancel),
        (Key::Enter, ButtonRole::Default),
        (Key::Escape, ButtonRole::Cancel),
    ] {
        let mut harness = harness(App::prompt(), OperatingSystem::Windows);
        harness.run();
        harness.get_by_label("(Y)es");
        harness.get_by_label("(N)o");
        harness.get_by_label("(C)ancel");
        harness.key_press(key);
        harness.run();
        assert_eq!(harness.state().answers, [role], "{key:?}");
    }
}

#[test]
fn a_click_answers_and_macos_shows_plain_labels_with_the_keys_working() {
    let mut harness = harness(App::prompt(), OperatingSystem::Mac);
    harness.run();
    harness.get_by_label("Yes");
    harness.get_by_label("No").click();
    harness.run();
    assert_eq!(harness.state().answers, [ButtonRole::Alternate]);

    let mut harness = self::harness(App::prompt(), OperatingSystem::Mac);
    harness.run();
    harness.key_press(Key::Y);
    harness.run();
    assert_eq!(harness.state().answers, [ButtonRole::Default]);
}

#[test]
fn a_prompt_under_another_dialog_ignores_its_keys() {
    let mut app = App::prompt();
    app.stack.push(Prompt::new("unsaved", "", "").id(), None);
    app.stack.push(dialog_id("above"), None);
    let mut harness = harness(app, OperatingSystem::Windows);
    harness.run();
    for key in [Key::Y, Key::Enter, Key::Escape] {
        harness.key_press(key);
        harness.run();
    }
    assert!(harness.state().answers.is_empty());
    harness.state_mut().stack.remove(dialog_id("above"));
    harness.run();
    harness.key_press(Key::N);
    harness.run();
    assert_eq!(harness.state().answers, [ButtonRole::Alternate]);
}

#[test]
fn the_default_answer_has_the_focus_and_the_buttons_name_their_keys() {
    let mut harness = harness(App::prompt(), OperatingSystem::Windows);
    harness.run();
    assert!(harness.get_by_label("(Y)es").accesskit_node().is_focused());
    assert_eq!(harness.get_by_label("(N)o").accesskit_node().data().keyboard_shortcut(), Some("N"));
}

#[test]
fn an_alert_is_an_alert_dialog_described_by_its_message() {
    let mut harness = harness(App::alert(), OperatingSystem::Nix);
    harness.run();
    let alert = harness.get_by(|n| n.role() == Role::AlertDialog);
    assert_eq!(alert.accesskit_node().description().as_deref(), Some("Version 1.0"));
    assert!(alert.accesskit_node().is_modal());
}

#[test]
fn enter_closes_an_alert_only_for_a_later_press_and_never_for_a_repeat() {
    let key = |pressed, repeat| Event::Key { key: Key::Enter, physical_key: None, pressed, repeat, modifiers: Modifiers::NONE };
    let mut harness = harness(App { alert: false, ..App::alert() }, OperatingSystem::Nix);
    harness.run();
    // The Enter that raises the alert is held: its press came before, its repeats come after.
    harness.event(key(true, false));
    harness.run();
    harness.state_mut().alert = true;
    harness.event(key(true, true));
    harness.run();
    harness.event(key(true, true));
    harness.run();
    assert!(harness.state().alert, "a repeating Enter does not close it");
    harness.event(key(false, false));
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().alerts, 1);
}

#[test]
fn the_enter_that_opens_an_alert_does_not_close_it() {
    let mut harness = harness(App { alert: false, ..App::alert() }, OperatingSystem::Nix);
    harness.run();
    harness.state_mut().alert = true;
    harness.key_press(Key::Enter);
    harness.run();
    assert!(harness.state().alert);
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().alerts, 1);
}

#[test]
fn esc_and_ok_close_an_alert() {
    let mut harness = harness(App::alert(), OperatingSystem::Nix);
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().alerts, 1);

    let mut harness = self::harness(App::alert(), OperatingSystem::Nix);
    harness.run();
    harness.get_by_label("OK").click();
    harness.run();
    assert_eq!(harness.state().alerts, 1);
}
