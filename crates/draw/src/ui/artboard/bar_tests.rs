//! Tests of the Artboard bar and Properties section (task SH-T8).

use egui::accesskit::Role;
use egui::{Key, Modifiers, vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use kurbo::{Rect, Size};

use super::{ArtboardBackground, ArtboardBar, ArtboardBarOutput, ArtboardItem, ArtboardPreset, ArtboardProperties, ArtboardPropertiesOutput};
use craft_ui::edit::EditEvent;
use craft_ui::patterns::OptionsBar;
use craft_ui::theme::{Base, install};

fn board(width: f64, height: f64) -> ArtboardItem {
    ArtboardItem { name: "a".into(), rect: Rect::new(10.0, 20.0, 10.0 + width, 20.0 + height), locked: false, background: ArtboardBackground::White }
}

fn presets() -> Vec<ArtboardPreset> {
    vec![ArtboardPreset { name: "Small".into(), size: Size::new(50.0, 60.0) }]
}

struct BarHost {
    board: ArtboardItem,
    out: ArtboardBarOutput,
}

fn bar_harness(board: ArtboardItem) -> Harness<'static, BarHost> {
    let host = BarHost { board, out: ArtboardBarOutput::default() };
    let mut h = Harness::builder().with_size(vec2(1200.0, 200.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut BarHost| {
            let presets = presets();
            let mut out = ArtboardBarOutput::default();
            OptionsBar::new("bar").show(ui, |slots| ArtboardBar::new("bar", Some(&host.board), &presets).output(&mut out).add_to(slots));
            if out.size.is_some() || out.add_new {
                host.out = out;
            }
        },
        host,
    );
    h.run_steps(3);
    h
}

fn set_size(out: &ArtboardBarOutput) -> Option<Size> {
    match out.size {
        Some(EditEvent::Set { value, .. } | EditEvent::Commit { value, .. }) => Some(value),
        _ => None,
    }
}

// Make Portrait shows only for a board wider than tall and reports the swapped size; Make
// Landscape only for one taller than wide; a preset pick reports its size.
#[test]
fn artboard_bar_make_portrait_only_when_wider() {
    let mut h = bar_harness(board(200.0, 100.0));
    assert!(h.query_by_label("Make Landscape").is_none());
    h.get_by_label("Make Portrait").click();
    h.run_steps(3);
    assert_eq!(set_size(&h.state().out), Some(Size::new(100.0, 200.0)));

    let mut h = bar_harness(board(100.0, 200.0));
    assert!(h.query_by_label("Make Portrait").is_none());
    h.get_by_label("Make Landscape").click();
    h.run_steps(3);
    assert_eq!(set_size(&h.state().out), Some(Size::new(200.0, 100.0)));

    h.state_mut().out = ArtboardBarOutput::default();
    h.get_by_role_and_label(Role::ComboBox, "Preset").click();
    h.run_steps(3);
    h.get_by_role_and_label(Role::ListBoxOption, "Small").click();
    h.run_steps(3);
    assert_eq!(set_size(&h.state().out), Some(Size::new(50.0, 60.0)));

    h.state_mut().out = ArtboardBarOutput::default();
    h.get_by_label("Add New Artboard").click();
    h.run_steps(3);
    assert!(h.state().out.add_new);
}

struct PropertiesHost {
    board: ArtboardItem,
    out: ArtboardPropertiesOutput,
}

// W and H in the bar and W, H, X and Y in the Properties section show their pixel unit.
#[test]
fn artboard_fields_show_the_pixel_unit() {
    let h = bar_harness(board(200.0, 100.0));
    for name in ["Width", "Height"] {
        let value = h.get_by_role_and_label(Role::SpinButton, name).value();
        assert!(value.is_some_and(|v| v.contains("px")), "bar {name}");
    }
    let host = PropertiesHost { board: board(100.0, 60.0), out: ArtboardPropertiesOutput::default() };
    let mut h = Harness::builder().with_size(vec2(400.0, 500.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut PropertiesHost| {
            let presets = presets();
            host.out = ArtboardProperties::new("props", &host.board, &presets).show(ui);
        },
        host,
    );
    h.run_steps(3);
    for name in ["Width", "Height", "X", "Y"] {
        let value = h.get_by_role_and_label(Role::SpinButton, name).value();
        assert!(value.is_some_and(|v| v.contains("px")), "properties {name}");
    }
}

// Other… in the Properties background reports a `ColorRequest`; X and Y fields report the moved
// rect.
#[test]
fn artboard_properties_background_other_asks_for_a_color() {
    let host = PropertiesHost { board: board(100.0, 60.0), out: ArtboardPropertiesOutput::default() };
    let mut h = Harness::builder().with_size(vec2(400.0, 500.0)).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut PropertiesHost| {
            let presets = presets();
            let out = ArtboardProperties::new("props", &host.board, &presets).show(ui);
            if out.rect.is_some() || out.pick_color.is_some() {
                host.out = out;
            }
        },
        host,
    );
    h.run_steps(3);
    h.get_by_role_and_label(Role::ComboBox, "Background").click();
    h.run_steps(3);
    h.get_by_role_and_label(Role::ListBoxOption, "Other…").click();
    h.run_steps(3);
    let Some(request) = h.state().out.pick_color.clone() else { panic!("Other… asks for a color") };
    assert_eq!(request.title.as_deref(), Some("Artboard Background Color"));
    assert!(h.state().out.background.is_none(), "the background changes once the color comes back");

    h.state_mut().out = ArtboardPropertiesOutput::default();
    h.get_by_role_and_label(Role::SpinButton, "X").focus();
    h.run_steps(3);
    h.key_press_modifiers(Modifiers::NONE, Key::ArrowUp);
    h.run_steps(3);
    let Some(EditEvent::Set { value, .. } | EditEvent::Commit { value, .. }) = h.state().out.rect.clone() else { panic!("X reports the moved rect") };
    assert_eq!(value, Rect::new(11.0, 20.0, 111.0, 80.0));
}
