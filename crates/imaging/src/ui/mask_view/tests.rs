//! Tests of the mask views.

use egui::{Context, Event, Key, Modifiers, RawInput};

use super::*;
use crate::selection::ColorIndicates;

/// One pass of `ctx` with `events` and `modifiers` held, then the keys read into `state`.
fn pass(ctx: &Context, state: &mut MaskViewState, events: Vec<Event>, modifiers: Modifiers) -> Vec<MaskViewChange> {
    let mut changes = Vec::new();
    let mut all = vec![Event::ModifiersChanged(modifiers)];
    all.extend(events);
    let mut output = ctx.run_ui(RawInput { events: all, ..Default::default() }, |ui| {
        changes = MaskViewKeys::read(ui, state);
    });
    output.textures_delta.clear();
    changes
}

fn key(key: Key, pressed: bool, modifiers: Modifiers) -> Event {
    Event::Key { key, physical_key: Some(key), pressed, repeat: false, modifiers }
}

// O M V A T K Y pick their views, F cycles in menu order (On Layers to Onion Skin), J and P toggle,
// X held hides the view until it is released; nothing is read with ⌘ held or while a text field has
// focus.
#[test]
fn mask_view_keys_pick_cycle_and_hold() {
    let ctx = Context::default();
    let mut state = MaskViewState::default();
    for mode in MaskViewMode::ALL {
        let changes = pass(&ctx, &mut state, vec![key(mode.key(), true, Modifiers::NONE)], Modifiers::NONE);
        assert_eq!(changes, [MaskViewChange::Mode(mode)]);
        assert_eq!(state.mode, mode);
        pass(&ctx, &mut state, vec![key(mode.key(), false, Modifiers::NONE)], Modifiers::NONE);
    }
    assert_eq!(state.mode, MaskViewMode::OnLayers);
    pass(&ctx, &mut state, vec![key(Key::F, true, Modifiers::NONE)], Modifiers::NONE);
    assert_eq!(state.mode, MaskViewMode::OnionSkin, "F wraps from On Layers");
    pass(&ctx, &mut state, vec![key(Key::F, false, Modifiers::NONE)], Modifiers::NONE);
    pass(&ctx, &mut state, vec![key(Key::F, true, Modifiers::NONE)], Modifiers::NONE);
    assert_eq!(state.mode, MaskViewMode::MarchingAnts);

    pass(&ctx, &mut state, vec![key(Key::J, true, Modifiers::NONE), key(Key::P, true, Modifiers::NONE)], Modifiers::NONE);
    assert!(state.show_edge && state.show_original);
    pass(&ctx, &mut state, vec![key(Key::J, false, Modifiers::NONE)], Modifiers::NONE);
    pass(&ctx, &mut state, vec![key(Key::J, true, Modifiers::NONE)], Modifiers::NONE);
    assert!(!state.show_edge);

    pass(&ctx, &mut state, vec![key(Key::X, true, Modifiers::NONE)], Modifiers::NONE);
    assert!(state.hide_view, "X held");
    pass(&ctx, &mut state, vec![], Modifiers::NONE);
    assert!(state.hide_view, "still held");
    let changes = pass(&ctx, &mut state, vec![key(Key::X, false, Modifiers::NONE)], Modifiers::NONE);
    assert_eq!(changes, [MaskViewChange::HideView(false)]);

    let before = state.clone();
    pass(&ctx, &mut state, vec![key(Key::O, true, Modifiers::COMMAND)], Modifiers::COMMAND);
    assert_eq!(state, before, "⌘O is not a view key");
}

// While a modal layer other than the reading one is on top, no view key is read.
#[test]
fn mask_view_keys_wait_for_the_top_modal_layer() {
    let ctx = Context::default();
    let mut state = MaskViewState::default();
    let mut changes = Vec::new();
    for pressed in [false, true] {
        let events = if pressed { vec![key(Key::M, true, Modifiers::NONE)] } else { vec![] };
        let mut output = ctx.run_ui(RawInput { events, ..Default::default() }, |ui| {
            egui::Modal::new(egui::Id::unique("dialog")).show(ui.ctx(), |_| {});
            changes = MaskViewKeys::read(ui, &mut state);
        });
        output.textures_delta.clear();
    }
    assert!(changes.is_empty() && state.mode == MaskViewMode::OnionSkin, "{changes:?}");
}

// Each view keeps its own opacity: set Overlay to 30, switch to On Black (100) and back, Overlay
// reads 30; the defaults are 50, 50, 100, 100.
#[test]
fn mask_view_opacity_kept_per_view() {
    let mut state = MaskViewState::default();
    let defaults: Vec<_> = MaskViewMode::ALL.into_iter().map(|m| state.opacity(m)).collect();
    assert_eq!(defaults, [Some(50.0), None, Some(50.0), Some(100.0), Some(100.0), None, None]);
    state.apply(MaskViewChange::Opacity(MaskViewMode::Overlay, 30.4));
    state.apply(MaskViewChange::Mode(MaskViewMode::OnBlack));
    assert_eq!(state.opacity(MaskViewMode::OnBlack), Some(100.0));
    state.apply(MaskViewChange::Mode(MaskViewMode::Overlay));
    assert_eq!(state.opacity(MaskViewMode::Overlay), Some(30.0));
    state.set_opacity(MaskViewMode::Overlay, 250.0);
    assert_eq!(state.opacity(MaskViewMode::Overlay), Some(100.0));
    state.set_opacity(MaskViewMode::OnLayers, 10.0);
    assert_eq!(state.opacity(MaskViewMode::OnLayers), None);
}

// The recipe per view: veil colours and masks, the layer floor `1 − o` (Onion Skin) and 0 (On
// Layers), ants only in Marching Ants, the coverage from Show Edge and Show Original, `hidden`
// while X is held.
#[test]
fn mask_view_recipe_per_mode() {
    let mut state = MaskViewState::default();
    let recipe = state.recipe();
    assert_eq!((recipe.layer_floor, recipe.hide_other_layers, recipe.ants), (Some(0.5), true, false));
    assert!(recipe.veils.is_empty());

    state.mode = MaskViewMode::MarchingAnts;
    let recipe = state.recipe();
    assert!(recipe.ants && recipe.veils.is_empty() && recipe.layer_floor.is_none());

    state.mode = MaskViewMode::Overlay;
    let veil = state.recipe().veils[0];
    assert_eq!((veil.color, veil.opacity, veil.mask), (state.overlay_color, 0.5, VeilMask::Inverse));
    state.overlay_indicates = ColorIndicates::SelectedAreas;
    assert_eq!(state.recipe().veils[0].mask, VeilMask::Coverage);

    state.mode = MaskViewMode::OnWhite;
    let veil = state.recipe().veils[0];
    assert_eq!((veil.color.r, veil.opacity, veil.mask), (1.0, 1.0, VeilMask::Inverse));
    state.mode = MaskViewMode::OnBlack;
    assert_eq!(state.recipe().veils[0].color.r, 0.0);

    state.mode = MaskViewMode::BlackAndWhite;
    let masks: Vec<_> = state.recipe().veils.iter().map(|v| (v.color.r, v.opacity, v.mask)).collect();
    assert_eq!(masks, [(0.0, 1.0, VeilMask::Everywhere), (1.0, 1.0, VeilMask::Coverage)]);

    state.mode = MaskViewMode::OnLayers;
    let recipe = state.recipe();
    assert_eq!((recipe.layer_floor, recipe.hide_other_layers), (Some(0.0), false));

    assert_eq!(state.recipe().coverage, MaskCoverage::Refined);
    state.show_original = true;
    assert_eq!(state.recipe().coverage, MaskCoverage::Original);
    state.show_edge = true;
    assert_eq!(state.recipe().coverage, MaskCoverage::EdgeBand);
    state.hide_view = true;
    assert!(state.recipe().hidden);
}

#[test]
fn mask_view_ids_round_trip() {
    for mode in MaskViewMode::ALL {
        assert_eq!(MaskViewMode::from_id(mode.id()), Some(mode));
    }
    assert_eq!(MaskViewMode::BlackAndWhite.id(), "blackWhite");
}

// Picking a view in the drop-down and moving the Opacity slider report one change each, applied to
// the state; the slider is absent for Marching Ants.
#[cfg(all(feature = "screen-testing", not(target_arch = "wasm32")))]
#[test]
fn mask_view_controls_report_changes() {
    use craft_ui::theme::{Base, install};
    use egui::accesskit::Role;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable as _;

    struct Host {
        state: MaskViewState,
        last: Option<MaskViewChange>,
    }
    let mut harness =
        Harness::builder().with_size([480.0, 480.0]).with_context_setup(|ctx: &Context| install(ctx, &Base::suite_default())).build_ui_state(
            |ui, host: &mut Host| {
                if let Some(change) = MaskViewControls::new("view", &mut host.state).show(ui).changed {
                    host.last = Some(change);
                }
            },
            Host { state: MaskViewState::default(), last: None },
        );
    harness.run();

    harness.get_by_role(Role::Slider).focus();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    assert!(matches!(harness.state().last, Some(MaskViewChange::Opacity(MaskViewMode::OnionSkin, v)) if v > 50.0));
    assert!(harness.state().state.opacity(MaskViewMode::OnionSkin).is_some_and(|v| v > 50.0));

    harness.get_by_role(Role::ComboBox).click();
    harness.run();
    harness.get_by(|n| n.role() == Role::ListBoxOption && n.label().as_deref().is_some_and(|l| l.starts_with("Marching Ants"))).click();
    harness.run();
    assert_eq!(harness.state().last, Some(MaskViewChange::Mode(MaskViewMode::MarchingAnts)));
    assert_eq!(harness.state().state.mode, MaskViewMode::MarchingAnts);
    assert!(harness.query_by_role(Role::Slider).is_none(), "Marching Ants has no opacity");
}
