//! `AlignBar`.

use egui::accesskit::Role;
use egui::{Context, Ui};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::*;
use crate::icons::IconSet;
use crate::theme::{Base, install};

#[derive(Default)]
struct Host {
    picked: Vec<AlignAction>,
}

fn harness(distribute: DistributeButtons) -> Harness<'static, Host> {
    Harness::<Host>::builder().with_size([400.0, 400.0]).with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default())).build_ui_state(
        move |ui: &mut Ui, host: &mut Host| {
            let bar = AlignBar::new("test").distribute(distribute).enabled(|a| a != AlignAction::AlignTopEdges).show(ui);
            host.picked.extend(bar.picked);
        },
        Host::default(),
    )
}

// Six buttons named "Align: Left Edges"…, disabled when enabled() says so; a click picks its
// action; the ⋯ menu lists Align then Distribute and a pick closes it.
#[test]
fn align_buttons_and_menu_pick_actions() {
    let mut h = harness(DistributeButtons::Menu);
    h.run();
    assert!(h.query_by_label("Distribute: Left Edges").is_none(), "distribute commands sit in the menu only");
    assert!(h.get_by_label("Align: Top Edges").accesskit_node().is_disabled());
    h.get_by_label("Align: Top Edges").click();
    h.run();
    assert!(h.state().picked.is_empty(), "a disabled button picks nothing");
    h.get_by_label("Align: Left Edges").click();
    h.run();
    assert_eq!(h.state().picked, [AlignAction::AlignLeftEdges]);

    h.get_by_label("Align and distribute").click();
    h.run();
    let item = h.get_by_role_and_label(Role::MenuItem, "Distribute: Vertically");
    item.click();
    h.run();
    assert_eq!(h.state().picked, [AlignAction::AlignLeftEdges, AlignAction::DistributeVertically]);
    assert!(h.query_by_role_and_label(Role::MenuItem, "Distribute: Vertically").is_none(), "a pick closes the menu");
}

// With DistributeButtons::Row the eight distribute buttons follow the align ones.
#[test]
fn row_shows_the_distribute_buttons() {
    let mut h = harness(DistributeButtons::Row);
    h.run();
    h.get_by_role_and_label(Role::Button, "Distribute: Horizontally").click();
    h.run();
    assert_eq!(h.state().picked, [AlignAction::DistributeHorizontally]);
}

// Every action has an icon in the set and a label; ALIGN and DISTRIBUTE cover all 14 once.
#[test]
fn every_action_has_an_icon_and_a_label() {
    let all: Vec<AlignAction> = AlignAction::ALIGN.into_iter().chain(AlignAction::DISTRIBUTE).collect();
    assert_eq!(all.len(), 14);
    let unique: std::collections::HashSet<_> = all.iter().copied().collect();
    assert_eq!(unique.len(), 14);
    let labels = AlignLabels::default();
    let icons: std::collections::HashSet<_> = all.iter().map(|a| a.icon()).collect();
    assert_eq!(icons.len(), 14, "each action has its own icon");
    for action in all {
        assert!(!action.label(&labels).is_empty(), "{action:?}");
        for set in IconSet::ALL {
            assert!(!action.icon().svg(set).is_empty(), "{action:?} in {set:?}");
        }
    }
}
