//! Tests of the Gradient Map editor (task AD-6).

use craft_theme::Srgba;
use egui::accesskit::Role;
use egui::{Context, Pos2, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::map::{GradientMap, GradientMapEditor};
use super::{ColorStop, GradientGroup, GradientItem, GradientStops, OpacityStop};
use craft_ui::edit::EditEvent;
use craft_ui::localized::LocaleNumbers;
use craft_ui::theme::{Base, install};

struct Host {
    value: GradientMap,
    groups: Vec<GradientGroup>,
    events: Vec<EditEvent<GradientMap>>,
    imports: usize,
    requests: Vec<craft_ui::dialog::ColorRequest>,
}

impl Host {
    fn show(&mut self, ui: &mut egui::Ui) {
        let numbers = LocaleNumbers::from_tag("en");
        let value = self.value.clone();
        let out = GradientMapEditor::new("map", &value, &numbers).presets(&self.groups).show(ui);
        self.imports += usize::from(out.import_presets);
        self.requests.extend(out.pick_color);
        if let Some(event) = out.edit.event {
            if let Some(v) = event.value() {
                self.value = v.clone();
            }
            self.events.push(event);
        }
    }
}

fn stops(n: usize) -> GradientStops {
    GradientStops::new((0..n).map(|i| ColorStop::new(i as f32 / (n - 1) as f32, Srgba::rgb(i as f32 / n as f32, 0.2, 0.5))).collect())
}

fn host(n: usize) -> Harness<'static, Host> {
    let preset = GradientStops {
        color: vec![ColorStop::new(0.0, Srgba::new(1.0, 0.0, 0.0, 0.5)), ColorStop::new(1.0, Srgba::new(0.0, 0.0, 1.0, 0.5))],
        opacity: vec![OpacityStop::new(0.0, 0.5), OpacityStop::new(1.0, 0.5)],
    };
    let host = Host {
        value: GradientMap { stops: stops(n), reverse: false, dither: false },
        groups: vec![GradientGroup::new("Basics", vec![GradientItem::new("Warm", preset)])],
        events: Vec::new(),
        imports: 0,
        requests: Vec::new(),
    };
    let mut harness = Harness::builder()
        .with_size([400.0, 400.0])
        .with_step_dt(1.0 / 60.0)
        .with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default()))
        .build_ui_state(|ui, host: &mut Host| host.show(ui), host);
    harness.run();
    harness
}

fn kinds(events: &[EditEvent<GradientMap>]) -> Vec<&'static str> {
    events
        .iter()
        .map(|e| match e {
            EditEvent::Begin { .. } => "Begin",
            EditEvent::Update { .. } => "Update",
            EditEvent::Commit { .. } => "Commit",
            EditEvent::Cancel { .. } => "Cancel",
            EditEvent::Set { .. } => "Set",
            _ => "other",
        })
        .collect()
}

// Picking a preset with a 50 % opacity stop reports one `Set` whose stops are the preset's colour
// stops with alpha 1 and no opacity stops.
#[test]
fn gradient_map_preset_pick_sets_opaque_colour_stops() {
    let mut harness = host(3);
    harness.get_by_label("Custom").click();
    harness.run();
    harness.get_by_label("Warm").click();
    harness.run();
    let state = harness.state();
    assert_eq!(kinds(&state.events), ["Set"]);
    let stops = &state.value.stops;
    assert!(stops.opacity.is_empty());
    assert_eq!(stops.color.len(), 2);
    assert!(stops.color.iter().all(|s| s.color.a == 1.0));
    assert_eq!(stops.color[0].color.r, 1.0);
}

// A stop drag on the bar reports `Begin`, `Update` and `Commit` of the map with the new stops and
// Reverse and Dither unchanged.
#[test]
fn gradient_map_stop_edits_come_out_as_map_edits() {
    let mut harness = host(2);
    harness.state_mut().value.dither = true;
    harness.run();
    let markers: Vec<_> = harness.get_all_by_role(Role::GraphicsObject).collect();
    let from: Pos2 = markers[0].rect().center();
    let to = from + vec2(60.0, 0.0);
    harness.hover_at(from);
    harness.step();
    harness.event(egui::Event::PointerButton { pos: from, button: egui::PointerButton::Primary, pressed: true, modifiers: egui::Modifiers::NONE });
    harness.step();
    harness.hover_at(to);
    harness.step();
    harness.event(egui::Event::PointerButton { pos: to, button: egui::PointerButton::Primary, pressed: false, modifiers: egui::Modifiers::NONE });
    harness.run();
    let state = harness.state();
    let kinds = kinds(&state.events);
    assert_eq!(kinds.first(), Some(&"Begin"), "{kinds:?}");
    assert_eq!(kinds.last(), Some(&"Commit"), "{kinds:?}");
    assert!(state.value.stops.color[0].location > 0.0 || state.value.stops.color[1].location < 1.0);
    assert!(state.value.dither && !state.value.reverse);
}

// The bar keeps two to 64 stops: Delete is disabled at two, a click on the bar adds none at 64.
#[test]
fn gradient_map_keeps_two_to_sixty_four_stops() {
    let mut harness = host(2);
    if let Some(marker) = harness.get_all_by_role(Role::GraphicsObject).next() {
        marker.click();
    }
    harness.run();
    assert!(harness.get_by_label("Delete stop").accesskit_node().is_disabled());

    // The same click on an empty bar position adds a stop at 63 and none at 64.
    let click_bar = |n: usize| {
        let mut harness = host(n);
        let ramp = harness.get_by_role_and_label(Role::Group, "Gradient stops").rect();
        let at = Pos2::new(ramp.center().x + 0.5, ramp.top() + 20.0);
        harness.hover_at(at);
        harness.step();
        for pressed in [true, false] {
            harness.event(egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE });
            harness.step();
        }
        harness.run();
        harness.state().value.stops.color.len()
    };
    assert_eq!(click_bar(63), 64);
    assert_eq!(click_bar(64), 64);
}

// Reverse and Dither each report one `Set` with the flag toggled; Import .GRD sets `import_presets`
// and no edit.
#[test]
fn gradient_map_reverse_dither_and_import() {
    let mut harness = host(3);
    harness.get_by_label("Reverse").click();
    harness.run();
    harness.get_by_label("Dither").click();
    harness.run();
    let state = harness.state();
    assert_eq!(kinds(&state.events), ["Set", "Set"]);
    assert!(state.value.reverse && state.value.dither);
    assert_eq!(state.value.stops, stops(3));

    harness.get_by_label("Custom").click();
    harness.run();
    harness.get_by_label("Gradient Preset Options").click();
    harness.run();
    harness.get_by_label("Import .GRD").click();
    harness.run();
    assert_eq!(harness.state().imports, 1);
    assert_eq!(harness.state().events.len(), 2);
}

// A preset pick selects stop 0, also when another stop was selected.
#[test]
fn gradient_map_preset_pick_selects_stop_zero() {
    let mut harness = host(3);
    harness.get_all_by_role(Role::GraphicsObject).nth(2).expect("a marker").click();
    harness.run();
    let selected = |harness: &Harness<'static, Host>| {
        harness.get_all_by_role(Role::GraphicsObject).map(|m| m.accesskit_node().is_selected() == Some(true)).collect::<Vec<_>>()
    };
    assert_eq!(selected(&harness), [false, false, true]);
    harness.get_by_label("Custom").click();
    harness.run();
    harness.get_by_label("Warm").click();
    harness.run();
    assert_eq!(selected(&harness), [true, false]);
}

// Enter on the focused bar with a stop selected asks the host for that stop's colour once, keyed
// by the stop's index.
#[test]
fn gradient_map_enter_on_a_stop_requests_its_colour() {
    let mut harness = host(3);
    harness.get_all_by_role(Role::GraphicsObject).nth(1).expect("a marker").click();
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    let requests = &harness.state().requests;
    assert_eq!(requests.len(), 1, "{requests:?}");
    assert_eq!(GradientMapEditor::stop_of_key(&requests[0].key), Some(1));
    assert_eq!(requests[0].color, stops(3).color[1].color);
}
