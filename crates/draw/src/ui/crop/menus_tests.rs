//! Tests of the Crop options bar's menus (task TO-C5).

use egui::accesskit::{Role, Toggled};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::{CropGuide, CropOptionsMenu, CropOverlayMenu, CropSettings, GuideShow};
use craft_ui::theme::{Base, install};

#[derive(Default)]
struct Host {
    settings: CropSettings,
    pick_color: Option<craft_ui::dialog::ColorRequest>,
    open: bool,
}

fn harness(settings: CropSettings, gear: bool) -> Harness<'static, Host> {
    let mut h = Harness::builder().with_size([480.0, 600.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        move |ui, host: &mut Host| {
            if gear {
                let out = CropOptionsMenu::new("gear", &mut host.settings.shield).show(ui, &mut host.open);
                host.pick_color = host.pick_color.take().or(out.pick_color);
            } else {
                CropOverlayMenu::new("overlay", &mut host.settings).show(ui);
            }
        },
        Host { settings, pick_color: None, open: false },
    );
    h.run_steps(3);
    h
}

fn open(h: &mut Harness<'_, Host>, button: &str) {
    h.get_by_label(button).click();
    h.run_steps(3);
}

fn row<'h>(h: &'h Harness<'_, Host>, role: Role, label: &'h str) -> egui_kittest::Node<'h> {
    h.get_by_role_and_label(role, label)
}

// The overlay menu checks the current overlay and show rule; a row sets it and closes; Cycle Orientation is disabled for Rule of Thirds and enabled for Triangle.
#[test]
fn overlay_menu_marks_the_current_overlay_and_show_rule() {
    let mut h = harness(CropSettings::default(), false);
    open(&mut h, "Set the overlay options for the Crop tool");
    let toggled = |h: &Harness<'_, Host>, label: &str| row(h, Role::MenuItemCheckBox, label).accesskit_node().toggled();
    assert_eq!(toggled(&h, "Rule of Thirds"), Some(Toggled::True));
    assert_eq!(toggled(&h, "Grid"), Some(Toggled::False));
    assert_eq!(toggled(&h, "Auto Show Overlay"), Some(Toggled::True));
    assert!(row(&h, Role::MenuItem, "Cycle Orientation").accesskit_node().is_disabled());

    row(&h, Role::MenuItemCheckBox, "Always Show Overlay").click();
    h.run_steps(3);
    assert_eq!(h.state().settings.guide_show, GuideShow::Always);
    assert!(h.query_by_role_and_label(Role::MenuItem, "Cycle Overlay").is_none(), "a row closes the menu");

    open(&mut h, "Set the overlay options for the Crop tool");
    row(&h, Role::MenuItemCheckBox, "Triangle").click();
    h.run_steps(3);
    assert_eq!(h.state().settings.guide, CropGuide::Triangle);

    open(&mut h, "Set the overlay options for the Crop tool");
    assert!(!row(&h, Role::MenuItem, "Cycle Orientation").accesskit_node().is_disabled());
    row(&h, Role::MenuItem, "Cycle Orientation").click();
    h.run_steps(3);
    assert_eq!(h.state().settings.guide_orientation, 1);
}

// With Classic Mode on, Show Cropped Area shows checked and disabled and Auto Center Preview is disabled; Custom… reports `pick_color`.
#[test]
fn gear_menu_greys_two_rows_in_classic_mode() {
    let mut settings = CropSettings::default();
    settings.shield.classic_mode = true;
    settings.shield.show_cropped_area = false;
    let mut h = harness(settings, true);
    open(&mut h, "Set additional Crop options");
    let area = row(&h, Role::CheckBox, "Show Cropped Area");
    assert!(area.accesskit_node().is_disabled());
    assert_eq!(area.accesskit_node().toggled(), Some(Toggled::True), "it acts as on");
    assert!(row(&h, Role::CheckBox, "Auto Center Preview").accesskit_node().is_disabled());
    assert!(!row(&h, Role::CheckBox, "Use Classic Mode").accesskit_node().is_disabled());
    assert!(h.state().pick_color.is_none());

    row(&h, Role::RadioButton, "Custom…").click();
    h.run_steps(3);
    assert!(h.state().pick_color.is_some());
    assert!(h.state().open, "the popover stays open on a click inside");
}
