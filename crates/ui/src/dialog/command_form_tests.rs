//! The command form in a harness.

use egui::accesskit::Role;
use egui::os::OperatingSystem;
use egui::{Event, Key, Modifiers, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::{CommandForm, DialogFields, DialogHost, DialogResult, FieldValue, Param, ParamKind, ParamResult, PathKind, PathRequest};
use crate::theme::{Base, install};

struct App {
    host: DialogHost,
    answers: Vec<DialogResult<ParamResult>>,
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
    let [DialogResult::Ok(ParamResult { key, fields })] = &harness.state().answers[..] else { panic!("one OK, got {:?}", harness.state().answers) };
    assert_eq!(key, "fit");
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

// A section under the rows reads the values; Browse reports the pressed row; confirm answers OK.
#[test]
fn the_form_draws_a_read_only_section_and_reports_browse() {
    let rows = vec![Param::new("dest", "Destination:", ParamKind::Text { default: "out".into() }).browse(PathKind::OpenFile)];
    let mut harness = harness(CommandForm::new("info", "File Info", rows).below(|ui, fields| {
        ui.label(format!("Camera: {}", fields.text("dest").unwrap_or_default()));
    }));
    harness.run();
    harness.get_by_label("Camera: out");
    harness.get_by_label("Browse…").click();
    harness.run();
    assert_eq!(
        harness.state_mut().host.get_mut::<CommandForm>().unwrap().pick_path(),
        Some(PathRequest { key: "dest".into(), kind: PathKind::OpenFile })
    );

    let ctx = harness.ctx.clone();
    assert!(harness.state_mut().host.confirm::<CommandForm>(&ctx));
    harness.run();
    let [DialogResult::Ok(ParamResult { key, .. })] = &harness.state().answers[..] else { panic!("one OK, got {:?}", harness.state().answers) };
    assert_eq!(key, "info");
}

// A bare number row has no slider, accepts any value on an open side and clamps to a bound.
#[test]
fn a_number_row_is_unbounded_on_open_sides_and_clamped_on_bounded_ones() {
    let rows = vec![Param::number("offset", "Offset:", 0.0, 0.5, 2), Param::number("count", "Count:", 3.0, 1.0, 0).bounds(Some(1.0), Some(10.0))];
    let mut harness = harness(CommandForm::new("shift", "Shift", rows));
    harness.run();
    assert!(harness.query_by_role(Role::Slider).is_none());
    harness.get_by_role_and_label(Role::SpinButton, "Offset:").focus();
    harness.run();
    harness.event(Event::Text("-5000.25".to_owned()));
    harness.key_press(Key::Tab);
    harness.run();
    harness.get_by_role_and_label(Role::SpinButton, "Count:").focus();
    harness.run();
    harness.event(Event::Text("99".to_owned()));
    harness.key_press(Key::Tab);
    harness.run();
    let fields = harness.state().host.get::<CommandForm>().expect("open").fields().clone();
    assert_eq!((fields.number("offset"), fields.number("count")), (Some(-5000.25), Some(10.0)));
}
