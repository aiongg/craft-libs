//! Panel footer tests: the reserved height, the bar over the body margin, button size and order,
//! and mirroring.

use std::cell::Cell;
use std::rc::Rc;

use egui::{Context, Margin, Rect, Ui, vec2};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, HarnessBuilder};

use super::panel_footer::show_layers;
use super::*;
use crate::direction::{self, Direction};
use crate::icons::Icon;
use crate::story::Args;
use crate::testing::a11y;
use crate::theme::{Base, Tokens, TokensOf as _, install};

fn harness<State>(dir: Direction) -> HarnessBuilder<State> {
    Harness::<State>::builder().with_size([640.0, 320.0]).with_context_setup(move |ctx: &Context| {
        install(ctx, &Base::suite_default());
        direction::apply(ctx, dir);
    })
}

#[allow(clippy::expect_used)]
fn style() -> PanelFooterStyle {
    let ctx = Context::default();
    install(&ctx, &Base::suite_default());
    let mut style = None;
    ctx.run_ui(egui::RawInput::default(), |ui| style = Some(PanelFooter::style(&Tokens::of(ui), ui.pixels_per_point())))
        .drop_without_applying_deltas();
    style.expect("style")
}

/// A panel body 200 wide inside its margin, with a footer last; records the frame's rect.
fn body(ui: &mut Ui, frame: &Cell<Rect>) {
    let margin = style().margin as i8;
    let shown = ui.allocate_ui(vec2(200.0, 0.0), |ui| {
        ui.set_max_width(200.0);
        egui::Frame::new().inner_margin(Margin::same(margin)).show(ui, |ui| {
            ui.label("Body");
            PanelFooter::show(ui, |ui| {
                PanelFooter::button(ui, Icon::Trash, "Delete");
                PanelFooter::button(ui, Icon::Plus, "New");
            });
        })
    });
    frame.set(shown.inner.response.rect);
}

/// PhotoCraft's bar is `FOOTER_BAR` under a rule, and runs on over the body's bottom margin to the
/// panel's bottom edge (`panel.rs:46-90`); the first button is at the reading end, 2 from the next.
#[test]
fn the_bar_ends_at_the_panel_edge_and_the_buttons_run_from_the_reading_end() {
    let style = style();
    let frame = Rc::new(Cell::new(Rect::NOTHING));
    let seen = frame.clone();
    let harness = harness(Direction::Ltr).build_ui(move |ui| body(ui, &seen));
    let frame = frame.get();
    let delete = harness.get_by_label("Delete").rect();
    let new = harness.get_by_label("New").rect();
    assert_eq!((delete.width(), delete.height()), (style.button, style.button));
    assert!((frame.right() - style.margin - delete.right()).abs() < 1.0, "first button at the reading end: {delete:?} in {frame:?}");
    assert!((delete.left() - new.right() - style.gap).abs() < 1.0, "gap: {delete:?} {new:?}");
    let bar_bottom = frame.bottom();
    assert!((delete.center().y - (bar_bottom - style.height / 2.0)).abs() < 1.0, "centred in the bar: {delete:?} bottom {bar_bottom}");
    let label = harness.get_by_label("Body").rect();
    assert!(new.top() >= label.bottom(), "the footer is under the body");
}

#[test]
fn the_reserved_height_is_the_rule_and_the_bar_less_the_body_margin() {
    let style = style();
    assert_eq!(PanelFooter::reserved_height(&style), style.rule.width + style.height - style.margin);
}

#[test]
fn the_buttons_run_from_the_right_edge_when_mirrored() {
    let style = style();
    let frame = Rc::new(Cell::new(Rect::NOTHING));
    let seen = frame.clone();
    let harness = harness(Direction::Rtl).build_ui(move |ui| body(ui, &seen));
    let frame = frame.get();
    let delete = harness.get_by_label("Delete").rect();
    let new = harness.get_by_label("New").rect();
    assert!((delete.left() - (frame.left() + style.margin)).abs() < 1.0, "first button at the left: {delete:?} in {frame:?}");
    assert!(new.left() > delete.left());
}

#[test]
fn the_story_names_every_button() {
    for dir in [Direction::Ltr, Direction::Rtl] {
        let mut args = Args::new().with_number("width", 240.0, 60.0, 400.0);
        let harness = harness(dir).build_ui(move |ui| show_layers(ui, &mut args));
        a11y::assert_labelled(&harness);
    }
}
