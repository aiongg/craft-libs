//! Tests of the effect pages and the Blending Options page (task AD-9).

use egui::Key;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT as _, Queryable as _};

use craft_theme::Srgba;
use craft_ui::dialog::ColorRequest;
use craft_ui::localized::LocaleNumbers;
use craft_ui::theme::{Base, install};

use super::blending::{BlendingInput, blend_if_entry_label, blending_page};
use super::labels::{BlendIfBarLabels, ContourFieldLabels, LayerStyleLabels};
use super::page::{FieldEdit, PageInput, effect_page, first_invalid};
use crate::layer_style::{Blending, EffectId, EffectInstance, EffectKind, EffectSpec, LayerStyle, LayerStyleDraft, StyleParams, StyleValue};

const SIZE: [f32; 2] = [360.0, 1800.0];

/// Drop Shadow without its Contour row: the Contour field is task AD-10's.
fn shadow_spec() -> EffectSpec {
    EffectSpec::photoshop(EffectKind::DropShadow)
}

struct PageRig {
    spec: EffectSpec,
    draft: LayerStyleDraft,
    /// Apply each reported edit to the draft with `set_param`, as the dialog does.
    apply: bool,
    edits: Vec<FieldEdit>,
    requests: Vec<ColorRequest>,
    made: Vec<StyleParams>,
}

fn page_rig(params: Option<StyleParams>, apply: bool) -> Harness<'static, PageRig> {
    let spec = shadow_spec();
    let params = params.unwrap_or_else(|| spec.defaults.clone());
    let effects = vec![EffectInstance { id: EffectId(1), kind: EffectKind::DropShadow, on: true, params }];
    let draft = LayerStyleDraft::new(LayerStyle { effects, ..LayerStyle::default() });
    let state = PageRig { spec, draft, apply, edits: Vec::new(), requests: Vec::new(), made: Vec::new() };
    let mut h = Harness::builder().with_size(SIZE).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, rig: &mut PageRig| {
            let (labels, contour_labels, numbers) = (LayerStyleLabels::default(), ContourFieldLabels::default(), LocaleNumbers::from_tag("en"));
            let Some(instance) = rig.draft.style.effect(EffectId(1)).cloned() else { panic!("the instance exists") };
            let input = PageInput {
                spec: &rig.spec,
                instance: &instance,
                blend_modes: &[],
                patterns: &[],
                user_contours: &[],
                labels: &labels,
                contour_labels: &contour_labels,
                numbers: &numbers,
                enabled: true,
            };
            let out = effect_page(ui, &input);
            if let Some(edit) = out.edit {
                if let (true, Some(value)) = (rig.apply, edit.event.value()) {
                    rig.draft.set_param(EffectId(1), &edit.key, value.clone());
                }
                rig.edits.push(edit);
            }
            rig.requests.extend(out.pick_color);
            rig.made.extend(out.make_default);
        },
        state,
    );
    h.run_steps(3);
    h
}

// A click on Drop Shadow's Color swatch reports a ColorRequest keyed "fx1/color" with the current
// colour and the dialog's picker title.
#[test]
fn colour_field_reports_a_colour_request_with_the_effect_and_key() {
    let mut h = page_rig(None, false);
    h.get_by_role_and_label(Role::ColorWell, "Color").click();
    h.run_steps(3);
    let expected = ColorRequest::new("fx1/color", Srgba::BLACK).title(LayerStyleLabels::default().color_picker_title);
    let requests = &h.state().requests;
    let [request] = requests.as_slice() else { panic!("one request: {}", requests.len()) };
    assert_eq!((&request.key, &request.title, request.color), (&expected.key, &expected.title, expected.color));
}

// With Use Global Light on, dragging the Angle dial changes `global_light` and every globally lit
// instance; with it off, only this instance's angle.
#[test]
fn use_global_light_check_changes_the_angle_field_meaning() {
    let mut h = page_rig(None, true);
    h.get_by_role_and_label(Role::Slider, "Angle").focus();
    h.key_press(Key::ArrowRight);
    h.run_steps(3);
    let angle = |h: &Harness<'_, PageRig>| h.state().draft.style.effect(EffectId(1)).and_then(|e| e.params.number("angle"));
    assert_eq!((h.state().draft.style.global_light, angle(&h)), (121.0, Some(121.0)));

    h.get_by_role_and_label(Role::CheckBox, "Use Global Light").click();
    h.run_steps(3);
    h.get_by_role_and_label(Role::Slider, "Angle").focus();
    h.key_press(Key::ArrowRight);
    h.run_steps(3);
    assert_eq!((h.state().draft.style.global_light, angle(&h)), (121.0, Some(122.0)));
}

struct BlendingRig {
    blending: Blending,
    names: Vec<String>,
    edits: Vec<Blending>,
}

fn blending_rig(blending: Blending, names: &[&str]) -> Harness<'static, BlendingRig> {
    let state = BlendingRig { blending, names: names.iter().map(|n| (*n).to_owned()).collect(), edits: Vec::new() };
    let mut h = Harness::builder().with_size(SIZE).with_context_setup(|ctx| install(ctx, &Base::suite_default())).build_ui_state(
        |ui, rig: &mut BlendingRig| {
            let (labels, bar_labels, numbers) = (LayerStyleLabels::default(), BlendIfBarLabels::default(), LocaleNumbers::from_tag("en"));
            let input = BlendingInput {
                blending: &rig.blending,
                blend_if_channel: 0,
                channel_names: &rig.names,
                blend_modes: &[],
                labels: &labels,
                bar_labels: &bar_labels,
                numbers: &numbers,
                enabled: true,
            };
            let out = blending_page(ui, &input);
            rig.edits.extend(out.edit.and_then(|e| e.value().cloned()));
        },
        state,
    );
    h.run_steps(3);
    h
}

// With channel names C, M, Y, K the page shows four Channels checks; unchecking M reports
// `channels` [true, false, true, true].
#[test]
fn blending_options_channel_checks_follow_the_document_channels() {
    let blending = Blending { channels: vec![true; 4], blend_if: Vec::new(), ..Blending::default() };
    let mut h = blending_rig(blending, &["C", "M", "Y", "K"]);
    for name in ["C", "Y", "K"] {
        let _ = h.get_by_role_and_label(Role::CheckBox, name);
    }
    h.get_by_role_and_label(Role::CheckBox, "M").click();
    h.run_steps(3);
    let edits = &h.state().edits;
    let [edit] = edits.as_slice() else { panic!("one edit: {}", edits.len()) };
    assert_eq!(edit.channels, [true, false, true, true]);
}

// The Blend If select names Gray, R, G, B for an RGB document with four entries, and only Gray for
// a one-channel document.
#[test]
fn blend_if_select_names_gray_first_in_colour_documents() {
    let rgb: Vec<String> = ["R", "G", "B"].map(String::from).to_vec();
    let names: Vec<_> = (0..4).map(|i| blend_if_entry_label(i, 4, &rgb, "Gray")).collect();
    assert_eq!(names, ["Gray", "R", "G", "B"]);
    assert_eq!(blend_if_entry_label(0, 1, &["Gray".to_owned()], "Gray"), "Gray");
}

// A page whose `opacity` is Text shows one invalid-value line naming Opacity and no fields; the
// draft is reported unchanged.
#[test]
fn invalid_values_show_one_message_and_are_kept() {
    let mut params = shadow_spec().defaults;
    params.set("opacity", StyleValue::Text("high".into()));
    assert_eq!(first_invalid(&shadow_spec(), &params).map(|f| f.key.as_ref()), Some("opacity"));
    let h = page_rig(Some(params.clone()), true);
    let _ = h.get_by_label_contains("Opacity has a value");
    assert!(h.query_by_role(Role::Slider).is_none() && h.query_by_role(Role::CheckBox).is_none());
    assert!(h.state().edits.is_empty());
    assert_eq!(h.state().draft.style.effect(EffectId(1)).map(|e| &e.params), Some(&params));
}

// A page shows the spec's defaults for keys the instance lacks without writing them; editing one
// field writes only that key.
#[test]
fn page_shows_defaults_under_the_values_and_writes_only_edits() {
    let mut h = page_rig(Some(StyleParams::default()), false);
    assert!(h.state().edits.is_empty());
    let distance = h.get_by_role_and_label(Role::Slider, "Distance");
    assert_eq!(distance.accesskit_node().numeric_value(), Some(5.0));
    h.get_by_role_and_label(Role::Button, "Make Default").click();
    h.run_steps(3);
    let made = &h.state().made;
    let [shown] = made.as_slice() else { panic!("one Make Default: {}", made.len()) };
    assert_eq!(shown.number("opacity"), Some(75.0));

    h.get_by_role_and_label(Role::Slider, "Distance").focus();
    h.key_press(Key::ArrowRight);
    h.run_steps(3);
    let edits = &h.state().edits;
    let [edit] = edits.as_slice() else { panic!("one edit: {}", edits.len()) };
    assert_eq!((edit.key.as_str(), edit.event.value()), ("distance", Some(&StyleValue::Number(6.0))));
}
