//! Tests of the painting symmetry menu (task TO-S3).

use egui::Context;
use egui::accesskit::{Role, Toggled};
use egui_kittest::Harness;
use egui_kittest::Node;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::{SymmetryChoice, SymmetryCommand, SymmetryMenu, SymmetryMode, SymmetryPathRow};
use craft_ui::theme::{Base, install};

struct Host {
    choice: SymmetryChoice,
    paths: Vec<SymmetryPathRow>,
    can_transform: bool,
    commands: Vec<SymmetryCommand>,
}

fn host(choice: SymmetryChoice) -> Host {
    let paths = vec![SymmetryPathRow { key: "work".to_owned(), label: "Work Path".to_owned() }];
    Host { choice, paths, can_transform: false, commands: Vec::new() }
}

fn harness(host: Host) -> Harness<'static, Host> {
    let mut h =
        Harness::<Host>::builder().with_size([400.0, 400.0]).with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, host: &mut Host| {
                let out = SymmetryMenu::new("symmetry").current(&host.choice).paths(&host.paths).can_transform(host.can_transform).show(ui);
                host.commands.extend(out.command);
            },
            host,
        );
    h.run();
    h
}

fn open(h: &mut Harness<'_, Host>) {
    h.get_by_label("Set painting symmetry options").click();
    h.run();
}

fn row<'a>(h: &'a Harness<'_, Host>, label: &'a str) -> Node<'a> {
    h.get_by(|n| n.label().as_deref() == Some(label) && matches!(n.role(), Role::MenuItem | Role::MenuItemRadio | Role::MenuItemCheckBox))
}

#[test]
fn rows_check_the_current_choice_and_run_commands() {
    let mut h = harness(host(SymmetryChoice::Mode(SymmetryMode::Dual)));
    open(&mut h);
    assert_eq!(row(&h, "Dual Axis").accesskit_node().toggled(), Some(Toggled::True));
    assert_eq!(row(&h, "Vertical").accesskit_node().toggled(), Some(Toggled::False));
    assert_eq!(row(&h, "Symmetry Off").accesskit_node().toggled(), Some(Toggled::False));
    assert!(!row(&h, "Symmetry Off").accesskit_node().is_disabled());
    assert!(row(&h, "Transform Symmetry").accesskit_node().is_disabled(), "no transform without can_transform");
    row(&h, "Horizontal").click();
    h.run();
    assert_eq!(h.state().commands, [SymmetryCommand::Mode(SymmetryMode::Horizontal)]);
    assert!(h.query_by_label("Horizontal").is_none(), "the menu closed");

    // Off: Symmetry Off is checked and disabled; a path row is checked when it is the source.
    let mut h = harness(host(SymmetryChoice::Off));
    open(&mut h);
    assert_eq!(row(&h, "Symmetry Off").accesskit_node().toggled(), Some(Toggled::True));
    assert!(row(&h, "Symmetry Off").accesskit_node().is_disabled());
    row(&h, "Work Path").click();
    h.run();
    assert_eq!(h.state().commands, [SymmetryCommand::FromPath("work".to_owned())]);

    let mut state = host(SymmetryChoice::Path("work".to_owned()));
    state.can_transform = true;
    let mut h = harness(state);
    open(&mut h);
    assert_eq!(row(&h, "Work Path").accesskit_node().toggled(), Some(Toggled::True));
    row(&h, "Transform Symmetry").click();
    h.run();
    assert_eq!(h.state().commands, [SymmetryCommand::Transform]);
}

#[test]
fn the_button_is_pressed_while_a_symmetry_is_on() {
    let h = harness(host(SymmetryChoice::Mode(SymmetryMode::Diagonal)));
    let node = h.get_by_label("Set painting symmetry options");
    assert_eq!(node.accesskit_node().toggled(), Some(Toggled::True));
    assert_eq!(node.accesskit_node().value().as_deref(), Some("Diagonal"));

    let h = harness(host(SymmetryChoice::Off));
    let node = h.get_by_label("Set painting symmetry options");
    assert_ne!(node.accesskit_node().toggled(), Some(Toggled::True));
    assert_eq!(node.accesskit_node().value(), None);
}
