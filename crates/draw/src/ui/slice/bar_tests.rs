//! Tests of the Slice tools' options bar (task SH-T9).

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::{SliceBar, SliceBarOutput, SliceCommand, SliceMode, SliceOrigin, SliceToolSettings};
use craft_ui::canvas::Constraint;
use craft_ui::edit::EditEvent;
use craft_ui::localized::units::{LengthUnit, UnitContext};
use craft_ui::patterns::OptionsBar;
use craft_ui::theme::{Base, install};

struct App {
    mode: SliceMode,
    settings: SliceToolSettings,
    selected: Option<SliceOrigin>,
    no_document: bool,
    units: Option<LengthUnit>,
    picked: Vec<LengthUnit>,
    styles: Vec<EditEvent<Constraint>>,
    commands: Vec<SliceCommand>,
}

fn harness(mode: SliceMode, selected: Option<SliceOrigin>) -> Harness<'static, App> {
    let app = App {
        mode,
        settings: SliceToolSettings::default(),
        selected,
        no_document: false,
        units: None,
        picked: Vec::new(),
        styles: Vec::new(),
        commands: Vec::new(),
    };
    let mut h = Harness::builder().with_size([900.0, 200.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, app: &mut App| {
            let mut out = SliceBarOutput::default();
            OptionsBar::new("bar").show(ui, |slots| {
                let context = UnitContext { pixels_per_inch: 300.0, ..UnitContext::default() };
                let mut bar =
                    SliceBar::new("slice-bar", app.mode, &app.settings).selected(app.selected).no_document(app.no_document).output(&mut out);
                if let Some(unit) = app.units {
                    bar = bar.size_units(unit, &LengthUnit::ALL, context, context);
                }
                bar.add_to(slots);
            });
            app.picked.extend(out.unit_picked);
            if let Some(event) = out.style {
                if let Some(&value) = event.value() {
                    app.settings.style = value;
                }
                app.styles.push(event);
            }
            if let Some(command) = out.command {
                if let SliceCommand::HideAuto(hide) = command {
                    app.settings.hide_auto = hide;
                }
                app.commands.push(command);
            }
        },
        app,
    );
    h.run_steps(2);
    h
}

fn pick_style(h: &mut Harness<'_, App>, name: &str) {
    h.get_by_role(Role::ComboBox).click();
    h.run_steps(2);
    h.get_by(|n| n.role() == Role::ListBoxOption && n.label().as_deref() == Some(name)).click();
    h.run_steps(3);
}

fn press(h: &mut Harness<'_, App>, label: &str) {
    h.get_by_role_and_label(Role::Button, label).click();
    h.run_steps(2);
}

fn disabled(h: &Harness<'_, App>, label: &str) -> bool {
    h.get_by_role_and_label(Role::Button, label).accesskit_node().is_disabled()
}

// Picking Fixed Size reports Size(64, 64), Normal and Fixed Ratio 1 × 1; Select mode's buttons
// report their commands and are disabled without a selection.
#[test]
fn slice_style_change_sets_default_size() {
    let mut h = harness(SliceMode::Slice, None);
    pick_style(&mut h, "Fixed Size");
    assert_eq!(h.state().settings.style, Constraint::Size(64.0, 64.0));
    pick_style(&mut h, "Fixed Ratio");
    assert_eq!(h.state().settings.style, Constraint::Ratio(1.0, 1.0));
    pick_style(&mut h, "Normal");
    assert_eq!(h.state().settings.style, Constraint::Free);
    press(&mut h, "Slices From Guides");
    assert_eq!(h.state().commands, [SliceCommand::FromGuides]);

    let h = harness(SliceMode::Select, None);
    for label in ["Promote", "Divide…", "Delete", "Slice Options…"] {
        assert!(disabled(&h, label), "{label}");
    }
    let mut h = harness(SliceMode::Select, Some(SliceOrigin::Auto));
    assert!(disabled(&h, "Delete"));
    for (label, command) in [("Promote", SliceCommand::Promote), ("Divide…", SliceCommand::Divide), ("Slice Options…", SliceCommand::Options)] {
        press(&mut h, label);
        assert_eq!(h.state().commands.last(), Some(&command), "{label}");
    }
    h.get_by_role_and_label(Role::CheckBox, "Hide Auto Slices").click();
    h.run_steps(2);
    assert_eq!(h.state().commands.last(), Some(&SliceCommand::HideAuto(true)));

    let mut h = harness(SliceMode::Select, Some(SliceOrigin::User));
    assert!(disabled(&h, "Promote"));
    press(&mut h, "Delete");
    assert_eq!(h.state().commands, [SliceCommand::Delete]);
}

// A layer slice is stored: Delete is enabled for it, as for a user slice, and the press reports the
// command; an auto slice has none.
#[test]
fn slice_bar_deletes_layer_slices() {
    let mut h = harness(SliceMode::Select, Some(SliceOrigin::Layer));
    assert!(!disabled(&h, "Delete"));
    press(&mut h, "Delete");
    assert_eq!(h.state().commands, [SliceCommand::Delete]);
    assert!(disabled(&harness(SliceMode::Select, Some(SliceOrigin::Auto)), "Delete"));
}

// Without a document Slices From Guides is greyed.
#[test]
fn slice_bar_greys_slices_from_guides_without_a_document() {
    let mut h = harness(SliceMode::Slice, None);
    assert!(!disabled(&h, "Slices From Guides"));
    h.state_mut().no_document = true;
    h.run_steps(2);
    assert!(disabled(&h, "Slices From Guides"));
}

// A unit picked from a Fixed Size field's menu reaches `SliceBarOutput::unit_picked` through
// `size_units`.
#[test]
fn slice_bar_reports_the_picked_size_unit() {
    let mut h = harness(SliceMode::Slice, None);
    h.state_mut().settings.style = Constraint::Size(300.0, 150.0);
    h.state_mut().units = Some(LengthUnit::Inches);
    h.run_steps(3);
    assert_eq!(h.get_by_role_and_label(Role::SpinButton, "Width").accesskit_node().data().value(), Some("1 in"));
    let at = h.get_by_role_and_label(Role::SpinButton, "Height").rect().center();
    h.event(egui::Event::PointerMoved(at));
    h.run();
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Secondary, pressed, modifiers: egui::Modifiers::NONE });
        h.run();
    }
    h.run_steps(3);
    h.get_by_label("Millimeters").click();
    h.run_steps(3);
    assert_eq!(h.state().picked, [LengthUnit::Millimetres]);
}
