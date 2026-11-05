//! Tests of Hue/Saturation (task AD-4).

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use super::hue_saturation::{RANGE, lightness_track, saturation_track};
use super::{HueRangeValue, HueSaturation, HueSaturationEditor};
use craft_theme::Srgba;
use craft_ui::edit::EditEvent;
use craft_ui::localized::LocaleNumbers;
use craft_ui::theme::{Base, install};

struct Host {
    value: HueSaturation,
    /// The range to show from the first pass: 0 Master, 1 Reds …
    range: Option<usize>,
    events: Vec<EditEvent<HueSaturation>>,
    shown: Option<Option<usize>>,
}

fn rig(value: HueSaturation, range: Option<usize>) -> Harness<'static, Host> {
    let state = Host { value, range, events: Vec::new(), shown: None };
    let mut h = Harness::builder().with_size([420.0, 720.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            let numbers = LocaleNumbers::from_tag("en");
            let id = HueSaturationEditor::id_in(ui, "hs");
            if let Some(range) = host.range.take() {
                ui.data_mut(|d| d.insert_temp(id.with(RANGE), range));
            }
            let out = HueSaturationEditor::new("hs", &host.value, &numbers).show(ui);
            host.shown = Some(HueSaturationEditor::shown_range(ui.ctx(), id));
            host.events.extend(out.event);
        },
        state,
    );
    h.run_steps(3);
    h
}

// Colorize on: the range select is disabled and shows Master, Hue −90 becomes 270, Saturation 10 becomes 25; off: Hue 270 becomes −90.
#[test]
fn hue_saturation_colorize_forces_master_and_wraps_hue() {
    let mut h = rig(HueSaturation { hue: -90.0, saturation: 10.0, ..HueSaturation::default() }, None);
    h.get_by_role_and_label(Role::CheckBox, "Colorize").click();
    h.run_steps(3);
    let events = &h.state().events;
    let [EditEvent::Set { value, .. }] = events.as_slice() else { panic!("one Set: {events:?}") };
    assert!(value.colorize);
    assert_eq!((value.hue, value.saturation), (270.0, 25.0));

    let on = HueSaturation { hue: 270.0, saturation: 25.0, colorize: true, ..HueSaturation::default() };
    let mut h = rig(on, Some(3));
    assert_eq!(h.state().shown, Some(None), "Colorize shows Master although Yellows were shown");
    assert!(h.get_by_role_and_label(Role::ComboBox, "Range").accesskit_node().is_disabled());
    h.get_by_role_and_label(Role::CheckBox, "Colorize").click();
    h.run_steps(3);
    let events = &h.state().events;
    let [EditEvent::Set { value, .. }] = events.as_slice() else { panic!("one Set: {events:?}") };
    assert!(!value.colorize);
    assert_eq!(value.hue, -90.0);
}

// Dragging the Reds' range-start thumb across 0° keeps the four bounds increasing.
#[test]
fn hue_saturation_range_thumbs_wrap_through_zero() {
    // The Magentas' end thumb dragged from 345° to 20°.
    let moved = HueRangeValue::move_bound(HueRangeValue::neutral(5).bounds, 3, 20.0);
    assert_eq!(moved, [255.0, 285.0, 315.0, 380.0]);
    // The Reds' start thumb dragged from 315° (−45°) to 340°, past the second thumb's room.
    let moved = HueRangeValue::move_bound(HueRangeValue::neutral(0).bounds, 0, 340.0);
    assert_eq!(moved, [-20.0, -15.0, 15.0, 45.0]);
    assert!(moved.windows(2).all(|pair| pair[0] <= pair[1]));
}

// `HueRangeValue::move_bound` canonicalises before clamping (PhotoCraft's `hue_range_drag_canonicalizes_bounds_before_clamping`).
#[test]
fn hue_saturation_range_thumb_move_canonicalises_bounds_first() {
    let moved = HueRangeValue::move_bound([0.0, 350.0, 100.0, 200.0], 2, 354.0);
    assert_eq!(moved, [-360.0, -10.0, -6.0, -1.0]);
    let canonical = HueRangeValue::move_bound([0.0, 30.0, 60.0, 90.0], 2, 65.0);
    assert_eq!(canonical, [0.0, 30.0, 65.0, 90.0]);
    assert_eq!(HueRangeValue::canonical_bounds([f64::NAN, 0.0, 0.0, 0.0]), [-45.0, -15.0, 15.0, 45.0]);
    assert_eq!(HueRangeValue::move_bound([0.0, 30.0, 60.0, 90.0], 4, 10.0), [0.0, 30.0, 60.0, 90.0], "an index past 3 only canonicalises");
}

// The Saturation track runs grey to the current hue; the Lightness track is black, grey, white at any hue (PhotoCraft's two track tests).
#[test]
fn hue_saturation_saturation_track_follows_hue_and_lightness_track_is_fixed() {
    let grey = Srgba::rgb(128.0 / 255.0, 128.0 / 255.0, 128.0 / 255.0);
    assert_eq!(saturation_track(0.0), [grey, Srgba::rgb(1.0, 0.0, 0.0)]);
    assert_eq!(saturation_track(120.0)[1], Srgba::rgb(0.0, 1.0, 0.0));
    assert_eq!(saturation_track(720.0)[1], saturation_track(0.0)[1], "hue wraps");
    assert_eq!(lightness_track(), [Srgba::BLACK, grey, Srgba::WHITE]);
}

// Each range thumb is a Tab stop and a `Slider` with its name and degrees; → moves it 1°, Shift 10°.
#[test]
fn hue_saturation_range_thumbs_take_the_keyboard() {
    let mut h = rig(HueSaturation::default(), Some(1));
    assert_eq!(h.state().shown, Some(Some(0)));
    for (name, degrees) in [("Range start", 315.0), ("Fully affected start", 345.0), ("Fully affected end", 15.0), ("Range end", 45.0)] {
        let node = h.get_by_role_and_label(Role::Slider, name);
        assert_eq!(node.accesskit_node().numeric_value(), Some(degrees), "{name}");
    }
    assert!(h.query_by_role_and_label(Role::Image, "Hue spectrum, before").is_some());
    assert!(h.query_by_role_and_label(Role::Image, "Hue spectrum, after").is_some());

    h.get_by_role_and_label(Role::Slider, "Range start").focus();
    h.run_steps(2);
    h.key_press(Key::ArrowRight);
    h.run_steps(3);
    h.key_press_modifiers(Modifiers::SHIFT, Key::ArrowRight);
    h.run_steps(3);
    let bounds: Vec<[f64; 4]> = h.state().events.iter().filter_map(|e| e.value().map(|v| v.ranges[0].bounds)).collect();
    assert_eq!(bounds[0], [-44.0, -15.0, 15.0, 45.0], "→ moves 1°");
    // The host does not echo the first step, so the second starts from the same value: 10° more, up to the second thumb.
    assert_eq!(bounds[1], [-35.0, -15.0, 15.0, 45.0], "Shift+→ moves 10°");
}
