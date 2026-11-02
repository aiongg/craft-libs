//! Tests of the session's options bar (task TR-T9).

use egui::Context;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use kurbo::Rect;

use super::{BarGroups, Interpolation, TransformBar, TransformEdit, TransformSession, TransformSettings};
use craft_ui::localized::units::{LengthUnit, UnitContext};
use craft_ui::theme::{Base, install};

const RECT: Rect = Rect::new(100.0, 80.0, 300.0, 220.0);

struct Host {
    session: TransformSession<()>,
    settings: TransformSettings,
    groups: BarGroups,
    mode_switch: bool,
    edits: Vec<String>,
    reset: bool,
    context: Option<UnitContext>,
}

fn host(session: TransformSession<()>) -> Host {
    Host {
        session,
        settings: TransformSettings::default(),
        groups: BarGroups::ALL,
        mode_switch: true,
        edits: Vec::new(),
        reset: false,
        context: None,
    }
}

fn harness(host: Host) -> Harness<'static, Host> {
    let mut h = Harness::<Host>::builder()
        .with_size([1800.0, 320.0])
        .with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, host: &mut Host| {
                let context = host.context;
                let out = ui
                    .horizontal(|ui| {
                        let mut bar =
                            TransformBar::new("bar", &host.session).groups(host.groups).mode_switch(host.mode_switch).settings(&mut host.settings);
                        if let Some(context) = context {
                            bar = bar.unit_context(context);
                        }
                        bar.show(ui)
                    })
                    .inner;
                host.reset |= out.reset_warp;
                for edit in out.edits {
                    host.edits.push(match &edit {
                        TransformEdit::Fields(_) => "fields".to_owned(),
                        TransformEdit::Interpolation(i) => format!("interpolation {}", i.id()),
                        TransformEdit::ReferencePoint(on) => format!("reference {on}"),
                        TransformEdit::WarpRequested => "warp".to_owned(),
                        TransformEdit::LeaveWarp => "leave warp".to_owned(),
                        _ => "other".to_owned(),
                    });
                    host.session.apply(edit, 0.0);
                }
                if out.apply {
                    host.edits.push("commit".to_owned());
                }
                if out.discard {
                    host.edits.push("cancel".to_owned());
                }
            },
            host,
        );
    h.run();
    h
}

fn warping() -> TransformSession<()> {
    let mut session = TransformSession::new(RECT);
    session.apply(TransformEdit::Warp(()), 0.0);
    session
}

fn left(h: &Harness<'_, Host>, label: &str) -> f32 {
    h.get_all_by_label(label).next().unwrap_or_else(|| panic!("{label}")).rect().left()
}

fn shown(h: &Harness<'_, Host>, label: &str) -> bool {
    h.query_all_by_label(label).next().is_some()
}

#[test]
fn the_bar_shows_fields_interpolation_check_and_buttons_in_phase_order() {
    let h = harness(host(TransformSession::new(RECT)));
    let order = [
        left(&h, "X position"),
        left(&h, "Width"),
        left(&h, "Interpolation:"),
        left(&h, "Show Reference Point"),
        left(&h, "Cancel transform"),
        left(&h, "Commit transform"),
        left(&h, "Switch between free transform and warp modes"),
    ];
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");

    // The Content-Aware subset: position and size, no switch, no check.
    let mut subset = host(TransformSession::new(RECT));
    subset.groups = BarGroups { reference: false, angle: false, skew: false, interpolation: false, ..BarGroups::ALL };
    subset.mode_switch = false;
    let h = harness(subset);
    assert!(shown(&h, "X position") && shown(&h, "Width") && shown(&h, "Commit transform"));
    assert!(!shown(&h, "Switch between free transform and warp modes") && !shown(&h, "Angle") && !shown(&h, "Interpolation:"));

    // The Warp phase: the app's group instead of the fields, Reset warp, the switch.
    let h = harness(host(warping()));
    assert!(!shown(&h, "X position") && !shown(&h, "Show Reference Point"));
    let order = [left(&h, "Cancel warp"), left(&h, "Commit warp"), left(&h, "Reset warp"), left(&h, "Switch between free transform and warp modes")];
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?}");
}

#[test]
fn commit_and_cancel_buttons_carry_the_phase_names_and_keys() {
    let mut h = harness(host(TransformSession::new(RECT)));
    assert!(shown(&h, "Commit transform") && shown(&h, "Cancel transform") && !shown(&h, "Commit warp"));
    h.get_by_label("Commit transform").click();
    h.run();
    h.get_by_label("Cancel transform").click();
    h.run();
    assert_eq!(h.state().edits, ["commit", "cancel"]);

    let h = harness(host(warping()));
    assert!(shown(&h, "Commit warp") && shown(&h, "Cancel warp") && !shown(&h, "Commit transform"));
    // The key names are in the tooltips (`OptionsBar::commit_cancel`).
    let mut h = h;
    h.get_by_label("Commit warp").hover();
    h.run();
    h.run();
    assert!(h.query_by_label_contains("Enter").is_some(), "the commit tooltip names Enter");
}

#[test]
fn the_check_and_the_interpolation_report_edits_and_write_the_settings() {
    let mut h = harness(host(TransformSession::new(RECT)));
    h.get_by_label("Show Reference Point").click();
    h.run();
    assert!(h.state().settings.show_reference_point && h.state().session.reference_point());
    assert_eq!(h.state().edits, ["reference true"]);

    h.get_by_label("Interpolation").click();
    h.run();
    h.get_by_label("Nearest Neighbor").click();
    h.run();
    assert_eq!(h.state().settings.interpolation, Interpolation::NearestNeighbor);
    assert_eq!(h.state().session.interpolation(), Interpolation::NearestNeighbor);
    assert_eq!(h.state().edits.last().map(String::as_str), Some("interpolation nearest"));
}

#[test]
fn the_switch_asks_for_a_warp_and_reset_asks_for_an_identity_warp() {
    let mut h = harness(host(TransformSession::new(RECT)));
    h.get_by_label("Switch between free transform and warp modes").click();
    h.run();
    assert_eq!(h.state().edits, ["warp"]);
    assert!(!h.state().reset);

    let mut h = harness(host(warping()));
    h.get_by_label("Reset warp").click();
    h.run();
    assert!(h.state().reset);
    h.get_by_label("Switch between free transform and warp modes").click();
    h.run();
    assert_eq!(h.state().edits, ["leave warp"]);
}

// A pick in the W menu changes the unit the fields show and is written to `size_unit`.
#[test]
fn a_unit_picked_in_the_size_menu_is_written_to_the_settings() {
    let mut start = host(TransformSession::new(RECT));
    start.context = Some(UnitContext::default());
    let mut h = harness(start);
    assert_eq!(h.state().settings.size_unit, None);
    h.run_steps(2);
    let at = h.get_by_role_and_label(Role::SpinButton, "Width").rect().center();
    h.event(egui::Event::PointerMoved(at));
    h.run();
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Secondary, pressed, modifiers: egui::Modifiers::NONE });
        h.run();
    }
    h.run_steps(3);
    let to = h.get_by_label("Millimeters").rect().center();
    h.event(egui::Event::PointerMoved(to));
    h.run();
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton { pos: to, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE });
        h.run();
    }
    h.run_steps(2);
    assert_eq!(h.state().settings.size_unit, Some(LengthUnit::Millimetres));
}
