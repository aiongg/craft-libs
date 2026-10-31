//! The parameter dialog in a harness.

use egui::accesskit::Role;
use egui::load::SizedTexture;
use egui::os::OperatingSystem;
use egui::{Event, Key, Modifiers, TextureId, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::{CommandForm, DialogFields, DialogHost, DialogResult, FieldValue, Param, ParamDialog, ParamKind, ParamResult};
use crate::controls::SliderScale;
use crate::localized::Unit;
use crate::theme::{Base, install};

struct App {
    host: DialogHost,
    answers: Vec<DialogResult<ParamResult>>,
}

fn params() -> Vec<Param> {
    vec![
        Param::new("radius", "Radius:", ParamKind::Range { min: 0.0, max: 100.0, default: 1.0, scale: SliderScale::Linear })
            .unit(Unit::new("{n} px")),
        Param::new("monochromatic", "Monochromatic:", ParamKind::Bool { default: false }),
        Param::new("tint", "Tint:", ParamKind::Color { default: Some([1.0, 0.0, 0.0]) }),
    ]
}

fn harness(dialog: ParamDialog) -> Harness<'static, App> {
    let mut host = DialogHost::default();
    host.open(dialog);
    HarnessBuilder::default()
        .with_size(vec2(700.0, 600.0))
        .with_os(OperatingSystem::Nix)
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            craft_host::Host::builder(craft_host::AppInfo { id: "test", name: "Test", version: "1.0" }).install(ctx);
        })
        .build_ui_state(
            |ui, app: &mut App| {
                app.host.show(ui.ctx());
                if let Some(answer) = app.host.take::<ParamDialog>() {
                    app.answers.push(answer);
                }
            },
            App { host, answers: Vec::new() },
        )
}

fn radius(harness: &Harness<'_, App>) -> Option<f64> {
    harness.state().host.get::<ParamDialog>()?.fields().number("radius")
}

// PhotoCraft's notation parses into the kinds, bounds, defaults and labels the dialog shows.
#[test]
fn the_registry_notation_parses_into_params() {
    let params = Param::parse_list(
        r#"{"radius":0.1..1000=1,"method":"spin|zoom"="zoom","monochromatic":bool,"seed":u32=0,"cellSize":px=4,"tint":color=#ff8000,"kernel":int[25],"layer":layer id,"curve":json,"note":text} -> {"x":1}"#,
    );
    let keys: Vec<_> = params.iter().map(|p| p.key.as_str()).collect();
    assert_eq!(keys, ["radius", "method", "monochromatic", "seed", "cellSize", "tint", "kernel", "note"]);
    assert_eq!(params[0].label, "Radius");
    assert_eq!(params[4].label, "Cell Size");

    assert_eq!(params[0].kind, ParamKind::Range { min: 0.1, max: 1000.0, default: 1.0, scale: SliderScale::Log });
    let options = vec![("spin".to_owned(), "Spin".to_owned()), ("zoom".to_owned(), "Zoom".to_owned())];
    assert_eq!(params[1].kind, ParamKind::Choice { options, default: Some("zoom".to_owned()) });
    assert_eq!(params[2].kind, ParamKind::Bool { default: false });
    assert_eq!(params[3].kind, ParamKind::Int { min: None, max: None, default: 0 });
    assert_eq!(params[5].kind, ParamKind::Color { default: Some([1.0, 128.0 / 255.0, 0.0]) });
    assert_eq!(params[6].kind, ParamKind::Grid { cells: 25, default: 0 });

    let kernel = params[6].kind.default_value().expect("a grid has a default");
    assert_eq!(kernel, FieldValue::Grid((0..25).map(|i| i64::from(i == 12)).collect()));
    assert!(params[6].kind.fits(&kernel));
    assert!(!params[0].kind.fits(&FieldValue::Number(5000.0)));
    assert!(!params[1].kind.fits(&FieldValue::Text("wobble".into())));
    assert_eq!(params[5].kind.default_value(), Some(FieldValue::Color([1.0, 128.0 / 255.0, 0.0])));
}

// The first field opens selected; typing replaces its value and Enter answers OK with it.
#[test]
fn typing_into_the_first_field_and_enter_answers_ok() {
    let mut harness = harness(ParamDialog::new("blur", "Blur", params()));
    harness.run();
    assert!(harness.get_by_role_and_label(Role::SpinButton, "Radius:").accesskit_node().is_focused());
    harness.event(Event::Text("7".to_owned()));
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    harness.run();
    let [DialogResult::Ok(ParamResult { key, fields })] = &harness.state().answers[..] else { panic!("one OK, got {:?}", harness.state().answers) };
    assert_eq!(key, "blur");
    assert_eq!(fields.number("radius"), Some(7.0));
    assert_eq!(fields.bool("monochromatic"), Some(false));
    assert_eq!(fields.color("tint"), Some([1.0, 0.0, 0.0]), "defaults fill what the host gave no value for");
}

// Alt turns Cancel into Reset; a click restores the opening values and the dialog stays open.
#[test]
fn alt_cancel_resets_to_the_opening_values() {
    let given = DialogFields::from_pairs([("radius", FieldValue::Number(12.0))]);
    let mut harness = harness(ParamDialog::new("blur", "Blur", params()).values(given));
    harness.run();
    harness.event(Event::Text("40".to_owned()));
    harness.run();
    harness.get_by_label("Monochromatic").click();
    harness.run();
    assert_eq!(radius(&harness), Some(40.0));
    harness.event(Event::ModifiersChanged(Modifiers::ALT));
    harness.run();
    harness.get_by_label("Reset").focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::ALT, Key::Space);
    harness.run();
    let fields = harness.state().host.get::<ParamDialog>().expect("still open").fields();
    assert_eq!((fields.number("radius"), fields.bool("monochromatic")), (Some(12.0), Some(false)));
    assert!(harness.state().answers.is_empty());
}

// With `remember`, OK stores the values and the next open starts from them; Cancel does not store.
#[test]
fn ok_remembers_the_values_and_cancel_does_not() {
    let dialog = || {
        let mut all = params();
        all.push(Param::new("note", "Note:", ParamKind::Text { default: String::new() }));
        ParamDialog::new("blur", "Blur", all).remember("filter.blur")
    };
    let mut harness = harness(dialog());
    harness.run();
    harness.event(Event::Text("7".to_owned()));
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    harness.run();
    assert!(matches!(harness.state().answers[..], [DialogResult::Cancel]));
    harness.state_mut().host.open(dialog());
    harness.run();
    assert_eq!(radius(&harness), Some(1.0), "Cancel stored nothing");

    harness.event(Event::Text("7".to_owned()));
    harness.run();
    harness.get_by_label("Monochromatic").click();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    harness.run();
    assert!(matches!(harness.state().answers[1], DialogResult::Ok(_)));
    harness.state_mut().host.open(dialog());
    harness.run();
    let fields = harness.state().host.get::<ParamDialog>().expect("reopened").fields();
    assert_eq!((fields.number("radius"), fields.bool("monochromatic")), (Some(7.0), Some(true)));
    assert_eq!(fields.color("tint"), Some([1.0, 0.0, 0.0]), "colours are not remembered");
}

// Preview off, or the label held, makes `preview_on` false; the host's image shows in the pane
// otherwise.
#[test]
fn the_preview_check_box_and_hold_to_compare_drive_preview_on() {
    let mut harness = harness(ParamDialog::new("blur", "Blur", params()).preview(true));
    harness.run();
    let preview_on = |harness: &Harness<'_, App>| harness.state().host.get::<ParamDialog>().is_some_and(ParamDialog::preview_on);
    assert!(preview_on(&harness));
    harness
        .state_mut()
        .host
        .get_mut::<ParamDialog>()
        .expect("open")
        .set_preview_image(Some(SizedTexture::new(TextureId::default(), vec2(64.0, 48.0))));
    harness.run();

    let at = harness.get_by_label("Preview").rect().center();
    harness.hover_at(at);
    harness.run();
    let button = |pressed| Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
    harness.event(button(true));
    harness.run();
    assert!(!preview_on(&harness), "label held");
    harness.event(button(false));
    harness.run();
    assert!(!preview_on(&harness), "the click turned Preview off");
    harness.get_by_label("Preview").click();
    harness.run();
    assert!(preview_on(&harness));
}

// Tab walks the number and text fields, then the footer buttons, and never a slider, check box or
// colour swatch (the dialog's field rule).
#[test]
fn tab_visits_the_fields_and_buttons_only() {
    let mut harness = harness(ParamDialog::new("blur", "Blur", params()));
    harness.run();
    let mut seen = Vec::new();
    for _ in 0..6 {
        harness.key_press(Key::Tab);
        harness.run();
        seen.push(harness.query_by(|n| n.is_focused()).and_then(|n| n.accesskit_node().label()).unwrap_or_default());
    }
    assert!(seen.iter().all(|label| ["Radius:", "OK", "Cancel"].contains(&label.as_str())), "{seen:?}");
    assert!(seen.iter().any(|label| label == "Cancel"), "{seen:?}");
}

// Parameter and option labels (English fallbacks from `parse_list`) show in the Host's language.
#[test]
fn labels_and_option_labels_show_through_the_hosts_catalog() {
    let (i18n, problems) = craft_i18n::I18n::new(&[("de", "\tRadius\tRadius-de\n\tZoom\tVergrößern\n", "")], &["de"]);
    assert!(problems.is_empty(), "{problems:?}");
    let de = i18n.from_code("de").expect("de");
    let i18n = std::sync::Arc::new(i18n);
    let mut host = DialogHost::default();
    host.open(ParamDialog::new("blur", "Blur", Param::parse_list(r#"{"radius":0..100=1,"method":"spin|zoom"="zoom"}"#)));
    let mut harness = HarnessBuilder::default()
        .with_size(vec2(700.0, 500.0))
        .with_context_setup(move |ctx| {
            install(ctx, &Base::suite_default());
            craft_host::Host::builder(craft_host::AppInfo { id: "test", name: "Test", version: "1.0" }).i18n(i18n.clone()).lang(de).install(ctx);
        })
        .build_ui_state(|ui, host: &mut DialogHost| host.show(ui.ctx()), host);
    harness.run();
    harness.get_by_role_and_label(Role::SpinButton, "Radius-de:");
    let select = harness.get_by_label("Method:");
    assert_eq!(select.accesskit_node().value().as_deref(), Some("Vergrößern"));
}

// A parsed `color=#808080` default shows as that hex: colour values are sRGB in 0..=1, as
// PhotoCraft keeps them.
#[test]
fn a_parsed_hex_default_shows_as_that_hex() {
    let params = Param::parse_list(r##"{"tint":color=#808080}"##);
    let mut harness = harness(ParamDialog::new("tint", "Tint", params));
    harness.run();
    let swatch = harness.get_by_label("Tint:");
    assert_eq!(swatch.accesskit_node().value().as_deref().map(str::to_lowercase).as_deref(), Some("#808080"));
}

// Labels of a PhotoCraft filter spec equal PhotoCraft's `source_label` (Title Case, no colon,
// fixed names); options too; the dialog adds the colon after translating.
#[test]
fn labels_follow_photocrafts_source_label() {
    let params = Param::parse_list(r#"{"cellSize":px=4,"localized":bool,"bpc":bool,"iPhone 14":bool,"intent":"perceptual|relativeColorimetric"}"#);
    let labels: Vec<_> = params.iter().map(|p| p.label.as_str()).collect();
    assert_eq!(labels, ["Cell Size", "Localized Color Clusters", "Black Point Compensation", "iPhone 14", "Intent"]);
    let ParamKind::Choice { options, .. } = &params[4].kind else { panic!("choice") };
    assert_eq!(options[1].1, "Relative Colorimetric");

    let mut harness = harness(ParamDialog::new("c", "C", params));
    harness.run();
    harness.get_by_label("Cell Size:");
    harness.get_by_label("Intent:");
}

// The log slider starts above a max/min ratio of 500 (PhotoCraft `uses_logarithmic_slider`).
#[test]
fn log_slider_threshold_is_500() {
    let scale = |spec: &str| match Param::parse_list(spec)[0].kind {
        ParamKind::Range { scale, .. } => scale,
        _ => panic!("range"),
    };
    assert_eq!(scale(r#"{"r":1..500=1}"#), SliderScale::Linear);
    assert_eq!(scale(r#"{"r":1..501=1}"#), SliderScale::Log);
    assert_eq!(scale(r#"{"r":0..1000=1}"#), SliderScale::Linear);
}

// A kernel that is not square keeps every cell: PhotoCraft wraps `round(sqrt(n))` per row.
#[test]
fn a_non_square_kernel_keeps_its_cells() {
    let params = Param::parse_list(r#"{"k":int[10]}"#);
    assert_eq!(params[0].kind, ParamKind::Grid { cells: 10, default: 0 });
    assert_eq!(ParamKind::grid_side(10), 3);
    assert_eq!(ParamKind::grid_side(25), 5);
    let FieldValue::Grid(cells) = params[0].kind.default_value().unwrap() else { panic!("grid") };
    assert_eq!(cells, [0, 0, 0, 0, 0, 1, 0, 0, 0, 0]);
    assert!(params[0].kind.fits(&FieldValue::Grid(vec![0; 10])));
    let mut harness = harness(ParamDialog::new("k", "K", params));
    harness.run();
    assert_eq!(harness.query_all_by_role(Role::SpinButton).count(), 10);
}

// A swatch press belongs to the dialog that drew it: a form over a parent never takes the
// parent's press, nor the parent the form's.
#[test]
fn a_swatch_press_goes_to_its_own_dialog() {
    let mut host = DialogHost::default();
    let parent = host.open(ParamDialog::new("parent", "Parent", vec![Param::new("a", "Under", ParamKind::Color { default: None })]));
    host.open_child(CommandForm::new("child", "Child", vec![Param::new("b", "Over", ParamKind::Color { default: None })]), parent);
    let mut harness = HarnessBuilder::default()
        .with_size(vec2(900.0, 700.0))
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(|ui, host: &mut DialogHost| host.show(ui.ctx()), host);
    harness.run();
    harness.get_by_label("Over:").click();
    harness.run();
    assert_eq!(harness.state_mut().get_mut::<CommandForm>().unwrap().pick_color().as_deref(), Some("b"));
    assert_eq!(harness.state_mut().get_mut::<ParamDialog>().unwrap().pick_color(), None);
}
