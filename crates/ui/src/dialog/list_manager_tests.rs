//! `ListManagerDialog`.

use egui::{Event, Key};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::dialog::DialogHost;
use crate::theme::{Base, install};

struct App {
    host: DialogHost,
    answers: Vec<DialogResult<ListManagerChoice>>,
}

fn entry(id: &str, name: &str, kind: u32) -> ListManagerEntry {
    ListManagerEntry { id: id.into(), name: name.into(), detail: None, description: format!("About {name}"), kind }
}

fn entries() -> Vec<ListManagerEntry> {
    vec![entry("a", "Alpha", 1), ListManagerEntry { detail: Some("Ctrl+B".into()), ..entry("b", "Beta", 1) }, entry("g", "Gamma", 2)]
}

fn dialog(searchable: bool) -> ListManagerDialog {
    let labels = ListManagerLabels::default();
    ListManagerDialog::new("macros", "Macros", "Macro name:")
        .entries(entries())
        .actions(vec![ListManagerAction::run(&labels), ListManagerAction::edit(&labels).kind(1), ListManagerAction::create(&labels), ListManagerAction::delete(&labels)])
        .searchable(searchable)
}

fn harness(dialog: ListManagerDialog) -> Harness<'static, App> {
    let mut host = DialogHost::default();
    host.open(dialog);
    HarnessBuilder::default()
        .with_size(egui::vec2(640.0, 480.0))
        .with_step_dt(0.05)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, app: &mut App| {
                app.host.show(ui.ctx());
                if let Some(answer) = app.host.take::<ListManagerDialog>() {
                    app.answers.push(answer);
                }
            },
            App { host, answers: Vec::new() },
        )
}

fn type_name(harness: &mut Harness<'static, App>, text: &str) {
    harness.get_by_role_and_label(egui::accesskit::Role::TextInput, "Macro name:").focus();
    harness.run();
    harness.event(Event::Text(text.to_owned()));
    harness.run();
}

fn disabled(harness: &Harness<'static, App>, label: &str) -> bool {
    harness.get_by_label(label).accesskit_node().is_disabled()
}

// Typing a name selects its entry (case ignored) and enables Run and Delete; a new name enables
// Create only; Up/Down move the selection; Enter runs; double-click runs.
#[test]
fn name_box_selects_and_enables_actions() {
    let mut harness = harness(dialog(false));
    harness.run();
    assert!(disabled(&harness, "Run") && disabled(&harness, "Create") && disabled(&harness, "Delete"));
    type_name(&mut harness, "bETA");
    assert!(!disabled(&harness, "Run") && !disabled(&harness, "Edit") && !disabled(&harness, "Delete") && disabled(&harness, "Create"));
    harness.get_by_label_contains("About Beta");

    harness.key_press(Key::ArrowDown);
    harness.run();
    harness.get_by_label_contains("About Gamma");
    assert!(disabled(&harness, "Edit"), "Edit takes entries of kind 1 only");
    harness.key_press(Key::ArrowUp);
    harness.key_press(Key::ArrowUp);
    harness.run();
    harness.get_by_label_contains("About Alpha");

    harness.key_press(Key::Enter);
    harness.run();
    let Some(DialogResult::Ok(choice)) = harness.state_mut().answers.pop() else { panic!("Enter runs the first action") };
    assert_eq!(choice, ListManagerChoice { action: "run".into(), entry: Some("a".into()), name: "Alpha".into() });

    let mut harness = self::harness(dialog(false));
    harness.run();
    type_name(&mut harness, " brand new ");
    assert!(disabled(&harness, "Run") && !disabled(&harness, "Create"));
    harness.get_by_label("Create").click();
    harness.run();
    let Some(DialogResult::Ok(choice)) = harness.state_mut().answers.pop() else { panic!("Create answers") };
    assert_eq!(choice, ListManagerChoice { action: "create".into(), entry: None, name: "brand new".into() });

    let mut harness = self::harness(dialog(false));
    harness.run();
    harness.get_by_label("Gamma").click();
    harness.run();
    assert!(!disabled(&harness, "Delete"), "a click puts the row's name in the box");
    harness.run_steps(30); // the double-click delay passes
    let at = harness.get_by_label("Gamma").rect().center();
    for _ in 0..2 {
        harness.event(Event::PointerMoved(at));
        for pressed in [true, false] {
            harness.event(Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE });
        }
        harness.run_steps(1);
    }
    harness.run();
    let Some(DialogResult::Ok(choice)) = harness.state_mut().answers.pop() else { panic!("a double click runs") };
    assert_eq!((choice.action.as_str(), choice.entry.as_deref()), ("run", Some("g")));
}

// Delete keeps the dialog open and is read with take_action; set_entries replaces the rows and
// keeps the name; searchable filters rows by folded substring.
#[test]
fn keep_open_actions_and_live_rows() {
    let mut harness = harness(dialog(false));
    harness.run();
    type_name(&mut harness, "Gamma");
    harness.get_by_label("Delete").click();
    harness.run();
    assert!(harness.state().answers.is_empty(), "Delete keeps the dialog open");
    let choice = harness.state_mut().host.get_mut::<ListManagerDialog>().expect("still open").take_action().expect("pressed");
    assert_eq!((choice.action.as_str(), choice.entry.as_deref()), ("delete", Some("g")));
    harness.state_mut().host.get_mut::<ListManagerDialog>().expect("open").set_entries(vec![entry("a", "Alpha", 1)]);
    harness.run();
    assert!(disabled(&harness, "Delete"), "the name box keeps `Gamma`, which no row has now");
    assert!(!disabled(&harness, "Create"));

    let mut harness = self::harness(dialog(true));
    harness.run();
    type_name(&mut harness, "ETA");
    assert!(harness.query_by_label("Alpha").is_none() && harness.query_by_label("Gamma").is_none());
    harness.get_by_label_contains("Beta");
}

// An empty list says so in italics, as GridCraft's Macros dialog does.
#[test]
fn the_empty_list_text_is_italic() {
    fn italics(shape: &egui::Shape, out: &mut Vec<bool>) {
        match shape {
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| italics(s, out)),
            egui::Shape::Text(text) if text.galley.text() == "No macros here." => out.push(text.galley.job.sections[0].format.italics),
            _ => {}
        }
    }
    let ctx = egui::Context::default();
    install(&ctx, &Base::suite_default());
    let mut host = DialogHost::default();
    host.open(ListManagerDialog::new("macros", "Macros", "Macro name:").empty_text("No macros here."));
    let mut found = Vec::new();
    for _ in 0..3 {
        found.clear();
        let output = ctx.run_ui(
            egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(640.0, 480.0))), ..Default::default() },
            |ui| {
                host.show(ui.ctx());
            },
        );
        output.shapes.iter().for_each(|clipped| italics(&clipped.shape, &mut found));
        output.drop_without_applying_deltas();
    }
    assert_eq!(found, vec![true]);
}
