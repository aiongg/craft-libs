//! Tests of the pin bar (task TR-T12).

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use kurbo::Point;

use super::*;
use craft_ui::edit::EditEvent;
use craft_ui::patterns::OptionsBar;
use craft_ui::theme::{Base, install};

struct Quiet;

impl PinSolver for Quiet {
    fn rebind(&mut self, _: &[Pin], _: &PinWarpOptions) {}
    fn solve(&mut self, _: &[Pin], _: &PinWarpOptions) {}
    fn remesh(&mut self, _: &PinWarpOptions) -> Result<(), String> {
        Ok(())
    }
    fn mesh(&self) -> PinMeshView<'_> {
        PinMeshView { rest: &[], deformed: &[], triangles: &[], order: &[] }
    }
}

struct Host {
    session: PinWarpSession,
    notice: Option<String>,
    out: Vec<PinEdit>,
    apply: bool,
    discard: bool,
}

fn pin(x: f64) -> Pin {
    Pin { rest: Point::new(x, 0.0), at: Point::new(x, 0.0), angle: None, depth: 0 }
}

fn harness(selected: bool, notice: Option<&str>) -> Harness<'static, Host> {
    let mut session = PinWarpSession::from_pins(vec![pin(10.0), pin(20.0)], &PinWarpSettings::default());
    if selected {
        session.apply(PinEdit::Select(vec![1]), &mut Quiet, 0.0).expect("select");
    }
    let host = Host { session, notice: notice.map(str::to_owned), out: Vec::new(), apply: false, discard: false };
    let mut h = Harness::builder().with_size([1700.0, 80.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            let mut r = PinWarpBarOutput::default();
            OptionsBar::new("options")
                .show(ui, |slots| PinWarpBar::new("bar", &host.session).notice(host.notice.as_deref()).output(&mut r).add_to(slots));
            host.apply |= r.apply;
            host.discard |= r.discard;
            host.out.extend(r.edits);
        },
        host,
    );
    h.run_steps(3);
    h
}

fn pins_of(edit: &PinEdit) -> &[Pin] {
    match edit {
        PinEdit::Pins(event) => event.value().map_or(&[], Vec::as_slice),
        _ => &[],
    }
}

// The + and − buttons report the selected pin's depth up and down by one; they show only with a
// selected pin.
#[test]
fn pins_depth_buttons_change_the_selected_pins_depth() {
    let h = harness(false, None);
    assert!(h.query_by_label("Bring the selected pin forward").is_none());

    let mut h = harness(true, None);
    h.get_by_label("Bring the selected pin forward").click();
    h.run_steps(3);
    h.get_by_label("Send the selected pin backward").click();
    h.run_steps(3);
    let depths: Vec<Vec<i32>> = h.state().out.iter().map(|e| pins_of(e).iter().map(|p| p.depth).collect()).collect();
    assert_eq!(depths, vec![vec![0, 1], vec![0, -1]]);
}

// Mode, Density and Expansion report Options; Show Mesh reports ShowMesh; a notice shows its text.
#[test]
fn the_options_report_edits_and_a_refused_remesh_shows_its_notice() {
    let mut h = harness(false, Some("Could not use Puppet Warp"));
    assert!(h.query_by_label("Could not use Puppet Warp").is_some());
    h.get_by_role_and_label(Role::ComboBox, "Mode").click();
    h.run_steps(2);
    h.get_by(|n| n.role() == Role::ListBoxOption && n.label().as_deref() == Some("Rigid")).click();
    h.run_steps(3);
    h.get_by_role_and_label(Role::ComboBox, "Density").click();
    h.run_steps(2);
    h.get_by(|n| n.role() == Role::ListBoxOption && n.label().as_deref() == Some("More Points")).click();
    h.run_steps(3);
    h.get_by_role_and_label(Role::SpinButton, "Expansion").click();
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("12".into()));
    h.run_steps(2);
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    h.get_by_label("Show Mesh").click();
    h.run_steps(3);
    let out = &h.state().out;
    assert!(matches!(&out[0], PinEdit::Options(o) if o.mode == PinMode::Rigid), "{out:?}");
    assert!(matches!(&out[1], PinEdit::Options(o) if o.density == PinDensity::More), "{out:?}");
    assert!(matches!(&out[2], PinEdit::Options(o) if o.expansion == 12.0), "{out:?}");
    assert_eq!(out.len(), 4, "{out:?}");
    assert_eq!(out[3], PinEdit::ShowMesh(false));
}

// Rotate Fixed reports the pin with angle 0, the degrees field sets it, Auto clears it; Remove All
// Pins reports an empty pin list.
#[test]
fn rotation_and_remove_all_report_pin_lists() {
    let mut h = harness(true, None);
    h.get_by_role_and_label(Role::ComboBox, "Rotation").click();
    h.run_steps(2);
    h.get_by(|n| n.role() == Role::ListBoxOption && n.label().as_deref() == Some("Fixed")).click();
    h.run_steps(3);
    assert_eq!(pins_of(&h.state().out[0])[1].angle, Some(0.0));

    // The host applies the edit; the degrees field and Auto then show.
    let edits = std::mem::take(&mut h.state_mut().out);
    for edit in edits {
        h.state_mut().session.apply(edit, &mut Quiet, 0.0).expect("applies");
    }
    h.run_steps(3);
    h.get_by_role_and_label(Role::SpinButton, "Rotation").click();
    h.run_steps(2);
    h.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    h.event(egui::Event::Text("45".into()));
    h.run_steps(2);
    h.key_press(egui::Key::Enter);
    h.run_steps(3);
    let angle = h.state().out.iter().rev().find_map(|e| match e {
        PinEdit::Pins(EditEvent::Commit { value, .. } | EditEvent::Set { value, .. }) => value[1].angle,
        _ => None,
    });
    assert_eq!(angle, Some(45.0), "{:?}", h.state().out);

    h.state_mut().out.clear();
    h.get_by_label("Remove All Pins").click();
    h.run_steps(3);
    assert!(matches!(&h.state().out[..], [PinEdit::Pins(EditEvent::Set { value, .. })] if value.is_empty()), "{:?}", h.state().out);

    h.get_by_label("Commit Puppet Warp").click();
    h.run_steps(3);
    assert!(h.state().apply);
    h.get_by_label("Cancel Puppet Warp").click();
    h.run_steps(3);
    assert!(h.state().discard);
}
