//! Tests of the Layer Style dialog body: the effect list, the Styles page, colour answers (task
//! AD-8).
//!
//! The specs here list no fields, so the effect pages draw only their title and Preview check box:
//! the field pages are AD-9's.
#![cfg(not(target_arch = "wasm32"))]

use std::borrow::Cow;

use egui::Key;
use egui::accesskit::{Role, Toggled};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, Node};

use super::{LayerStyleDialog, StylePresets};
use crate::layer_style::{EffectId, EffectInstance, EffectKind, EffectSpec, LayerStyle, LayerStyleDraft, LayerStylePage, StyleParams, StyleValue};
use craft_theme::Srgba;
use craft_ui::edit::EditEvent;
use craft_ui::localized::LocaleNumbers;
use craft_ui::presets::{PresetRow, PresetState};
use craft_ui::theme::{Base, install};

/// A host that applies every change and records what the dialog reported.
struct Host {
    draft: LayerStyleDraft,
    specs: Vec<EffectSpec>,
    events: Vec<EditEvent<LayerStyleDraft>>,
    applied: Vec<String>,
    state: PresetState<String>,
    rows: Vec<PresetRow<String>>,
}

fn spec(kind: EffectKind) -> EffectSpec {
    EffectSpec { kind, label: Cow::Borrowed(""), fields: Vec::new(), defaults: StyleParams::default() }
}

fn shadow(id: u32, on: bool) -> EffectInstance {
    EffectInstance { id: EffectId(id), kind: EffectKind::DropShadow, on, params: StyleParams::default() }
}

/// A Drop Shadow (`fx1`, on) and an absent Satin, the Styles page shown.
fn rig() -> Harness<'static, Host> {
    let mut style = LayerStyle::default();
    style.effects.push(shadow(1, true));
    let mut draft = LayerStyleDraft::new(style);
    draft.page = LayerStylePage::Styles;
    let rows = vec![
        PresetRow::group("g".to_owned(), "Basics", 0),
        PresetRow::item("Soft".to_owned(), "Soft", 1),
        PresetRow::item("Hard".to_owned(), "Hard", 1),
    ];
    let host = Host {
        draft,
        specs: vec![spec(EffectKind::DropShadow), spec(EffectKind::Satin)],
        events: Vec::new(),
        applied: Vec::new(),
        state: PresetState::new(),
        rows,
    };
    let mut h = Harness::builder().with_size([640.0, 480.0]).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, host: &mut Host| {
            let numbers = LocaleNumbers::from_tag("en");
            let channels = ["R".to_owned(), "G".to_owned(), "B".to_owned()];
            let Host { draft, specs, events, applied, state, rows } = host;
            let presets = StylePresets::new(state, rows, |painter, rect, _| {
                painter.rect_filled(rect, 0.0, egui::Color32::GRAY);
            });
            let out = LayerStyleDialog::new("layer-style", draft, &channels, &numbers).specs(specs).style_presets(presets).show(ui);
            if let Some(event) = out.edit.event {
                if let Some(value) = event.value() {
                    *draft = value.clone();
                }
                events.push(event);
            }
            applied.extend(out.apply_style);
        },
        host,
    );
    h.run_steps(3);
    h
}

fn row<'a>(h: &'a Harness<'_, Host>, name: &'a str) -> Node<'a> {
    h.get_by_role_and_label(Role::ListBoxOption, name)
}

fn effects(h: &Harness<'_, Host>) -> Vec<(u32, EffectKind, bool)> {
    h.state().draft.style.effects.iter().map(|e| (e.id.0, e.kind, e.on)).collect()
}

// A click on an instance's check box turns it off and selects its page; a click on the row only
// selects it.
#[test]
fn effect_list_check_box_toggles_and_selects_the_page() {
    let mut h = rig();
    h.get_by_role_and_label(Role::CheckBox, "Drop Shadow").click();
    h.run_steps(3);
    assert_eq!(effects(&h), [(1, EffectKind::DropShadow, false)]);
    assert_eq!(h.state().draft.page, LayerStylePage::Effect(EffectId(1)));
    assert_eq!(h.state().events.len(), 1, "one Set for the click");

    h.state_mut().draft.style.effects[0].on = true;
    h.state_mut().draft.page = LayerStylePage::Styles;
    h.run_steps(3);
    row(&h, "Drop Shadow").click();
    h.run_steps(3);
    assert_eq!(effects(&h), [(1, EffectKind::DropShadow, true)], "the row click leaves the check box alone");
    assert_eq!(h.state().draft.page, LayerStylePage::Effect(EffectId(1)));
}

// A click on the dimmed Satin row reports a Set whose draft has a new Satin, on, selected.
#[test]
fn absent_effect_row_adds_the_effect_on() {
    let mut h = rig();
    assert_eq!(h.get_by_role_and_label(Role::CheckBox, "Satin").accesskit_node().toggled(), Some(Toggled::False));
    row(&h, "Satin").click();
    h.run_steps(3);
    assert_eq!(effects(&h), [(1, EffectKind::DropShadow, true), (2, EffectKind::Satin, true)]);
    assert_eq!(h.state().draft.page, LayerStylePage::Effect(EffectId(2)));
    assert!(matches!(h.state().events.as_slice(), [EditEvent::Set { .. }]));
}

// `+` on Drop Shadow adds a second instance after the first; `−` shows only with two and removes
// one; Satin rows have neither button.
#[test]
fn plus_and_minus_add_and_remove_instances_of_multi_effects() {
    let mut h = rig();
    assert!(h.query_by_label("Remove this Drop Shadow").is_none(), "one instance: no minus");
    h.get_by_label("Add another Drop Shadow").click();
    h.run_steps(3);
    assert_eq!(effects(&h), [(1, EffectKind::DropShadow, true), (2, EffectKind::DropShadow, true)]);

    h.state_mut().draft.style.effects.push(EffectInstance { id: EffectId(3), kind: EffectKind::Satin, on: true, params: StyleParams::default() });
    h.run_steps(3);
    assert!(h.query_by_label("Add another Satin").is_none() && h.query_by_label("Remove this Satin").is_none());

    let Some(minus) = h.get_all_by_label("Remove this Drop Shadow").next() else { panic!("two instances show a minus") };
    minus.click();
    h.run_steps(3);
    assert_eq!(effects(&h), [(2, EffectKind::DropShadow, true), (3, EffectKind::Satin, true)]);
}

// Tab reaches the list once; ↑ and ↓ move the lead, Space toggles, Enter selects the page; the
// AccessKit tree has a ListBox of CheckBox rows with positions.
#[test]
fn effect_list_is_one_tab_stop_with_arrow_navigation() {
    let mut h = rig();
    let list = h.get_by_role_and_label(Role::ListBox, "Effects");
    assert!(list.accesskit_node().size_of_set().is_some());
    assert!(row(&h, "Drop Shadow").accesskit_node().position_in_set().is_some());

    h.key_press(Key::Tab);
    h.run_steps(3);
    assert!(h.ctx.memory(|m| m.focused().is_some()), "Tab focuses the list");
    // Styles, Blending Options, then over the gap to Drop Shadow.
    h.key_press(Key::ArrowDown);
    h.run_steps(2);
    h.key_press(Key::ArrowDown);
    h.run_steps(2);
    assert_eq!(h.state().draft.page, LayerStylePage::Styles, "the arrows move the lead only");
    h.key_press(Key::Enter);
    h.run_steps(3);
    assert_eq!(h.state().draft.page, LayerStylePage::Effect(EffectId(1)));
    assert_eq!(effects(&h), [(1, EffectKind::DropShadow, true)], "Enter does not toggle");

    h.key_press(Key::Space);
    h.run_steps(3);
    assert_eq!(effects(&h), [(1, EffectKind::DropShadow, false)]);

    h.key_press(Key::ArrowDown);
    h.run_steps(2);
    h.key_press(Key::Space);
    h.run_steps(3);
    assert_eq!(effects(&h), [(1, EffectKind::DropShadow, false), (2, EffectKind::Satin, true)], "Space on an absent row adds it");
}

// A click on a preset of the Styles page reports `apply_style` with its key and no edit.
#[test]
fn styles_page_click_reports_apply_style() {
    let mut h = rig();
    h.get_by_label("Soft").click();
    h.run_steps(3);
    assert_eq!(h.state().applied, ["Soft"]);
    assert!(h.state().events.is_empty(), "the host applies the preset; the dialog reports no edit");
    assert_eq!(h.state().draft.page, LayerStylePage::Styles);
}

// `set_color(draft, "fx2/color", red)` sets that parameter; an unknown id or a key without a
// slash changes nothing and returns false.
#[test]
fn set_color_writes_the_named_parameter_and_ignores_unknown_keys() {
    let mut style = LayerStyle::default();
    style.effects.push(shadow(1, true));
    style.effects.push(shadow(2, true));
    let mut draft = LayerStyleDraft::new(style);
    let red = Srgba::from_hex("#ff0000").unwrap_or(Srgba::BLACK);

    assert!(LayerStyleDialog::set_color(&mut draft, "fx2/color", red));
    assert_eq!(draft.style.effects[1].params.get("color"), Some(&StyleValue::Color(red)));
    assert_eq!(draft.style.effects[0].params.get("color"), None);

    let before = draft.clone();
    for key in ["fx9/color", "color", "fx2", "fx2/", "x/color"] {
        assert!(!LayerStyleDialog::set_color(&mut draft, key, Srgba::BLACK), "{key}");
    }
    assert_eq!(draft, before);
}

// The Preview check box at the title's end flips `draft.preview` as a Set; the render hash stays.
#[test]
fn preview_check_box_flips_the_drafts_preview() {
    let mut h = rig();
    assert!(h.query_by_role_and_label(Role::CheckBox, "Preview").is_none(), "the Styles page has no Preview");
    h.state_mut().draft.page = LayerStylePage::Effect(EffectId(1));
    h.run_steps(3);
    let hash = h.state().draft.render_hash();
    h.get_by_role_and_label(Role::CheckBox, "Preview").click();
    h.run_steps(3);
    assert!(!h.state().draft.preview);
    assert_eq!(h.state().draft.render_hash(), hash);
    assert!(matches!(h.state().events.as_slice(), [EditEvent::Set { .. }]));
}
