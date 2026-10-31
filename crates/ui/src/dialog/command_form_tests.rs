//! The command form in a harness.

use egui::accesskit::Role;
use egui::os::OperatingSystem;
use egui::{Event, Key, Modifiers, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::{CommandForm, DialogFields, DialogHost, DialogResult, FieldValue, Param, ParamKind};
use crate::theme::{Base, install};

struct App {
    host: DialogHost,
    answers: Vec<DialogResult<DialogFields>>,
}

fn form() -> CommandForm {
    CommandForm::new(
        "fit",
        "Fit Image",
        vec![
            Param::new("width", "Width:", ParamKind::Int { min: Some(1), max: None, default: 800 }),
            Param::new("name", "Name:", ParamKind::Text { default: "out".into() }),
            Param::new("sharpen", "Sharpen:", ParamKind::Bool { default: true }),
        ],
    )
    .ok_label("Fit")
}

fn harness(form: CommandForm) -> Harness<'static, App> {
    let mut host = DialogHost::default();
    host.open(form);
    HarnessBuilder::default()
        .with_size(vec2(700.0, 500.0))
        .with_os(OperatingSystem::Nix)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, app: &mut App| {
                app.host.show(ui.ctx());
                if let Some(answer) = app.host.take::<CommandForm>() {
                    app.answers.push(answer);
                }
            },
            App { host, answers: Vec::new() },
        )
}

// One row per Param in list order, the first field focused and selected on open.
#[test]
fn rows_follow_the_param_list_and_the_first_field_is_focused() {
    let mut harness = harness(form());
    harness.run();
    let top = |harness: &Harness<'_, App>, role, label: &str| harness.get_by_role_and_label(role, label).rect().top();
    let (width, name) = (top(&harness, Role::SpinButton, "Width:"), top(&harness, Role::TextInput, "Name:"));
    assert!(width < name && name < harness.get_by_label("Sharpen").rect().top());
    assert!(harness.get_by_role_and_label(Role::SpinButton, "Width:").accesskit_node().is_focused());
    harness.event(Event::Text("1200".to_owned()));
    harness.run();
    harness.get_by_label("Fit").click();
    harness.run();
    harness.run();
    let [DialogResult::Ok(fields)] = &harness.state().answers[..] else { panic!("one OK, got {:?}", harness.state().answers) };
    assert_eq!((fields.int("width"), fields.text("name"), fields.bool("sharpen")), (Some(1200), Some("out"), Some(true)));
}

// `ok_enabled(false)` disables OK and Enter does nothing; enabling it answers.
#[test]
fn ok_can_be_disabled_by_the_host() {
    let mut harness = harness(form().ok_enabled(false));
    harness.run();
    assert!(harness.get_by_label("Fit").accesskit_node().is_disabled());
    harness.key_press(Key::Enter);
    harness.run();
    assert!(harness.state().answers.is_empty());
    harness.state_mut().host.get_mut::<CommandForm>().expect("open").set_ok_enabled(true);
    harness.run();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    harness.run();
    assert!(matches!(harness.state().answers[..], [DialogResult::Ok(_)]));
}

// Alt+Cancel restores the values the form opened with.
#[test]
fn alt_cancel_resets_the_form() {
    let mut harness = harness(form().values(DialogFields::from_pairs([("name", FieldValue::Text("given".into()))])));
    harness.run();
    harness.get_by_role_and_label(Role::TextInput, "Name:").focus();
    harness.run();
    harness.event(Event::Text("x".to_owned()));
    harness.run();
    assert_ne!(harness.state().host.get::<CommandForm>().expect("open").fields().text("name"), Some("given"));
    harness.event(Event::ModifiersChanged(Modifiers::ALT));
    harness.run();
    harness.get_by_label("Reset").focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::ALT, Key::Space);
    harness.run();
    assert_eq!(harness.state().host.get::<CommandForm>().expect("still open").fields().text("name"), Some("given"));
}
