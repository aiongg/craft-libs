//! Tests of the Blend If bar (task AD-10).
#![cfg(not(target_arch = "wasm32"))]

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use craft_ui::edit::EditEvent;
use craft_ui::localized::LocaleNumbers;
use craft_ui::theme::{Base, Tokens, TokensOf as _, install};

use super::blending::{BlendingInput, blending_page};
use super::labels::{BlendIfBarLabels, LayerStyleLabels};
use super::{BlendIfBar, BlendIfBarStyle};
use crate::layer_style::{BlendIf, BlendIfRange, Blending};

/// A host that applies every value the bar reports and keeps the events.
struct Host {
    range: BlendIfRange,
    before: Option<BlendIfRange>,
    events: Vec<EditEvent<BlendIfRange>>,
    rect: Rect,
    style: Option<BlendIfBarStyle>,
}

impl Host {
    fn kinds(&self) -> Vec<&'static str> {
        self.events
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

    /// The screen position of `level` on the ramp, at mid height of the bar.
    fn at(&self, level: f32) -> Pos2 {
        let inset = self.style.as_ref().map_or(0.0, |s| s.inset);
        let ramp = self.rect.width() - 2.0 * inset;
        Pos2::new(self.rect.left() + inset + ramp * level / 255.0, self.rect.center().y)
    }
}

fn rig(range: BlendIfRange) -> Harness<'static, Host> {
    let host = Host { range, before: None, events: Vec::new(), rect: Rect::NOTHING, style: None };
    let mut h = Harness::builder().with_size([400.0, 200.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            host.style = Some(BlendIfBar::style(&Tokens::of(ui)));
            let shown = BlendIfBar::new("bar", "This Layer", host.range).show(ui);
            host.rect = shown.response.rect;
            if let Some(event) = shown.event {
                match &event {
                    EditEvent::Begin { .. } => host.before = Some(host.range),
                    EditEvent::Cancel { .. } => host.range = host.before.unwrap_or(host.range),
                    other => {
                        if let Some(value) = other.value() {
                            host.range = *value;
                        }
                    }
                }
                host.events.push(event);
            }
        },
        host,
    );
    h.run_steps(3);
    h
}

fn pointer(h: &mut Harness<'_, Host>, pos: Pos2, pressed: bool, modifiers: Modifiers) {
    h.event(Event::ModifiersChanged(modifiers));
    h.event(Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers });
    h.step();
}

/// Presses at `from`, drags in steps to `to` and releases, with `modifiers` held.
fn drag(h: &mut Harness<'_, Host>, from: Pos2, to: Pos2, modifiers: Modifiers) {
    h.hover_at(from);
    h.step();
    pointer(h, from, true, modifiers);
    for step in 1..=4 {
        h.hover_at(from + (to - from) * (step as f32 / 4.0));
        h.step();
    }
    pointer(h, to, false, modifiers);
    h.event(Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(3);
}

// A drag of the unsplit black handle moves both halves; with Alt held it moves only the half on
// the pointer's side; a gesture Begin, Update, Commit; Esc cancels.
#[test]
fn blend_if_bar_drag_moves_handles_and_alt_drag_splits() {
    let mut h = rig(BlendIfRange([10, 10, 255, 255]));
    let (from, to) = (h.state().at(11.0), h.state().at(50.0));
    drag(&mut h, from, to, Modifiers::NONE);
    let kinds = h.state().kinds();
    assert_eq!((kinds.first(), kinds.last()), (Some(&"Begin"), Some(&"Commit")), "{kinds:?}");
    assert!(kinds[1..kinds.len() - 1].iter().all(|k| *k == "Update"), "{kinds:?}");
    assert_eq!(h.state().range, BlendIfRange([50, 50, 255, 255]), "the pair moves together");

    let mut h = rig(BlendIfRange([10, 10, 255, 255]));
    let (from, to) = (h.state().at(11.0), h.state().at(50.0));
    drag(&mut h, from, to, Modifiers::ALT);
    assert_eq!(h.state().range, BlendIfRange([10, 50, 255, 255]), "Alt moves only the half on the pointer's side");

    let mut h = rig(BlendIfRange([10, 10, 255, 255]));
    let (from, to) = (h.state().at(11.0), h.state().at(50.0));
    h.hover_at(from);
    h.step();
    pointer(&mut h, from, true, Modifiers::NONE);
    h.hover_at(to);
    h.step();
    h.key_press(Key::Escape);
    h.run_steps(3);
    assert_eq!(h.state().kinds().last(), Some(&"Cancel"), "{:?}", h.state().kinds());
    assert_eq!(h.state().range, BlendIfRange([10, 10, 255, 255]));
}

// Tab visits the four handles; → moves the focused pair one level, ⇧→ ten, ⌥→ only the focused
// half; each press is one Set; AccessKit sliders are named "Black low, This Layer" and so on.
#[test]
fn blend_if_bar_arrow_keys_move_the_focused_handle_and_alt_arrow_moves_one_half() {
    let mut h = rig(BlendIfRange([10, 20, 200, 220]));
    for name in ["Black low", "Black high", "White low", "White high"] {
        let label = format!("{name}, This Layer");
        let slider = h.get_by_role_and_label(Role::Slider, &label);
        assert!(slider.accesskit_node().numeric_value().is_some(), "{name} has a level");
    }
    for _ in 0..3 {
        h.key_press(Key::Tab);
        h.run_steps(2);
    }
    h.key_press(Key::ArrowRight);
    h.run_steps(3);
    assert_eq!(h.state().range, BlendIfRange([10, 20, 201, 221]), "the third stop is White low: its pair moves");

    h.get_by_role_and_label(Role::Slider, "Black high, This Layer").focus();
    h.run_steps(2);
    h.key_press_modifiers(Modifiers::SHIFT, Key::ArrowRight);
    h.run_steps(3);
    assert_eq!(h.state().range, BlendIfRange([20, 30, 201, 221]), "⇧→ moves the pair ten levels");
    h.key_press_modifiers(Modifiers::ALT, Key::ArrowLeft);
    h.run_steps(3);
    assert_eq!(h.state().range, BlendIfRange([20, 29, 201, 221]), "⌥← moves only the focused half");
    assert!(h.state().kinds().iter().all(|k| *k == "Set"), "{:?}", h.state().kinds());
    assert_eq!(h.state().events.len(), 3);
}

struct PageHost {
    blending: Blending,
    names: Vec<String>,
}

// The Blending Options page shows the Blend If select and, under it, a bar for This Layer and one
// for Underlying Layer; a key on a bar reports the page's whole Blending.
#[test]
fn blending_page_shows_the_blend_if_select_with_both_bars() {
    let blending = Blending { blend_if: vec![BlendIf::default(); 4], ..Blending::default() };
    let host = PageHost { blending, names: ["R", "G", "B"].map(String::from).to_vec() };
    let mut h = Harness::builder().with_size([400.0, 1800.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, (host, edits): &mut (PageHost, Vec<Blending>)| {
            let (labels, bar_labels, numbers) = (LayerStyleLabels::default(), BlendIfBarLabels::default(), LocaleNumbers::from_tag("en"));
            let input = BlendingInput {
                blending: &host.blending,
                blend_if_channel: 0,
                channel_names: &host.names,
                blend_modes: &[],
                labels: &labels,
                bar_labels: &bar_labels,
                numbers: &numbers,
                enabled: true,
            };
            let out = blending_page(ui, &input);
            if let Some(value) = out.edit.as_ref().and_then(|e| e.value()) {
                host.blending = value.clone();
                edits.push(value.clone());
            }
        },
        (host, Vec::new()),
    );
    h.run_steps(3);
    let _ = h.get_by_role_and_label(Role::ComboBox, "Blend If:");
    for layer in ["This Layer", "Underlying Layer"] {
        for handle in ["Black low", "Black high", "White low", "White high"] {
            let _ = h.get_by_role_and_label(Role::Slider, &format!("{handle}, {layer}"));
        }
    }
    h.get_by_role_and_label(Role::Slider, "White high, Underlying Layer").focus();
    h.run_steps(2);
    h.key_press(Key::ArrowLeft);
    h.run_steps(3);
    let (host, edits) = h.state();
    assert_eq!(edits.len(), 1);
    assert_eq!(host.blending.blend_if[0].underlying, BlendIfRange([0, 0, 254, 254]), "the unsplit white pair moves together");
}
