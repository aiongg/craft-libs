//! `PasswordDialog`.

use egui::{Event, Key};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::dialog::DialogHost;
use crate::theme::{Base, install};

struct App {
    host: DialogHost,
    answers: Vec<DialogResult<PasswordAnswer>>,
}

fn harness(dialog: PasswordDialog) -> Harness<'static, App> {
    let mut host = DialogHost::default();
    host.open(dialog);
    HarnessBuilder::default().with_size(egui::vec2(640.0, 480.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, app: &mut App| {
            app.host.show(ui.ctx());
            if let Some(answer) = app.host.take::<PasswordDialog>() {
                app.answers.push(answer);
            }
        },
        App { host, answers: Vec::new() },
    )
}

fn type_into(harness: &mut Harness<'static, App>, label: &str, text: &str) {
    harness.get_by_label(label).focus();
    harness.run();
    harness.event(Event::Text(text.to_owned()));
    harness.run();
}

// "abc" is Weak, "abcdef12" Medium, "Abcdefgh1234" Strong; Secret's Debug prints "…".
#[test]
fn strength_and_secret_follow_pdfcraft() {
    assert_eq!(password_strength("abc"), PasswordStrength::Weak);
    assert_eq!(password_strength("abcdefgh"), PasswordStrength::Weak, "one class is weak at any length");
    assert_eq!(password_strength("abcdef12"), PasswordStrength::Medium);
    assert_eq!(password_strength("Abcdefgh1234"), PasswordStrength::Strong);
    assert_eq!(password_strength("abcdefgh1234"), PasswordStrength::Medium, "twelve characters need three classes");
    assert_eq!(format!("{:?}", Secret::new("hunter2".into())), "…");
    assert_eq!(format!("{:?}", Secret::default()), "\"\"");
}

// The field has focus on open; Enter answers the typed password with the tag; Esc cancels; the eye
// button switches its name between Show and Hide; an error line shows under the field.
#[test]
fn open_prompt_submits_with_enter_and_keeps_focus() {
    let mut harness = harness(PasswordDialog::open("report.pdf").tag(7_u32).error("Wrong one"));
    harness.run();
    harness.get_by_label_contains("“report.pdf” is protected.");
    harness.get_by_label_contains("Wrong one");
    assert!(harness.get_by_label("Password").accesskit_node().is_focused(), "the field has focus on open");
    harness.event(Event::Text("hunter2".to_owned()));
    harness.run();
    harness.get_by_label("Show password").click();
    harness.run();
    harness.get_by_label("Hide password");
    assert!(harness.get_by_label("Password").accesskit_node().is_focused(), "the eye button hands the focus back to the field");
    harness.key_press(Key::Enter);
    harness.run();
    let Some(DialogResult::Ok(mut answer)) = harness.state_mut().answers.pop() else { panic!("Enter answers") };
    assert_eq!(answer.secret.expose(), "hunter2");
    assert_eq!(answer.take_tag::<u32>(), Some(7));

    let mut harness = self::harness(PasswordDialog::permissions("report.pdf"));
    harness.run();
    harness.get_by_label("Unlock");
    harness.key_press(Key::Escape);
    harness.run();
    assert!(matches!(harness.state().answers[..], [DialogResult::Cancel]));
}

// Set keeps OK disabled until both fields match and aren't empty, and shows the strength line.
#[test]
fn set_password_needs_a_matching_second_entry() {
    let mut harness = harness(PasswordDialog::new(PasswordPurpose::Set));
    harness.run();
    assert!(harness.get_by_label("OK").accesskit_node().is_disabled());
    type_into(&mut harness, "Type Password", "Abcdefgh1234");
    harness.get_by_label_contains("Strength: Strong");
    assert!(harness.get_by_label("OK").accesskit_node().is_disabled(), "the second field is still empty");
    type_into(&mut harness, "Re-type Password", "Abcdefgh123");
    harness.get_by_label_contains("The passwords don't match.");
    assert!(harness.get_by_label("OK").accesskit_node().is_disabled());
    harness.event(Event::Text("4".to_owned()));
    harness.run();
    assert!(!harness.get_by_label("OK").accesskit_node().is_disabled());
    harness.get_by_label("OK").click();
    harness.run();
    let Some(DialogResult::Ok(answer)) = harness.state_mut().answers.pop() else { panic!("OK answers") };
    assert_eq!(answer.secret.expose(), "Abcdefgh1234");
}
