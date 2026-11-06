//! `FullWindowZoomBar` in a harness.

use egui::accesskit::Role;
use egui::os::OperatingSystem;
use egui::{Rect, Vec2, pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::{FullWindowView, FullWindowZoomBar};
use crate::theme::{Base, install};

const WINDOW: Vec2 = vec2(600.0, 300.0);
const IMAGE: [u32; 2] = [200, 100];

#[derive(Default)]
struct App {
    view: FullWindowView,
    progress: Option<Option<f32>>,
    note: Option<&'static str>,
    changed: bool,
}

fn harness() -> Harness<'static, App> {
    let mut h = HarnessBuilder::default()
        .with_size(WINDOW)
        .with_os(OperatingSystem::Nix)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, app: &mut App| {
                // The canvas records the image size and the fitted zoom; the bar steps from them.
                app.view.canvas_view(Rect::from_min_max(pos2(0.0, 0.0), pos2(500.0, 250.0)), IMAGE, 20.0, 1.0);
                let mut bar = FullWindowZoomBar::new("bar", &mut app.view).progress(app.progress);
                if let Some(note) = app.note {
                    bar = bar.note(note);
                }
                app.changed |= bar.show(ui).changed;
            },
            App::default(),
        );
    h.run();
    h
}

// Zoom In and Zoom Out step the ladder and disable at its ends; the select lists an off-menu
// zoom in its place and Fit in View fits.
#[test]
fn zoom_bar_steps_the_ladder_and_shows_an_off_menu_value() {
    let mut h = harness();
    assert!(h.state().view.is_fit());
    let fitted = h.state().view.zoom();

    h.get_by_label("Zoom In").click();
    h.run();
    let stepped = h.state().view.zoom();
    assert!(!h.state().view.is_fit() && stepped > fitted, "{stepped} after {fitted}");
    assert!(h.state().changed);

    h.get_by_label("Zoom Out").click();
    h.run();
    assert!(h.state().view.zoom() < stepped);

    // The ends of the ladder disable their button.
    h.state_mut().view.set_zoom(1e-9);
    h.run();
    assert!(h.get_by_label("Zoom Out").accesskit_node().is_disabled());
    assert!(!h.get_by_label("Zoom In").accesskit_node().is_disabled());
    h.state_mut().view.set_zoom(1e9);
    h.run();
    assert!(h.get_by_label("Zoom In").accesskit_node().is_disabled());

    // 300 % is not on the menu: it is listed as itself, between 200 % and 400 %.
    h.state_mut().view.set_zoom(3.0);
    h.run();
    h.get_by_role_and_label(Role::ComboBox, "Zoom").click();
    h.run();
    let options: Vec<String> = h.query_all_by_role(Role::ListBoxOption).filter_map(|n| n.accesskit_node().label()).collect();
    let at = |text: &str| options.iter().position(|o| o.as_str() == text).unwrap_or_else(|| panic!("{text} in {options:?}"));
    assert!(at("200%") < at("300%") && at("300%") < at("400%"), "{options:?}");
    assert_eq!(options[0], "Fit in View");

    h.get_by_role_and_label(Role::ListBoxOption, "Fit in View").click();
    h.run();
    assert!(h.state().view.is_fit());
}

// A progress shows the bar (indeterminate for `Some(None)`); without one the note shows.
#[test]
fn zoom_bar_shows_progress_or_the_note() {
    let mut h = harness();
    h.state_mut().note = Some("Preview 120 ms");
    h.run();
    assert!(h.query_by_label("Preview 120 ms").is_some());
    assert!(h.query_by_role(Role::ProgressIndicator).is_none());

    // An indeterminate bar repaints for ever, so the harness steps instead of settling.
    h.state_mut().progress = Some(None);
    h.step();
    h.step();
    assert!(h.query_by_role(Role::ProgressIndicator).is_some());
    assert!(h.query_by_label("Preview 120 ms").is_none());

    h.state_mut().progress = Some(Some(0.5));
    h.run();
    assert!(h.query_by_role(Role::ProgressIndicator).is_some());
}
