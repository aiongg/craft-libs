//! Tests of the Slice tools' options bar (task SH-T9).

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::{SliceBar, SliceCommand, SliceMode, SliceOrigin, SliceToolSettings};
use craft_ui::canvas::Constraint;
use craft_ui::edit::EditEvent;
use craft_ui::theme::{Base, install};

struct App {
    mode: SliceMode,
    settings: SliceToolSettings,
    selected: Option<SliceOrigin>,
    styles: Vec<EditEvent<Constraint>>,
    commands: Vec<SliceCommand>,
}

fn harness(mode: SliceMode, selected: Option<SliceOrigin>) -> Harness<'static, App> {
    let app = App { mode, settings: SliceToolSettings::default(), selected, styles: Vec::new(), commands: Vec::new() };
    let mut h = Harness::builder().with_size([900.0, 200.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, app: &mut App| {
            let out = ui.horizontal(|ui| SliceBar::new("slice-bar", app.mode, &app.settings).selected(app.selected).show(ui)).inner;
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
