//! The harmony wheel and the Color Guide in a kittest harness.

use craft_theme::Srgba;
use egui::accesskit::Role;
use egui::{Event, Modifiers, PointerButton};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;

use super::{HarmonyPanel, HarmonyPanelState, HarmonyWheel};
use crate::harmony::Harmony;
use craft_ui::theme::{Base, install};

fn builder<State>() -> egui_kittest::HarnessBuilder<State> {
    Harness::<State>::builder().with_size([480.0, 640.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default()))
}

const COLORS: [Srgba; 2] = [Srgba::rgb(0.9, 0.2, 0.2), Srgba::rgb(0.2, 0.7, 0.4)];

#[derive(Default)]
struct WheelHost {
    pressed: Vec<usize>,
    moved: Vec<(usize, [Option<f32>; 3])>,
}

/// Dragging a marker reports the move of that colour; pressing selects it.
#[test]
fn dragging_a_marker_moves_its_colour() {
    let mut harness = builder().build_ui_state(
        |ui, host: &mut WheelHost| {
            let shown = HarmonyWheel::new("wheel", "Harmony", &COLORS).selected(Some(0)).size(200.0).show(ui);
            host.pressed.extend(shown.pressed);
            host.moved.extend(shown.moved);
        },
        WheelHost::default(),
    );
    harness.run();
    let marker = harness.get_by(|n| n.role() == Role::Slider && n.label().is_some_and(|l| l.contains("#33b366"))).rect().center();
    harness.hover_at(marker);
    harness.event(Event::PointerButton { pos: marker, button: PointerButton::Primary, pressed: true, modifiers: Modifiers::NONE });
    harness.run();
    assert!(harness.state().pressed.contains(&1), "{:?}", harness.state().pressed);
    let target = marker + egui::vec2(-20.0, 0.0);
    harness.event(Event::PointerMoved(target));
    harness.run();
    harness.event(Event::PointerButton { pos: target, button: PointerButton::Primary, pressed: false, modifiers: Modifiers::NONE });
    harness.run();
    let moved = &harness.state().moved;
    assert!(!moved.is_empty() && moved.iter().all(|(i, [h, s, b])| *i == 1 && h.is_some() && s.is_some() && b.is_none()), "{moved:?}");
}

struct PanelHost {
    state: HarmonyPanelState,
    chosen: Vec<Srgba>,
}

/// The guide's centre column is outlined and a cell click chooses its colour.
#[test]
fn a_grid_click_chooses_its_colour() {
    let base = Srgba::rgb(0.9, 0.4, 0.1);
    let mut harness = builder().build_ui_state(
        move |ui, host: &mut PanelHost| {
            let shown = HarmonyPanel::new("guide", &mut host.state, base).show(ui);
            host.chosen.extend(shown.chosen);
        },
        PanelHost { state: HarmonyPanelState { rule: Harmony::Complementary, ..Default::default() }, chosen: Vec::new() },
    );
    harness.run();
    let guide = crate::harmony::Guide::new(base, Harmony::Complementary, &harness.state().state.options);
    let centre = guide.grid[1][guide.centre()];
    let hex = centre.to_hex();
    // The strip holds the harmony colours; the grid's centre column holds them too, so the
    // second row's centre cell is the second harmony colour.
    assert_eq!(centre, guide.colors[1]);
    let cells: Vec<_> = harness.query_all(egui_kittest::kittest::By::new().role(Role::Button).label(hex.as_str())).collect();
    assert!(cells.len() >= 2, "strip cell and grid centre cell");
    cells.last().unwrap().click();
    harness.run();
    assert_eq!(harness.state().chosen, [centre]);
}
