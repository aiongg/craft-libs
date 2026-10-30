//! The embedded path of [`ViewportWindow`] in a harness; kittest has no OS viewports.

use egui::accesskit::Role;
use egui::{Id, Key, Vec2, ViewportBuilder, ViewportId, vec2};
use egui_kittest::kittest::Queryable as _;
use egui_kittest::{Harness, HarnessBuilder};

use super::*;
use crate::theme::{Base, install};

const WINDOW: Vec2 = vec2(640.0, 480.0);

struct App {
    open: bool,
    modal: bool,
    scrim: bool,
    behind_clicks: usize,
    hostings: Vec<Hosting>,
    closes: usize,
    blocks: bool,
    blocks_at_close: Option<bool>,
}

fn harness(modal: bool) -> Harness<'static, App> {
    let app = App { open: true, modal, scrim: true, behind_clicks: 0, hostings: Vec::new(), closes: 0, blocks: false, blocks_at_close: None };
    HarnessBuilder::default().with_size(WINDOW).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, app: &mut App| {
            let ctx = &ui.ctx().clone();
            app.blocks = ViewportWindow::blocks_main(ctx);
            if ui.button("Behind").clicked() {
                app.behind_clicks += 1;
            }
            if !app.open {
                return;
            }
            let builder = ViewportBuilder::default().with_inner_size(vec2(300.0, 200.0)).with_min_inner_size(vec2(200.0, 120.0));
            let mut window = ViewportWindow::new(ViewportId(Id::unique("test")), builder).title("Settings").embedded(true);
            if app.modal {
                window = window.app_modal(app.scrim);
            }
            let hostings = &mut app.hostings;
            let shown = window.show(ctx, |ui, hosting| {
                hostings.push(hosting);
                ui.label("Body");
            });
            app.closes += usize::from(shown.close_requested);
            if shown.close_requested {
                app.blocks_at_close = Some(ViewportWindow::blocks_main(ctx));
            }
        },
        app,
    )
}

#[test]
fn the_embedded_window_has_a_title_a_close_box_and_an_embedded_body() {
    let mut harness = harness(false);
    harness.run();
    harness.get_by_role_and_label(Role::Heading, "Settings");
    harness.get_by_label("Body");
    assert!(harness.state().hostings.iter().all(|h| *h == Hosting::Embedded));
    harness.get_by_label("Close").click();
    harness.run();
    assert_eq!(harness.state().closes, 1);
}

#[test]
fn the_window_opens_at_the_builders_size() {
    let mut harness = harness(false);
    harness.run();
    let width = harness.get_by_label("Close").rect().right() - harness.get_by_role_and_label(Role::Heading, "Settings").rect().left();
    assert!((width - 300.0).abs() < 1.5, "{width}");
}

#[test]
fn an_app_modal_window_blocks_the_main_ui_and_reports_it() {
    let mut harness = harness(true);
    harness.run();
    harness.get_by_label("Behind").click();
    harness.run();
    assert_eq!(harness.state().behind_clicks, 0);
    assert!(harness.state().blocks);

    harness.state_mut().open = false;
    harness.run();
    harness.run();
    harness.run();
    assert!(!harness.state().blocks, "the flag follows the window");
    harness.get_by_label("Behind").click();
    harness.run();
    assert_eq!(harness.state().behind_clicks, 1);
}

#[test]
fn a_modal_stops_blocking_in_the_pass_that_reports_its_close() {
    let mut harness = harness(true);
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    // Read in the pass that reported the close; the host would stop showing the window now.
    assert_eq!(harness.state().blocks_at_close, Some(false));
}

#[test]
fn esc_closes_an_app_modal_window_only() {
    let mut harness = harness(true);
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().closes, 1);

    let mut harness = self::harness(false);
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().closes, 0);
}

#[test]
fn a_window_shown_again_after_a_gap_opens_afresh() {
    let mut harness = harness(false);
    harness.run();
    harness.get_by_label("Close").click();
    harness.run();
    harness.state_mut().open = false;
    harness.run();
    harness.run();
    harness.state_mut().open = true;
    harness.run();
    harness.run();
    harness.get_by_role_and_label(Role::Heading, "Settings");
}
