//! Tests of the Brush Settings panel frame (task BR-BS6).
#![cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]

use craft_ui::direction::{self, Direction};
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Context, Ui};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};

use super::{BrushPanelAction, BrushSection, BrushSectionEvent, BrushSettingsPanel};

type Event = BrushSectionEvent<&'static str>;

struct Host {
    sections: Vec<BrushSection<&'static str>>,
    selected: Option<&'static str>,
    can_update: bool,
    events: Vec<Event>,
    actions: Vec<BrushPanelAction>,
    /// The key `body` got, whether its `Ui` was enabled, and what the panel returned for it.
    body: Option<(&'static str, bool)>,
    returned: Option<Option<&'static str>>,
}

impl Host {
    fn new() -> Self {
        Host {
            sections: vec![
                BrushSection::pane("tip", "Brush Tip Shape"),
                BrushSection::pane("dyn", "Shape Dynamics").enabled(false).locked(false),
                BrushSection::pane("scatter", "Scattering").enabled(true).locked(false),
                BrushSection::switch("noise", "Noise", false),
            ],
            selected: Some("tip"),
            can_update: false,
            events: Vec::new(),
            actions: Vec::new(),
            body: None,
            returned: None,
        }
    }
}

fn show(ui: &mut Ui, host: &mut Host) {
    let mut seen = None;
    let out =
        BrushSettingsPanel::new("panel").sections(&host.sections).selected(host.selected.as_ref()).can_update(host.can_update).show(ui, |ui, key| {
            seen = Some((*key, ui.is_enabled()));
            ui.label(format!("pane {key}"));
            *key
        });
    host.body = seen;
    host.returned = Some(out.body);
    host.actions.extend(out.action);
    for event in out.sections {
        if let BrushSectionEvent::Select(key) = &event {
            host.selected = Some(*key);
        }
        host.events.push(event);
    }
}

fn harness(host: Host) -> Harness<'static, Host> {
    harness_in(host, Direction::Ltr, [540.0, 420.0])
}

fn harness_in(host: Host, dir: Direction, size: [f32; 2]) -> Harness<'static, Host> {
    let mut harness = Harness::<Host>::builder()
        .with_size(size)
        .with_step_dt(0.05)
        .with_context_setup(move |ctx: &Context| {
            install(ctx, &Base::suite_default());
            direction::apply(ctx, dir);
        })
        .build_ui_state(show, host);
    harness.run();
    harness
}

fn named<'h>(harness: &'h Harness<'_, Host>, role: Role, label: &'static str) -> Node<'h> {
    harness.get_by(move |n| n.role() == role && n.label().as_deref() == Some(label))
}

// The body of a section whose box is clear runs in a disabled `Ui`; an on section's runs enabled.
#[test]
fn an_off_section_greys_its_pane() {
    let mut host = Host::new();
    host.selected = Some("dyn");
    let mut harness = harness(host);
    assert_eq!(harness.state().body, Some(("dyn", false)));
    harness.state_mut().selected = Some("scatter");
    harness.run();
    assert_eq!(harness.state().body, Some(("scatter", true)));
}

// `body` gets the selected pane's key, or the first pane section's when `selected` is None or a
// switch; its result comes back in `body`.
#[test]
fn the_body_draws_the_selected_pane() {
    let mut harness = harness(Host::new());
    assert_eq!(harness.state().returned, Some(Some("tip")));
    harness.state_mut().selected = Some("scatter");
    harness.run();
    assert_eq!(harness.state().body, Some(("scatter", true)));
    assert_eq!(harness.state().returned, Some(Some("scatter")));
    for selected in [None, Some("noise")] {
        harness.state_mut().selected = selected;
        harness.run();
        assert_eq!(harness.state().body, Some(("tip", true)), "{selected:?} shows the first pane");
    }
    harness.state_mut().sections.clear();
    harness.run();
    assert_eq!(harness.state().returned, Some(None), "no pane section, no body");
}

// New and Reset report their actions; Update is disabled without `can_update` and reports
// `UpdatePreset` with it.
#[test]
fn the_strip_buttons_report_their_actions() {
    let mut harness = harness(Host::new());
    let new = "Create new brush from these settings";
    let reset = "Reset the brush to the defaults";
    let update = "Update the current brush with these settings";
    assert!(named(&harness, Role::Button, update).accesskit_node().is_disabled());
    named(&harness, Role::Button, new).click();
    harness.run();
    named(&harness, Role::Button, reset).click();
    harness.run();
    named(&harness, Role::Button, update).click();
    harness.run();
    assert_eq!(harness.state().actions, [BrushPanelAction::NewPreset, BrushPanelAction::Reset]);
    harness.state_mut().can_update = true;
    harness.run();
    assert!(!named(&harness, Role::Button, update).accesskit_node().is_disabled());
    named(&harness, Role::Button, update).click();
    harness.run();
    assert_eq!(harness.state().actions, [BrushPanelAction::NewPreset, BrushPanelAction::Reset, BrushPanelAction::UpdatePreset]);
}

// The list's events come through the panel's output in order.
#[test]
fn section_events_pass_through_in_order() {
    let mut harness = harness(Host::new());
    named(&harness, Role::Button, "Shape Dynamics").click();
    harness.run();
    assert_eq!(harness.state().events, [Event::Select("dyn"), Event::Enable { key: "dyn", on: true }]);
    harness.run();
    assert_eq!(harness.state().body.map(|(key, _)| key), Some("dyn"), "the pane follows the selection");
}

// Right to left mirrors the panel: the list sits at the right, and the buttons are left of the
// strip, as the mirror image of left to right.
#[test]
fn the_panel_mirrors_in_right_to_left() {
    let ltr = harness_in(Host::new(), Direction::Ltr, [540.0, 420.0]);
    let rtl = harness_in(Host::new(), Direction::Rtl, [540.0, 420.0]);
    let (list_ltr, list_rtl) = (ltr.get_by_role(Role::ListBox).rect(), rtl.get_by_role(Role::ListBox).rect());
    assert!(list_ltr.center().x < 270.0 && list_rtl.center().x > 270.0, "{list_ltr:?} {list_rtl:?}");
    let new = "Create new brush from these settings";
    let (strip_ltr, strip_rtl) = (named(&ltr, Role::Image, "Stroke preview").rect(), named(&rtl, Role::Image, "Stroke preview").rect());
    let (button_ltr, button_rtl) = (named(&ltr, Role::Button, new).rect(), named(&rtl, Role::Button, new).rect());
    assert!(button_ltr.center().x > strip_ltr.center().x, "left to right: buttons after the strip");
    assert!(button_rtl.center().x < strip_rtl.center().x, "right to left: buttons before the strip");
}

// In a host of bounded height the whole panel fits: the buttons, taller than the strip at the
// default size, are not cut off.
#[test]
fn the_panel_fits_a_bounded_height() {
    for height in [200.0, 300.0, 420.0] {
        let harness = harness_in(Host::new(), Direction::Ltr, [540.0, height]);
        let update = named(&harness, Role::Button, "Update the current brush with these settings").rect();
        let strip = named(&harness, Role::Image, "Stroke preview").rect();
        assert!(update.bottom() <= height + 0.5, "the Update button is inside the host: {update:?} in {height}");
        assert!(strip.bottom() <= height + 0.5, "the strip is inside the host: {strip:?} in {height}");
    }
}
