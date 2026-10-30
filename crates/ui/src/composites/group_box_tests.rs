//! Group box tests: the Group node named by its title, and the content inside the outline.

use egui::accesskit::Role;
use egui::{Context, Ui};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::group_box::{GroupArgs, show_default};
use super::*;
use crate::theme::{Base, Tokens, TokensOf as _, install};

fn harness() -> Harness<'static, GroupArgs> {
    Harness::builder()
        .with_size([400.0, 300.0])
        .with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()))
        .build_ui_state(|ui: &mut Ui, args| show_default(ui, args), GroupArgs { second: true, width: 320.0 })
}

/// Each box is a Group named by its title, and the next box starts a padding on both sides of
/// the outline plus the box gap below the last row.
#[test]
fn a_box_is_a_group_named_by_its_title() {
    let harness = harness();
    harness.get_by_role_and_label(Role::Group, "Group 1");
    harness.get_by_role_and_label(Role::Group, "Group 2");
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut style = None;
    ctx.run_ui(egui::RawInput::default(), |ui| style = Some(GroupBox::style(&Tokens::of(ui), 1.0))).drop_without_applying_deltas();
    let style = style.expect("the pass ran the closure");
    let last_row = harness.get_all_by_label("Another row").next().expect("the last row").rect();
    let next_title = harness.get_by_role_and_label(Role::Label, "Group 2").rect();
    assert!(next_title.top() - last_row.bottom() >= 2.0 * style.padding + style.gap - 0.5, "{last_row:?} then {next_title:?}");
}

#[test]
fn boxes_that_do_not_fill_sit_side_by_side_and_a_title_check_box_enables_the_content() {
    let harness =
        Harness::builder().with_size([600.0, 200.0]).with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default())).build_ui_state(
            |ui: &mut Ui, on: &mut [bool; 2]| {
                let [a, b] = on;
                ui.horizontal_top(|ui| {
                    GroupBox::new("Columns").checkbox(a).fill(false).show(ui, |ui| {
                        let _ = ui.button("In columns");
                    });
                    GroupBox::new("Rows").checkbox(b).fill(false).show(ui, |ui| {
                        let _ = ui.button("In rows");
                    });
                });
            },
            [true, false],
        );
    // Group nodes carry no bounds here, so the boxes are placed by their title check boxes and contents.
    let first = harness.get_by_label("In columns").rect().union(harness.get_by_role_and_label(Role::CheckBox, "Columns").rect());
    let second = harness.get_by_role_and_label(Role::CheckBox, "Rows").rect();
    assert!(first.right() < second.left() && first.width() < 300.0, "{first:?} {second:?}");
    assert!(!harness.get_by_label("In columns").accesskit_node().is_disabled());
    assert!(harness.get_by_label("In rows").accesskit_node().is_disabled());
    assert_eq!(harness.get_all_by_role_and_label(Role::CheckBox, "Rows").count(), 1);
}
