//! Tests of the Layer Style model.

use super::*;

fn style_with(kinds: &[(u32, EffectKind)]) -> LayerStyle {
    let effects =
        kinds.iter().map(|&(n, kind)| EffectInstance { id: EffectId(n), kind, on: true, params: EffectSpec::photoshop(kind).defaults }).collect();
    LayerStyle { effects, ..LayerStyle::default() }
}

fn ids(style: &LayerStyle) -> Vec<u32> {
    style.effects.iter().map(|e| e.id.0).collect()
}

fn pt(input: f32, output: f32) -> ContourPoint {
    ContourPoint { input, output, corner: false }
}

// Every field, and every Anti-aliased check beside a contour, has a default.
#[test]
fn every_effect_field_has_a_default() {
    for kind in EffectKind::ALL {
        let spec = EffectSpec::photoshop(kind);
        for field in &spec.fields {
            assert!(spec.defaults.get(&field.key).is_some(), "{kind:?} has no default for {}", field.key);
            if let FieldKind::Contour { anti_alias: Some(key) } = &field.kind {
                assert!(spec.defaults.get(key).is_some(), "{kind:?} has no default for {key}");
            }
        }
    }
}

// With fx1 Stroke, fx2 Drop Shadow, fx5 Stroke: adding a Stroke inserts fx6 after fx5 and selects
// it; adding a Satin appends it.
#[test]
fn adding_an_instance_places_it_after_the_last_of_its_kind_and_numbers_ids() {
    let mut draft = LayerStyleDraft::new(style_with(&[(1, EffectKind::Stroke), (5, EffectKind::Stroke), (2, EffectKind::DropShadow)]));
    let stroke = EffectSpec::photoshop(EffectKind::Stroke).defaults;
    assert_eq!(draft.add_instance(EffectKind::Stroke, &stroke), EffectId(6));
    assert_eq!(ids(&draft.style), [1, 5, 6, 2]);
    assert_eq!(draft.page, LayerStylePage::Effect(EffectId(6)));

    let satin = EffectSpec::photoshop(EffectKind::Satin).defaults;
    assert_eq!(draft.add_instance(EffectKind::Satin, &satin), EffectId(7));
    assert_eq!(ids(&draft.style), [1, 5, 6, 2, 7]);

    // A new globally lit shadow takes the shared angle.
    draft.style.global_light = 45.0;
    let shadow = draft.add_instance(EffectKind::DropShadow, &EffectSpec::photoshop(EffectKind::DropShadow).defaults);
    assert_eq!(draft.style.effect(shadow).and_then(|e| e.params.number("angle")), Some(45.0));
}

// Removing the selected of two Drop Shadows selects the other; removing the only one selects the
// Styles page; removing an unselected one keeps the page.
#[test]
fn removing_an_instance_selects_a_sibling_of_the_same_kind_else_the_styles_page() {
    let mut draft = LayerStyleDraft::new(style_with(&[(1, EffectKind::DropShadow), (2, EffectKind::DropShadow), (3, EffectKind::Satin)]));
    draft.page = LayerStylePage::Effect(EffectId(2));
    draft.remove_instance(EffectId(2));
    assert_eq!(draft.page, LayerStylePage::Effect(EffectId(1)));

    draft.remove_instance(EffectId(3));
    assert_eq!(draft.page, LayerStylePage::Effect(EffectId(1)));

    draft.remove_instance(EffectId(1));
    assert_eq!(draft.page, LayerStylePage::Styles);
    assert!(draft.style.effects.is_empty());
}

// Setting a globally lit Drop Shadow's angle sets `global_light` and the angle of every globally
// lit instance, a disabled Bevel too; turning Use Global Light on adopts the shared angle.
#[test]
fn global_light_angle_is_shared_by_effects_that_use_it() {
    let mut draft = LayerStyleDraft::new(style_with(&[(1, EffectKind::DropShadow), (2, EffectKind::BevelEmboss), (3, EffectKind::Satin)]));
    draft.toggle(EffectId(2));
    assert!(draft.style.effect(EffectId(2)).is_some_and(|e| !e.on));

    draft.set_param(EffectId(1), "angle", StyleValue::Number(45.0));
    assert_eq!(draft.style.global_light, 45.0);
    assert_eq!(draft.style.effect(EffectId(2)).and_then(|e| e.params.number("angle")), Some(45.0));
    // Satin has no Use Global Light.
    assert_eq!(draft.style.effect(EffectId(3)).and_then(|e| e.params.number("angle")), Some(19.0));

    // With Use Global Light off the angle is the instance's own.
    draft.set_param(EffectId(1), "useGlobalLight", StyleValue::Bool(false));
    draft.set_param(EffectId(1), "angle", StyleValue::Number(10.0));
    assert_eq!(draft.style.global_light, 45.0);
    assert_eq!(draft.style.effect(EffectId(2)).and_then(|e| e.params.number("angle")), Some(45.0));

    draft.set_param(EffectId(1), "useGlobalLight", StyleValue::Bool(true));
    assert_eq!(draft.style.effect(EffectId(1)).and_then(|e| e.params.number("angle")), Some(45.0));
}

// From 120° and 5 px, a drag of (4.5, −2.2) lands on whole degrees and pixels by the offset rule;
// with Use Global Light the shared angle follows; a drag back under 0.5 px keeps the angle.
#[test]
fn shadow_drag_uses_whole_degrees_and_pixels_and_moves_the_global_angle() {
    let mut draft = LayerStyleDraft::new(style_with(&[(1, EffectKind::DropShadow)]));
    assert!(draft.drags_shadow());
    let defaults = EffectSpec::photoshop(EffectKind::DropShadow).defaults;
    assert!(draft.drag_shadow(&defaults, true, [0.0, 0.0]) || draft.shadow_drag.is_some());

    assert!(draft.drag_shadow(&defaults, false, [4.5, -2.2]));
    let params = &draft.style.effects[0].params;
    assert_eq!((params.number("angle"), params.number("distance")), (Some(163.0), Some(7.0)));
    assert_eq!(draft.style.global_light, 163.0);

    // Back to the press point: the offset is under 0.5 px, so the start angle returns.
    assert!(draft.drag_shadow(&defaults, false, [-2.5, -4.330_127]));
    let params = &draft.style.effects[0].params;
    assert_eq!((params.number("angle"), params.number("distance")), (Some(120.0), Some(0.0)));
    assert_eq!(draft.style.global_light, 120.0);

    // Other pages are not dragged.
    draft.page = LayerStylePage::Blending;
    assert!(!draft.drag_shadow(&defaults, true, [10.0, 10.0]));
}

// An instance without angle and distance starts the drag from the spec's defaults, not from 120°
// and 0.
#[test]
fn shadow_drag_starts_from_the_defaults_under_the_instances_values() {
    let mut style = style_with(&[(1, EffectKind::DropShadow)]);
    style.effects[0].params = StyleParams::default();
    let mut draft = LayerStyleDraft::new(style);
    let mut defaults = StyleParams::default();
    defaults.set("angle", StyleValue::Number(30.0));
    defaults.set("distance", StyleValue::Number(10.0));
    assert!(draft.drag_shadow(&defaults, true, [0.0, 0.0]));
    assert_eq!(draft.shadow_drag, Some(ShadowDrag::begin(30.0, 10.0)));
    let params = &draft.style.effects[0].params;
    assert_eq!((params.number("angle"), params.number("distance")), (Some(30.0), Some(10.0)));
}

// The blend defaults of a spec become the host's keys; unknown names stay. Bevel & Emboss has two
// Blend fields: only the one defaulting to Multiply is renamed.
#[test]
fn blend_keys_remap_the_blend_defaults_only() {
    let map: std::collections::HashMap<String, String> = [("Multiply".to_owned(), "mul".to_owned())].into();
    let spec = EffectSpec::photoshop(EffectKind::DropShadow).with_blend_keys(&map);
    assert_eq!(spec.defaults.get("blend"), Some(&StyleValue::Text("mul".to_owned())));
    let bevel = EffectSpec::photoshop(EffectKind::BevelEmboss);
    let mapped = bevel.clone().with_blend_keys(&map);
    assert_eq!(mapped.defaults.get("shadowBlend"), Some(&StyleValue::Text("mul".to_owned())));
    assert_eq!(mapped.defaults.get("highlightBlend"), bevel.defaults.get("highlightBlend"));
}

// `BlendIfRange::ordered` and `moved` always give black low ≤ black high ≤ white low ≤ white
// high, pushing the other handles.
#[test]
fn blend_if_edits_keep_the_four_values_ordered() {
    assert_eq!(BlendIfRange::ordered([10, 5, 200, 100]), BlendIfRange([10, 10, 200, 200]));
    let start = BlendIfRange([10, 30, 200, 220]);
    for handle in 0..4 {
        for to in (0..=255u8).step_by(5) {
            for split in [false, true] {
                let r = start.moved(handle, to, split).0;
                assert!(r.windows(2).all(|w| w[0] <= w[1]), "{handle} {to} {split}: {r:?}");
            }
        }
    }
    // A split move past a neighbour pushes it.
    assert_eq!(start.moved(1, 210, true), BlendIfRange([10, 210, 210, 220]));
    assert_eq!(start.moved(9, 0, false), start);
}

// An unsplit move of the black pair [10, 30] by +20 gives [30, 50]; with `split` only the moved
// half changes.
#[test]
fn blend_if_unsplit_move_keeps_the_spread_and_alt_move_splits() {
    let start = BlendIfRange([10, 30, 200, 220]);
    assert_eq!(start.moved(0, 30, false), BlendIfRange([30, 50, 200, 220]));
    assert_eq!(start.moved(0, 20, true), BlendIfRange([20, 30, 200, 220]));
    // The white pair clamps at the ends.
    assert_eq!(start.moved(3, 255, false).0[2..], [235, 255]);
}

// Two drafts with the same style on different pages, Blend If channels and Preview states hash
// equally; a changed opacity changes the hash.
#[test]
fn layer_style_equal_values_have_equal_render_hash_ignoring_the_selected_page() {
    let style = style_with(&[(1, EffectKind::DropShadow), (2, EffectKind::Stroke)]);
    let a = LayerStyleDraft::new(style.clone());
    let mut b = LayerStyleDraft::new(style);
    b.page = LayerStylePage::Styles;
    b.blend_if_channel = 2;
    b.preview = false;
    assert_eq!(a.render_hash(), b.render_hash());

    b.set_param(EffectId(1), "opacity", StyleValue::Number(60.0));
    assert_ne!(a.render_hash(), b.render_hash());

    let mut c = a.clone();
    c.style.blending.opacity = 0.0;
    let mut d = a.clone();
    d.style.blending.opacity = -0.0;
    assert_eq!(c.render_hash(), d.render_hash());
    d.style.blending.opacity = 1.0;
    assert_ne!(c.render_hash(), d.render_hash());
}

// A `StyleValue::Other` key no field shows survives `set_param`, `toggle` and
// `StyleParams::over` unchanged.
#[test]
fn the_layer_style_snapshot_is_kept_for_fields_the_dialog_does_not_model() {
    let mut style = style_with(&[(1, EffectKind::GradientOverlay)]);
    let other = StyleValue::Other("stops:v1".into());
    style.effects[0].params.set("gradientData", other.clone());
    let mut draft = LayerStyleDraft::new(style);
    draft.set_param(EffectId(1), "opacity", StyleValue::Number(40.0));
    draft.toggle(EffectId(1));
    draft.set_param(EffectId(1), "angle", StyleValue::Number(30.0));
    assert_eq!(draft.style.effects[0].params.get("gradientData"), Some(&other));

    let shown = StyleParams::default().over(&draft.style.effects[0].params);
    assert_eq!(shown.get("gradientData"), Some(&other));
    let sparse = StyleParams(std::iter::once(("opacity".to_owned(), StyleValue::Number(10.0))).collect());
    let shown = sparse.over(&EffectSpec::photoshop(EffectKind::GradientOverlay).defaults);
    assert_eq!(shown.number("opacity"), Some(10.0));
    assert_eq!(shown.number("scale"), Some(100.0));
}

// `Contour::from_points` rejects one point, 65 points and NaN, sorts and clamps, collapses equal
// inputs; `edited` renames a preset to "Custom" and keeps an imported name.
#[test]
fn contours_are_validated_and_an_edited_preset_becomes_custom() {
    assert!(Contour::from_points("x", vec![pt(0.0, 0.0)]).is_none());
    assert!(Contour::from_points("x", (0..65).map(|i| pt(i as f32 / 64.0, 0.5)).collect()).is_none());
    assert!(Contour::from_points("x", vec![pt(0.0, 0.0), pt(f32::NAN, 1.0)]).is_none());
    assert!(Contour::from_points("x", vec![pt(0.5, 0.0), pt(0.5, 1.0)]).is_none());

    let c = Contour::from_points("x", vec![pt(1.5, 2.0), pt(0.0, -1.0), pt(0.5, 0.5), pt(0.5, 0.9)]);
    let Some(c) = c else { panic!("a valid contour") };
    let inputs: Vec<f32> = c.points.iter().map(|p| p.input).collect();
    assert_eq!(inputs, [0.0, 0.5, 1.0]);
    assert_eq!((c.points[0].output, c.points[1].output, c.points[2].output), (0.0, 0.5, 1.0));

    let presets = Contour::presets();
    assert!(!presets.is_empty());
    assert_eq!(presets[0], Contour::linear());
    assert!(presets.iter().all(|p| p.is_preset() && Contour::from_points(p.name.clone(), p.points.clone()).as_ref() == Some(p)));
    assert_eq!(Contour::preset("linear"), Some(Contour::linear()));
    assert!(Contour::preset("Domed (Inverted)").is_some());
    assert!(Contour::preset("Imported").is_none());

    let cone = Contour::preset("Cone");
    let Some(mut cone) = cone else { panic!("Cone ships") };
    cone.points[1].output += 0.1;
    assert!(!cone.is_preset());
    assert_eq!(cone.clone().edited().name, "Custom");
    assert_eq!(Contour { name: String::new(), ..cone.clone() }.edited().name, "Custom");
    assert_eq!(Contour { name: "My curve".into(), ..cone }.edited().name, "My curve");
}
