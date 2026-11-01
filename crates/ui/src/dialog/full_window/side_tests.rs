//! `FullWindowSide` in a `FullWindowDialog` harness.

use egui::accesskit::Role;
use egui::os::OperatingSystem;
use egui::{Rect, Vec2, vec2};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::dialog::{ButtonRole, DialogState, FooterButton, FullWindowDialog};
use crate::direction::{self, Direction};
use crate::theme::{Base, current, install};

const WINDOW: Vec2 = vec2(900.0, 500.0);

#[derive(Default)]
struct App {
    state: DialogState,
    rtl: bool,
    top_footer: bool,
    two: bool,
    rows: usize,
    header: Option<&'static str>,
    footer: Option<Rect>,
    columns: Vec<Rect>,
    body_bottom: f32,
}

fn buttons() -> Vec<FooterButton> {
    vec![FooterButton::new(ButtonRole::Default, "OK"), FooterButton::new(ButtonRole::Cancel, "Cancel")]
}

fn harness(app: App) -> Harness<'static, App> {
    HarnessBuilder::default()
        .with_size(WINDOW)
        .with_os(OperatingSystem::Nix)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, app: &mut App| {
                let ctx = ui.ctx().clone();
                direction::apply(&ctx, if app.rtl { Direction::Rtl } else { Direction::Ltr });
                let App { state, .. } = app;
                let mut seen = (None, Vec::new(), 0.0);
                let (top_footer, two, rows, header) = (app.top_footer, app.two, app.rows, app.header);
                FullWindowDialog::new("side").show(&ctx, state, &mut (), |ui, (), fx| {
                    let mut side = FullWindowSide::new("a").width(SideWidth::Wide);
                    if let Some(header) = header {
                        side = side.header(header);
                    }
                    let footer =
                        if top_footer { SideFooter::top(buttons()) } else { SideFooter::bottom(buttons()).leading(|ui| drop(ui.button("Reset"))) };
                    let out = side.footer(footer).show(ui, fx, |ui| {
                        for n in 0..rows {
                            ui.label(format!("row {n}"));
                        }
                        seen.2 = ui.min_rect().bottom();
                    });
                    seen.0 = out.footer.map(|f| f.responses.iter().map(|r| r.rect).reduce(|a, b| a.union(b)).unwrap());
                    seen.1.push(out.rect);
                    if two {
                        seen.1.push(FullWindowSide::new("b").show(ui, fx, |ui| drop(ui.label("second"))).rect);
                    }
                });
                (app.footer, app.columns, app.body_bottom) = (seen.0, seen.1, seen.2);
            },
            app,
        )
}

// `SideFooter::bottom` and `top`: the footer row's rect at the column's bottom or top, the
// buttons in the platform's order, the leading slot at the reading start.
#[test]
fn side_column_places_footer_at_bottom_or_top() {
    let mut bottom = harness(App { header: Some("Properties"), ..App::default() });
    bottom.run();
    let (column, footer) = (bottom.state().columns[0], bottom.state().footer.unwrap());
    assert!(footer.bottom() <= column.bottom() && column.bottom() - footer.bottom() < 20.0, "{footer:?} in {column:?}");
    assert!(footer.right() <= column.right() && column.right() - footer.right() < 20.0, "{footer:?} in {column:?}");
    let reset = bottom.get_by_label("Reset").rect();
    assert!(reset.left() < footer.left() && reset.left() - column.left() < 20.0, "Reset at the reading start: {reset:?}");

    let mut top = harness(App { top_footer: true, header: Some("Settings"), ..App::default() });
    top.run();
    let (column, footer) = (top.state().columns[0], top.state().footer.unwrap());
    assert!(footer.top() - column.top() < 80.0 && footer.top() > column.top(), "{footer:?} in {column:?}");
}

// Two columns stack from the end edge inwards, Standard and Wide widths from the tokens, and
// both mirror in a right-to-left interface.
#[test]
fn side_columns_stack_from_the_end_edge_inward_and_mirror_in_rtl() {
    let mut ltr = harness(App { two: true, ..App::default() });
    ltr.run();
    let style = FullWindowStyle::of(&current(&ltr.ctx));
    let [wide, standard] = ltr.state().columns[..] else { panic!("two columns") };
    assert!((wide.width() - style.side_width_wide).abs() < 1.5 && (standard.width() - style.side_width).abs() < 1.5, "{wide:?} {standard:?}");
    assert!((wide.right() - WINDOW.x).abs() < 1.5 && (standard.right() - wide.left()).abs() < 1.5, "{wide:?} {standard:?}");

    let mut rtl = harness(App { two: true, rtl: true, ..App::default() });
    rtl.run();
    let [wide, standard] = rtl.state().columns[..] else { panic!("two columns") };
    assert!(wide.left().abs() < 1.5 && (standard.left() - wide.right()).abs() < 1.5, "{wide:?} {standard:?}");
}

// A body taller than the column scrolls; the footer stays in view.
#[test]
fn side_body_scrolls_and_keeps_footer_in_view() {
    let mut harness = harness(App { rows: 200, ..App::default() });
    harness.run();
    let (column, footer) = (harness.state().columns[0], harness.state().footer.unwrap());
    assert!(column.bottom() <= WINDOW.y + 0.5 && footer.bottom() <= column.bottom(), "{footer:?} in {column:?}");
    assert!(harness.state().body_bottom > WINDOW.y, "the body is taller than the window and is clipped by a scroll area");
}

// The column is a Group named by its header, or "Settings" without one.
#[test]
fn side_column_is_a_group_named_by_its_header() {
    let mut named = harness(App { header: Some("Properties"), ..App::default() });
    named.run();
    named.get_by_role_and_label(Role::Group, "Properties");
    let mut plain = harness(App::default());
    plain.run();
    plain.get_by_role_and_label(Role::Group, "Settings");
}
