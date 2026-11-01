//! Tests of the contour field (task AD-10).
#![cfg(not(target_arch = "wasm32"))]

use egui::Key;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use craft_ui::edit::EditEvent;
use craft_ui::theme::{Base, install};

use super::{ContourAction, ContourField};
use crate::layer_style::Contour;

/// A host that applies every contour the field reports and keeps the events.
struct Host {
    contour: Contour,
    user: Vec<Contour>,
    events: Vec<EditEvent<Contour>>,
    actions: Vec<ContourAction>,
    anti_alias: bool,
}

impl Host {
    fn kinds(&self) -> Vec<&'static str> {
        self.events
            .iter()
            .map(|e| match e {
                EditEvent::Begin { .. } => "Begin",
                EditEvent::Update { .. } => "Update",
                EditEvent::Commit { .. } => "Commit",
                EditEvent::Cancel { .. } => "Cancel",
                EditEvent::Set { .. } => "Set",
                _ => "other",
            })
            .collect()
    }
}

fn rig(user: Vec<Contour>) -> Harness<'static, Host> {
    let host = Host { contour: Contour::linear(), user, events: Vec::new(), actions: Vec::new(), anti_alias: false };
    let mut h = Harness::builder().with_size([800.0, 900.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            let shown = ContourField::new("contour", "Contour", &host.contour).anti_alias(Some(host.anti_alias)).user_contours(&host.user).show(ui);
            if let Some(event) = shown.edit {
                match &event {
                    EditEvent::Cancel { .. } => {}
                    other => {
                        if let Some(value) = other.value() {
                            host.contour = value.clone();
                        }
                    }
                }
                host.events.push(event);
            }
            if let Some(on) = shown.anti_alias {
                host.anti_alias = on;
            }
            host.actions.extend(shown.action);
        },
        host,
    );
    h.run_steps(3);
    h
}

// Picking Gaussian in the picker reports one Set with the preset; a click on the thumbnail opens
// the editor with the contour's points (as levels) and reports Begin; OK reports Commit named
// "Custom" after an edit, Cancel reports Cancel.
#[test]
fn contour_field_picks_a_preset_and_opens_the_editor_with_its_points() {
    let mut h = rig(Vec::new());
    h.get_by_role_and_label(Role::Button, "Contour presets").click();
    h.run_steps(4);
    h.get_by_role_and_label(Role::Button, "Gaussian").click();
    h.run_steps(4);
    let Some(gaussian) = Contour::preset("Gaussian") else { panic!("Gaussian ships") };
    assert_eq!(h.state().kinds(), ["Set"]);
    assert_eq!(h.state().contour, gaussian);

    h.get_by_role_and_label(Role::Button, "Contour Editor").click();
    h.run_steps(4);
    assert_eq!(h.state().kinds(), ["Set", "Begin"]);
    let graph = h.get_by(|n| n.role() == Role::Group && n.label().as_deref() == Some("Gaussian"));
    assert!(graph.value().is_some_and(|v| v.contains(&format!("{} points", gaussian.points.len()))), "{:?}", graph.value());

    graph.focus();
    h.run_steps(2);
    h.key_press(Key::Insert);
    h.run_steps(4);
    assert_eq!(h.state().kinds().last(), Some(&"Update"), "{:?}", h.state().kinds());
    let edited = h.state().contour.clone();
    assert_eq!((edited.name.as_str(), edited.points.len()), ("Custom", gaussian.points.len() + 1));

    h.get_by_role_and_label(Role::Button, "OK").click();
    h.run_steps(4);
    let Some(EditEvent::Commit { value, changed, .. }) = h.state().events.last() else { panic!("{:?}", h.state().kinds()) };
    assert_eq!((value.name.as_str(), *changed), ("Custom", true));
    assert!(h.query_by_role_and_label(Role::Button, "OK").is_none(), "the editor closed");

    h.get_by_role_and_label(Role::Button, "Contour Editor").click();
    h.run_steps(4);
    h.get_by_role_and_label(Role::Button, "Cancel").click();
    h.run_steps(4);
    assert_eq!(h.state().kinds().last(), Some(&"Cancel"), "{:?}", h.state().kinds());
}

// `to_curve` then `from_curve` returns the same points (within a level) and corners; a drawn
// curve gives None.
#[test]
fn contour_converts_to_and_from_the_editor_curve() {
    for preset in Contour::presets() {
        let curve = ContourField::to_curve(&preset);
        let Some(back) = ContourField::from_curve(&curve, &preset.name) else { panic!("{} converts back", preset.name) };
        assert_eq!(back.points.len(), preset.points.len(), "{}", preset.name);
        for (a, b) in back.points.iter().zip(&preset.points) {
            assert!((a.input - b.input).abs() * 255.0 < 1.0 && (a.output - b.output).abs() * 255.0 < 1.0, "{}", preset.name);
            assert_eq!(a.corner, b.corner, "{}", preset.name);
        }
    }
    let Some(points) = ContourField::to_curve(&Contour::linear()).points().map(<[_]>::to_vec) else { panic!("points") };
    assert_eq!(points, [[0.0, 0.0], [255.0, 255.0]]);
    assert!(ContourField::from_curve(&craft_ui::composites::Curve::Map(vec![0.0; 256]), "Drawn").is_none());
}

// The picker lists the host's contours after the built-ins; Save Contours… is off without any, and
// the library entries report their action.
#[test]
fn contour_picker_lists_user_contours_and_reports_library_actions() {
    let Some(mine) = Contour::from_points("Mine", Contour::linear().points) else { panic!("valid") };
    let mut h = rig(vec![mine.clone()]);
    h.get_by_role_and_label(Role::Button, "Contour presets").click();
    h.run_steps(4);
    let _ = h.get_by_role_and_label(Role::Button, "Mine");
    h.get_by_role_and_label(Role::Button, "Replace Contours…").click();
    h.run_steps(4);
    assert_eq!(h.state().actions, [ContourAction::Replace]);

    let mut h = rig(Vec::new());
    h.get_by_role_and_label(Role::Button, "Contour presets").click();
    h.run_steps(4);
    assert!(h.get_by_role_and_label(Role::Button, "Save Contours…").accesskit_node().is_disabled());
}
