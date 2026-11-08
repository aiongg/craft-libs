//! `LayerStyleDialog`: the effect list, the Styles page, the effect pages and the Blending Options page, driven through
//! the dialog the way a host shows it.

use std::borrow::Cow;
use std::cell::RefCell;
use std::rc::Rc;

use craft_behavior::EditEvent;
use craft_imaging::layer_style::{
    Blending, EffectField, EffectId, EffectInstance, EffectKind, EffectSpec, FieldKind, FieldUnit, LayerStyle, LayerStyleDraft, LayerStylePage,
    StyleParams, StyleValue,
};
use craft_imaging::ui::layer_style::{LayerStyleDialog, StylePresets};
use craft_theme::Srgba;
use craft_ui::contract::Role;
use craft_ui::patterns::presets::PresetRow;
use craft_ui::runtime::requests::ColorRequest;
use craft_ui::testing::access::name;
use craft_ui::testing::{Fixture, Target, trial};
use egui::{Key, Modifiers, Vec2};

/// The room the dialog is shown in: tall enough for the longest effect page.
const ROOM: Vec2 = Vec2::new(900.0, 1400.0);

/// A host that applies every change and records what the dialog reported.
struct Host {
    draft: LayerStyleDraft,
    specs: Vec<EffectSpec>,
    channels: Vec<String>,
    patterns: Vec<PresetRow<String>>,
    edits: Vec<EditEvent<LayerStyle>>,
    applied: Vec<String>,
    colors: Vec<ColorRequest>,
    contours: Vec<String>,
    made: Vec<StyleParams>,
}

fn spec(kind: EffectKind) -> EffectSpec {
    EffectSpec { kind, label: Cow::Borrowed(""), fields: Vec::new(), defaults: StyleParams::default() }
}

fn instance(id: u32, kind: EffectKind, on: bool) -> EffectInstance {
    EffectInstance { id: EffectId(id), kind, on, params: StyleParams::default() }
}

fn host(draft: LayerStyleDraft, specs: Vec<EffectSpec>) -> Host {
    Host {
        draft,
        specs,
        channels: ["R", "G", "B"].map(String::from).to_vec(),
        patterns: Vec::new(),
        edits: Vec::new(),
        applied: Vec::new(),
        colors: Vec::new(),
        contours: Vec::new(),
        made: Vec::new(),
    }
}

fn rig_of(host: Host) -> (Fixture, Rc<RefCell<Host>>) {
    let host = Rc::new(RefCell::new(host));
    let shown = host.clone();
    let rows = vec![
        PresetRow::group("g".to_owned(), "Basics", 0),
        PresetRow::item("Soft".to_owned(), "Soft", 1),
        PresetRow::item("Hard".to_owned(), "Hard", 1),
    ];
    let mut fixture = Fixture::new(move |ui| {
        let mut guard = shown.borrow_mut();
        let host = &mut *guard;
        let presets = StylePresets::new(&rows, |cx, rect, _| cx.checker(rect));
        let out = LayerStyleDialog::new("Layer Style", &host.draft, &host.channels)
            .specs(&host.specs)
            .patterns(&host.patterns)
            .style_presets(presets)
            .show(ui);
        out.apply_view(&mut host.draft);
        if let Some(event) = out.edit {
            if let Some(value) = event.value() {
                host.draft.style = value.clone();
            }
            host.edits.push(event);
        }
        host.applied.extend(out.apply_style);
        host.colors.extend(out.pick_color);
        host.contours.extend(out.edit_contour.map(|request| request.key));
        host.made.extend(out.make_default.map(|(_, params)| params));
    });
    fixture.room(ROOM).settle();
    (fixture, host)
}

/// A Drop Shadow (`fx1`, on) and an absent Satin, the Styles page shown.
fn rig() -> (Fixture, Rc<RefCell<Host>>) {
    let mut style = LayerStyle::default();
    style.effects.push(instance(1, EffectKind::DropShadow, true));
    let mut draft = LayerStyleDraft::new(style);
    draft.page = LayerStylePage::Styles;
    rig_of(host(draft, vec![spec(EffectKind::DropShadow), spec(EffectKind::Satin)]))
}

/// The nodes of `role` named `label`, top to bottom.
fn named(fixture: &Fixture, role: Role, label: &str) -> Vec<Target> {
    let mut found: Vec<Target> = fixture.by_role(role).into_iter().filter(|target| name(fixture.tree(), target.node) == Some(label)).collect();
    found.sort_by(|a, b| a.rect.top().total_cmp(&b.rect.top()));
    found
}

fn one(fixture: &Fixture, role: Role, label: &str) -> Target {
    named(fixture, role, label).into_iter().next().unwrap_or_else(|| panic!("a {role:?} named {label}"))
}

fn effects(host: &Rc<RefCell<Host>>) -> Vec<(u32, EffectKind, bool)> {
    host.borrow().draft.style.effects.iter().map(|e| (e.id.0, e.kind, e.on)).collect()
}

/// A click on an instance's check box turns it off and shows its page; a click on the row only shows it.
#[trial]
fn effect_list_check_box_toggles_and_selects_the_page() {
    let (mut fixture, host) = rig();
    let check = one(&fixture, Role::CHECK_BOX, "Drop Shadow");
    fixture.click(&check);
    fixture.idle(2);
    assert_eq!(effects(&host), [(1, EffectKind::DropShadow, false)]);
    assert_eq!(host.borrow().draft.page, LayerStylePage::Effect(EffectId(1)));
    assert!(matches!(host.borrow().edits.as_slice(), [EditEvent::Set { .. }]), "one Set for the click");

    {
        let mut host = host.borrow_mut();
        host.draft.style.effects[0].on = true;
        host.draft.page = LayerStylePage::Styles;
    }
    fixture.idle(2);
    let row = one(&fixture, Role::LIST_ITEM, "Drop Shadow");
    fixture.click(&row);
    fixture.idle(2);
    assert_eq!(effects(&host), [(1, EffectKind::DropShadow, true)], "the row click leaves the check box alone");
    assert_eq!(host.borrow().draft.page, LayerStylePage::Effect(EffectId(1)));
}

/// A click on the dimmed Satin row adds a Satin, on, and shows its page, as one Set.
#[trial]
fn absent_effect_row_adds_the_effect_on() {
    let (mut fixture, host) = rig();
    let row = one(&fixture, Role::LIST_ITEM, "Satin");
    fixture.click(&row);
    fixture.idle(2);
    assert_eq!(effects(&host), [(1, EffectKind::DropShadow, true), (2, EffectKind::Satin, true)]);
    assert_eq!(host.borrow().draft.page, LayerStylePage::Effect(EffectId(2)));
    assert!(matches!(host.borrow().edits.as_slice(), [EditEvent::Set { .. }]), "one Set for the added effect");
}

/// A list click changes the page without a style edit, so the host's history has nothing to record.
#[trial]
fn a_page_switch_reports_the_page_and_no_edit() {
    let (mut fixture, host) = rig();
    let row = one(&fixture, Role::LIST_ITEM, "Blending Options");
    fixture.click(&row);
    fixture.idle(2);
    assert_eq!(host.borrow().draft.page, LayerStylePage::Blending);
    assert!(host.borrow().edits.is_empty());
}

fn size_spec(kind: EffectKind) -> EffectSpec {
    let field = EffectField { key: "size".into(), label: "Size".into(), kind: FieldKind::Slider { min: 0.0, max: 250.0, unit: FieldUnit::Pixels } };
    let mut defaults = StyleParams::default();
    defaults.set("size", StyleValue::Number(5.0));
    EffectSpec { kind, label: Cow::Borrowed(""), fields: vec![field], defaults }
}

/// Typing in Size and then clicking another effect's row commits the typing: the page the click leaves is drawn in the
/// click's pass, so its field sees the focus loss.
#[trial]
fn a_list_click_commits_the_typing_of_the_page_it_leaves() {
    let mut style = LayerStyle::default();
    style.effects.push(instance(1, EffectKind::DropShadow, true));
    style.effects.push(instance(2, EffectKind::Satin, true));
    let mut draft = LayerStyleDraft::new(style);
    draft.page = LayerStylePage::Effect(EffectId(1));
    let (mut fixture, host) = rig_of(host(draft, vec![size_spec(EffectKind::DropShadow), spec(EffectKind::Satin)]));
    let field = one(&fixture, Role::SPIN_BUTTON, "Size");
    fixture.click(&field);
    fixture.type_text("42");
    let row = one(&fixture, Role::LIST_ITEM, "Satin");
    fixture.click(&row);
    fixture.idle(3);
    let typed = host.borrow().draft.style.effect(EffectId(1)).and_then(|e| e.params.number("size"));
    assert_eq!(typed, Some(42.0));
    assert_eq!(host.borrow().draft.page, LayerStylePage::Effect(EffectId(2)), "the chosen page shows once the host applied it");
}

/// A spec without fields still shows Make Default and Reset to Default.
#[trial]
fn a_fieldless_spec_shows_make_default_and_reset() {
    let (mut fixture, _host) = rig();
    let row = one(&fixture, Role::LIST_ITEM, "Drop Shadow");
    fixture.click(&row);
    fixture.idle(2);
    assert!(fixture.by_label("Make Default").is_some());
    assert!(fixture.by_label("Reset to Default").is_some());
}

/// `+` on Drop Shadow adds a second instance after the first; `−` shows only with two and removes one; Satin has
/// neither.
#[trial]
fn plus_and_minus_add_and_remove_instances_of_multi_effects() {
    let (mut fixture, host) = rig();
    assert!(fixture.by_label("Remove this Drop Shadow").is_none(), "one instance: no minus");
    let add = fixture.by_label("Add another Drop Shadow").expect("a plus on Drop Shadow");
    fixture.click(&add);
    fixture.idle(2);
    assert_eq!(effects(&host), [(1, EffectKind::DropShadow, true), (2, EffectKind::DropShadow, true)]);

    host.borrow_mut().draft.style.effects.push(instance(3, EffectKind::Satin, true));
    fixture.idle(3);
    assert!(fixture.by_label("Add another Satin").is_none() && fixture.by_label("Remove this Satin").is_none());

    // Two instances are numbered, so no two buttons share a name.
    assert!(fixture.by_label("Add another Drop Shadow").is_none());
    assert!(fixture.by_label("Add another Drop Shadow 2").is_some());
    let remove = fixture.by_label("Remove this Drop Shadow 1").expect("a minus on the first instance");
    fixture.click(&remove);
    fixture.idle(2);
    assert_eq!(effects(&host), [(2, EffectKind::DropShadow, true), (3, EffectKind::Satin, true)]);
}

/// A click on a preset of the Styles page reports `apply_style` with its key and no edit.
#[trial]
fn styles_page_click_reports_apply_style() {
    let (mut fixture, host) = rig();
    let soft = fixture.by_label("Soft").expect("the Soft preset");
    fixture.click(&soft);
    fixture.idle(2);
    assert_eq!(host.borrow().applied, ["Soft"]);
    assert!(host.borrow().edits.is_empty(), "the host applies the preset; the dialog reports no edit");
    assert_eq!(host.borrow().draft.page, LayerStylePage::Styles);
}

/// `set_color(draft, "fx2/color", red)` sets that parameter; an unknown id or a key without a slash changes nothing and
/// returns false.
#[trial]
fn set_color_writes_the_named_parameter_and_ignores_unknown_keys() {
    let mut style = LayerStyle::default();
    style.effects.push(instance(1, EffectKind::DropShadow, true));
    style.effects.push(instance(2, EffectKind::DropShadow, true));
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

/// The Preview check box at the title's end flips the draft's `preview`, which is view state: no edit, and the render
/// hash stays.
#[trial]
fn preview_check_box_flips_the_drafts_preview() {
    let (mut fixture, host) = rig();
    assert!(named(&fixture, Role::CHECK_BOX, "Preview").is_empty(), "the Styles page has no Preview");
    host.borrow_mut().draft.page = LayerStylePage::Effect(EffectId(1));
    fixture.idle(3);
    let hash = host.borrow().draft.render_hash();
    let check = one(&fixture, Role::CHECK_BOX, "Preview");
    fixture.click(&check);
    fixture.idle(2);
    assert!(!host.borrow().draft.preview);
    assert_eq!(host.borrow().draft.render_hash(), hash);
    assert!(host.borrow().edits.is_empty(), "Preview is view state: nothing for the host's history to record");
}

/// A rig showing one effect of the Photoshop tables with its defaults, or `params` over them.
fn page_rig(kind: EffectKind, params: Option<StyleParams>) -> (Fixture, Rc<RefCell<Host>>) {
    let spec = EffectSpec::photoshop(kind);
    let params = params.unwrap_or_else(|| spec.defaults.clone());
    let effects = vec![EffectInstance { id: EffectId(1), kind, on: true, params }];
    let draft = LayerStyleDraft::new(LayerStyle { effects, ..LayerStyle::default() });
    rig_of(host(draft, vec![spec]))
}

/// A click on Drop Shadow's Color swatch reports a colour request keyed "fx1/color" with the current colour and the
/// dialog's picker title.
#[trial]
fn color_field_reports_a_colour_request_with_the_effect_and_key() {
    let (mut fixture, host) = page_rig(EffectKind::DropShadow, None);
    let swatch = one(&fixture, Role::COLOR_WELL, "Color");
    fixture.click(&swatch);
    fixture.idle(2);
    let host = host.borrow();
    let [request] = host.colors.as_slice() else { panic!("one request: {}", host.colors.len()) };
    assert_eq!((request.key.as_str(), request.color), ("fx1/color", Srgba::BLACK));
    assert_eq!(request.title.as_ref().map(|title| title.as_str()), Some("Color Picker (Layer Style Color)"));
}

/// A click on Drop Shadow's Contour thumbnail reports a contour request keyed "fx1/contour" and opens no dialog itself.
#[trial]
fn contour_thumbnail_reports_a_contour_request_with_the_effect_and_key() {
    let (mut fixture, host) = page_rig(EffectKind::DropShadow, None);
    let thumbnail = fixture.by_label("Contour Editor").expect("the contour thumbnail");
    fixture.click(&thumbnail);
    fixture.idle(2);
    assert_eq!(host.borrow().contours, ["fx1/contour"]);
    assert!(named(&fixture, Role::BUTTON, "OK").is_empty(), "the page opens no dialog");
}

fn angle(host: &Rc<RefCell<Host>>) -> Option<f64> {
    host.borrow().draft.style.effect(EffectId(1)).and_then(|e| e.params.number("angle"))
}

/// With Use Global Light on, stepping the Angle dial changes `global_light` and every globally lit instance; with it
/// off, only this instance's angle.
#[trial]
fn use_global_light_check_changes_the_angle_field_meaning() {
    let (mut fixture, host) = page_rig(EffectKind::DropShadow, None);
    let start = angle(&host).expect("the page shows an angle");
    let dial = one(&fixture, Role::SLIDER, "Angle");
    fixture.click(&dial);
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    fixture.idle(2);
    let shared = angle(&host).expect("the instance holds the angle");
    assert!(shared > start, "the step moved the angle");
    assert_eq!(f64::from(host.borrow().draft.style.global_light), shared, "the shared light follows the lit instance");

    let check = one(&fixture, Role::CHECK_BOX, "Use Global Light");
    fixture.click(&check);
    fixture.idle(2);
    let dial = one(&fixture, Role::SLIDER, "Angle");
    fixture.click(&dial);
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    fixture.idle(2);
    assert!(angle(&host) > Some(shared), "the instance's own angle moved on");
    assert_eq!(f64::from(host.borrow().draft.style.global_light), shared, "the shared light stayed");
}

fn blending_rig(blending: Blending, names: &[&str]) -> (Fixture, Rc<RefCell<Host>>) {
    let mut draft = LayerStyleDraft::new(LayerStyle { blending, ..LayerStyle::default() });
    draft.page = LayerStylePage::Blending;
    let mut host = host(draft, Vec::new());
    host.channels = names.iter().map(|name| (*name).to_owned()).collect();
    rig_of(host)
}

/// With channel names C, M, Y, K the page shows four Channels checks; unchecking M writes `channels` as
/// [true, false, true, true].
#[trial]
fn blending_options_channel_checks_follow_the_document_channels() {
    let blending = Blending { channels: vec![true; 4], blend_if: Vec::new(), ..Blending::default() };
    let (mut fixture, host) = blending_rig(blending, &["C", "M", "Y", "K"]);
    for label in ["C", "Y", "K"] {
        one(&fixture, Role::CHECK_BOX, label);
    }
    let magenta = one(&fixture, Role::CHECK_BOX, "M");
    fixture.click(&magenta);
    fixture.idle(2);
    assert_eq!(host.borrow().draft.style.blending.channels, [true, false, true, true]);
}

/// The Blend If select names its first entry Gray in a colour document, then the colour channels.
#[trial]
fn blend_if_select_names_gray_first_in_color_documents() {
    let blending = Blending {
        channels: vec![true; 3],
        blend_if: (0..4).map(|channel| craft_imaging::layer_style::BlendIf { channel, ..Default::default() }).collect(),
        ..Blending::default()
    };
    let (fixture, _host) = blending_rig(blending, &["R", "G", "B"]);
    assert!(fixture.by_label("Gray").is_some(), "the entry 0 of the Blend If select");
}

/// A page whose `opacity` is Text shows one invalid-value line naming Opacity and no fields; the style is reported
/// unchanged.
#[trial]
fn invalid_values_show_one_message_and_are_kept() {
    let mut params = EffectSpec::photoshop(EffectKind::DropShadow).defaults;
    params.set("opacity", StyleValue::Text("high".into()));
    let (fixture, host) = page_rig(EffectKind::DropShadow, Some(params.clone()));
    assert!(fixture.texts().iter().any(|text| text.text.contains("Opacity has a value")), "the line names the field");
    assert!(fixture.by_role(Role::SLIDER).is_empty() && named(&fixture, Role::CHECK_BOX, "Use Global Light").is_empty());
    assert!(host.borrow().edits.is_empty());
    assert_eq!(host.borrow().draft.style.effect(EffectId(1)).map(|e| &e.params), Some(&params));
}

/// A page shows the spec's defaults for keys the instance lacks without writing them; editing one field writes only
/// that key.
#[trial]
fn page_shows_defaults_under_the_values_and_writes_only_edits() {
    let (mut fixture, host) = page_rig(EffectKind::DropShadow, Some(StyleParams::default()));
    assert!(host.borrow().edits.is_empty());
    let make = fixture.by_label("Make Default").expect("Make Default");
    fixture.click(&make);
    fixture.idle(2);
    assert_eq!(host.borrow().made.len(), 1);
    let defaults = EffectSpec::photoshop(EffectKind::DropShadow).defaults;
    assert_eq!(host.borrow().made[0].number("opacity"), defaults.number("opacity"));

    let distance = one(&fixture, Role::SLIDER, "Distance");
    fixture.click(&distance);
    fixture.key(Key::ArrowRight, Modifiers::NONE);
    fixture.idle(2);
    let host = host.borrow();
    let [edit] = host.edits.as_slice() else { panic!("one edit: {}", host.edits.len()) };
    let written = edit.value().and_then(|style| style.effect(EffectId(1))).map(|e| e.params.clone()).unwrap_or_default();
    assert!(written.number("distance") > defaults.number("distance"), "the arrow key steps the distance up from its default");
    assert_eq!(written.number("opacity"), None, "only the edited key is written");
}

/// A Pattern page whose value is empty shows the first pattern and reports it as a fill-default hint, never as an edit.
#[trial]
fn empty_pattern_reports_a_fill_default_and_no_edit() {
    let mut params = EffectSpec::photoshop(EffectKind::PatternOverlay).defaults;
    params.set("pattern", StyleValue::Text(String::new()));
    let (mut fixture, host) = page_rig(EffectKind::PatternOverlay, Some(params));
    host.borrow_mut().patterns = vec![PresetRow::item("p1".to_owned(), "Bubbles", 0), PresetRow::item("p2".to_owned(), "Wood", 0)];
    fixture.idle(3);
    assert!(host.borrow().edits.is_empty(), "no edit: {}", host.borrow().edits.len());
    let filled = host.borrow().draft.style.effect(EffectId(1)).and_then(|e| e.params.get("pattern").cloned());
    assert_eq!(filled, Some(StyleValue::Text("p1".into())), "the host wrote the fill-default hint into its draft");
}
