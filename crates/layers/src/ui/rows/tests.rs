//! Screen tests of `BlendRows` (L3).

#![allow(clippy::expect_used, reason = "tests")]

use craft_ui::edit::EditEvent;
use craft_ui::theme::{Base, install};
use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::{BlendRows, LayerPropertyEdit, opacity_row_layout};
use crate::ui::tree::{BlendChoice, LayerLocks, LayerProperties, LockKind};

struct App {
    props: Option<LayerProperties>,
    modes: Vec<BlendChoice>,
    width: f32,
    events: Vec<LayerPropertyEdit>,
}

fn props() -> LayerProperties {
    LayerProperties { blend_key: String::from("normal"), group: false, opacity: 50.0, fill: 100.0, locks: LayerLocks::default(), editable: true }
}

fn modes() -> Vec<BlendChoice> {
    [("normal", "Normal", false), ("dissolve", "Dissolve", false), ("darken", "Darken", true), ("multiply", "Multiply", false)]
        .into_iter()
        .map(|(key, label, separator_before)| BlendChoice { key: key.to_owned(), label: label.to_owned(), separator_before })
        .collect()
}

/// The rows over `props`, `width` wide; the app applies each edit as PhotoCraft does.
fn rows(props: Option<LayerProperties>, width: f32) -> Harness<'static, App> {
    let app = App { props, modes: modes(), width, events: Vec::new() };
    let mut harness =
        HarnessBuilder::default().with_size(vec2(400.0, 200.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, app: &mut App| {
                ui.set_max_width(app.width);
                let out = BlendRows::new("rows", app.props.as_ref(), &app.modes).show(ui);
                for edit in out.events {
                    if let Some(props) = app.props.as_mut() {
                        match &edit {
                            LayerPropertyEdit::Blend(EditEvent::Set { value, .. } | EditEvent::Commit { value, .. }) => {
                                props.blend_key.clone_from(value)
                            }
                            LayerPropertyEdit::Opacity(
                                EditEvent::Set { value, .. } | EditEvent::Update { value, .. } | EditEvent::Commit { value, .. },
                            ) => {
                                props.opacity = *value;
                            }
                            _ => {}
                        }
                    }
                    app.events.push(edit);
                }
            },
            app,
        );
    harness.run();
    harness
}

fn wheel(lines: f32) -> Event {
    Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: vec2(0.0, lines),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Wheel,
        modifiers: Modifiers::NONE,
    }
}

fn press(pos: Pos2, pressed: bool) -> Event {
    Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
}

// A wheel notch over the closed blend select reports one `Blend(Set)` per notch; hovering a mode
// in the open list reports `BlendHover(Some(key))`, leaving it `BlendHover(None)`.
#[test]
fn blend_select_steps_with_the_wheel_and_reports_hover() {
    let mut harness = rows(Some(props()), 300.0);
    let select = harness.get_by_role(Role::ComboBox).rect();
    harness.hover_at(select.center());
    harness.run();
    harness.event(wheel(-1.0));
    harness.run();
    harness.event(wheel(-1.0));
    harness.run();
    let sets: Vec<_> = harness
        .state()
        .events
        .iter()
        .filter_map(|e| match e {
            LayerPropertyEdit::Blend(EditEvent::Set { value, .. }) => Some(value.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(sets, ["dissolve", "darken"], "a Set per notch: {:?}", harness.state().events);

    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    let row = harness.get_by(|n| n.role() == Role::ListBoxOption && n.label().as_deref() == Some("Multiply")).rect();
    harness.hover_at(row.center());
    harness.run();
    assert_eq!(harness.state().events.last(), Some(&LayerPropertyEdit::BlendHover(Some(String::from("multiply")))));
    harness.hover_at(Pos2::new(380.0, 190.0));
    harness.run();
    assert_eq!(harness.state().events.last(), Some(&LayerPropertyEdit::BlendHover(None)));
}

// A scrub of "Opacity:" reports `Begin`, `Update`s and one `Commit`; Esc during it reports `Cancel`.
#[test]
fn opacity_drag_is_one_gesture_and_cancel_restores() {
    let mut harness = rows(Some(props()), 300.0);
    let label = harness.get_by_label("Opacity:").rect();
    let from = label.center();
    harness.event(Event::PointerMoved(from));
    harness.event(press(from, true));
    harness.step();
    for dx in [6.0, 12.0, 20.0] {
        harness.event(Event::PointerMoved(from + vec2(dx, 0.0)));
        harness.step();
    }
    harness.event(press(from + vec2(20.0, 0.0), false));
    harness.run();
    let opacity: Vec<_> =
        harness.state().events.iter().filter_map(|e| if let LayerPropertyEdit::Opacity(e) = e { Some(e.clone()) } else { None }).collect();
    assert!(matches!(opacity.first(), Some(EditEvent::Begin { .. })), "{opacity:?}");
    assert_eq!(opacity.iter().filter(|e| matches!(e, EditEvent::Commit { .. })).count(), 1, "{opacity:?}");
    assert!(matches!(opacity.last(), Some(EditEvent::Commit { value, .. }) if *value > 50.0), "{opacity:?}");

    let before = harness.state().events.len();
    harness.event(Event::PointerMoved(from));
    harness.event(press(from, true));
    harness.step();
    for dx in [6.0, 12.0] {
        harness.event(Event::PointerMoved(from + vec2(dx, 0.0)));
        harness.step();
    }
    harness.key_press(Key::Escape);
    harness.run();
    harness.event(press(from + vec2(12.0, 0.0), false));
    harness.run();
    let later = &harness.state().events[before..];
    assert!(later.iter().any(|e| matches!(e, LayerPropertyEdit::Opacity(EditEvent::Cancel { .. }))), "{later:?}");
    assert!(!later.iter().any(|e| matches!(e, LayerPropertyEdit::Opacity(EditEvent::Commit { .. }))), "{later:?}");
}

// The "Opacity:" label hides below `label + field + 2 gaps + 16 + layers.blend.min_width`; the
// field stays (`opacity_row_layout` and the drawn rows).
#[test]
fn opacity_label_hides_on_a_narrow_panel() {
    let (wide_blend, shown) = opacity_row_layout(300.0, 50.0, 66.0, 8.0, 80.0);
    assert!(shown && (wide_blend - (300.0 - 66.0 - 8.0 - 50.0 - 8.0 - 16.0)).abs() < 1e-3);
    let (edge, shown) = opacity_row_layout(50.0 + 66.0 + 16.0 + 16.0 + 80.0, 50.0, 66.0, 8.0, 80.0);
    assert!(shown && (edge - 80.0).abs() < 1e-3, "the label shows exactly at the threshold");
    let (narrow_blend, shown) = opacity_row_layout(50.0 + 66.0 + 16.0 + 16.0 + 80.0 - 0.5, 50.0, 66.0, 8.0, 80.0);
    assert!(!shown && (narrow_blend - (50.0 + 66.0 + 16.0 + 16.0 + 80.0 - 0.5 - 66.0 - 8.0)).abs() < 1e-3);
    assert!((opacity_row_layout(100.0, 50.0, 66.0, 8.0, 80.0).0 - 40.0).abs() < 1e-3, "the select keeps 40");

    let wide = rows(Some(props()), 300.0);
    assert!(wide.query_by_label("Opacity:").is_some());
    let narrow = rows(Some(props()), 150.0);
    assert!(narrow.query_by_label("Opacity:").is_none(), "the label hides");
    let field = narrow.get_by_role_and_label(Role::SpinButton, "Opacity").rect();
    let select = narrow.get_by_role(Role::ComboBox).rect();
    assert!((field.width() - 66.0).abs() < 1.0 && field.left() >= select.right(), "the field stays, after the select: {field:?} {select:?}");
}

// With Lock image pixels on, Lock transparent pixels is disabled and described as included.
#[test]
fn lock_row_image_pixels_disables_transparent_pixels() {
    let mut on = props();
    on.locks.pixels = true;
    let mut harness = rows(Some(on), 300.0);
    let transparent = harness.get_by_label("Lock transparent pixels");
    assert!(transparent.accesskit_node().is_disabled());
    assert_eq!(transparent.accesskit_node().data().description(), Some("Included in Lock image pixels"));
    assert!(!harness.get_by_label("Lock image pixels").accesskit_node().is_disabled());

    harness.get_by_label("Lock position").click();
    harness.run();
    assert_eq!(harness.state().events, [LayerPropertyEdit::Lock { kind: LockKind::Position, on: true }]);
    harness.get_by_label("Lock image pixels").click();
    harness.run();
    assert_eq!(harness.state().events.last(), Some(&LayerPropertyEdit::Lock { kind: LockKind::Pixels, on: false }));
}

// `editable` false greys the select, Opacity and Fill but not the locks; no properties greys every
// control at Normal, 100 % and unlocked.
#[test]
fn background_and_no_layer_grey_the_property_rows() {
    let mut background = props();
    background.editable = false;
    let harness = rows(Some(background), 300.0);
    assert!(harness.get_by_role(Role::ComboBox).accesskit_node().is_disabled());
    for name in ["Opacity", "Fill"] {
        assert!(harness.get_by_role_and_label(Role::SpinButton, name).accesskit_node().is_disabled(), "{name}");
    }
    assert!(LockKind::ALL.len() == 5);
    assert!(!harness.get_by_label("Lock all").accesskit_node().is_disabled(), "the locks stay");

    let harness = rows(None, 300.0);
    assert!(harness.get_by_role(Role::ComboBox).accesskit_node().is_disabled());
    for name in ["Opacity", "Fill"] {
        let field = harness.get_by_role_and_label(Role::SpinButton, name);
        assert!(field.accesskit_node().is_disabled(), "{name}");
        assert_eq!(field.value().as_deref(), Some("100%"), "{name}");
    }
    for label in ["Lock transparent pixels", "Lock image pixels", "Lock position", "Lock all"] {
        assert!(harness.get_by_label(label).accesskit_node().is_disabled(), "{label}");
    }
}
