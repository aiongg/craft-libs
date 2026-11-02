//! The parameter dialog in a harness.

use egui::accesskit::Role;
use egui::load::SizedTexture;
use egui::os::OperatingSystem;
use egui::{Event, Key, Modifiers, TextureId, vec2};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_kittest::{Harness, HarnessBuilder};

use super::{CommandForm, DialogFields, DialogHost, DialogResult, FieldValue, Param, ParamDialog, ParamKind, ParamResult, PathKind, PathRequest};
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

// `translated` is the lookup the dialogs use for runtime labels, public for the hosts' own.
#[test]
fn translated_looks_a_runtime_label_up_in_the_hosts_catalog() {
    let (i18n, problems) = craft_i18n::I18n::new(&[("de", "\tZoom\tVergrößern\n", "")], &["de"]);
    assert!(problems.is_empty(), "{problems:?}");
    let de = i18n.from_code("de").expect("de");
    let ctx = egui::Context::default();
    assert_eq!(super::translated(&ctx, "Zoom"), "Zoom", "no Host: the text");
    craft_host::Host::builder(craft_host::AppInfo { id: "test", name: "Test", version: "1.0" })
        .i18n(std::sync::Arc::new(i18n))
        .lang(de)
        .install(&ctx);
    assert_eq!(super::translated(&ctx, "Zoom"), "Vergrößern");
    assert_eq!(super::translated(&ctx, "Not in the catalog"), "Not in the catalog");
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
    assert_eq!(
        harness.state_mut().get_mut::<CommandForm>().unwrap().pick_color().map(|request| (request.key, request.title, request.color)),
        Some(("b".to_owned(), Some("Over".to_owned()), craft_theme::Srgba::BLACK))
    );
    assert_eq!(harness.state_mut().get_mut::<ParamDialog>().unwrap().pick_color(), None);
}

// The picker's answer is written to the colour parameter it names, and only that one.
#[test]
fn a_colour_answer_sets_the_named_colour_parameter() {
    let params = || {
        vec![
            Param::new("tint", "Tint", ParamKind::Color { default: None }),
            Param::new("n", "N", ParamKind::Int { min: None, max: None, default: 1 }),
        ]
    };
    let mut dialog = ParamDialog::new("k", "K", params());
    dialog.set_color("tint", craft_theme::Srgba::rgb(0.5, 0.0, 1.0));
    dialog.set_color("n", craft_theme::Srgba::rgb(1.0, 1.0, 1.0));
    dialog.set_color("missing", craft_theme::Srgba::rgb(1.0, 1.0, 1.0));
    assert_eq!(dialog.fields().get("tint"), Some(&FieldValue::Color([0.5, 0.0, 1.0])));
    assert_eq!(dialog.fields().get("n"), None);
    let mut form = CommandForm::new("k", "K", params());
    form.set_color("tint", craft_theme::Srgba::rgb(0.0, 1.0, 0.0));
    assert_eq!(form.fields().get("tint"), Some(&FieldValue::Color([0.0, 1.0, 0.0])));
}

fn open_dialog<'a>(harness: &'a mut Harness<'static, App>) -> Option<&'a mut ParamDialog> {
    harness.state_mut().host.get_mut::<ParamDialog>()
}

// Alt+Reset sets the host's target (schema defaults), not the values from when the dialog opened.
#[test]
fn alt_reset_sets_the_hosts_target() {
    let given = DialogFields::from_pairs([("radius", FieldValue::Number(12.0))]);
    let target = DialogFields::from_pairs([("radius", FieldValue::Number(3.0))]);
    let mut harness = harness(ParamDialog::new("blur", "Blur", params()).values(given).reset_to(target));
    harness.run();
    harness.event(Event::ModifiersChanged(Modifiers::ALT));
    harness.run();
    harness.get_by_label("Reset").focus();
    harness.run();
    harness.key_press_modifiers(Modifiers::ALT, Key::Space);
    harness.run();
    assert_eq!(radius(&harness), Some(3.0));
    assert!(harness.state().answers.is_empty());
}

// A host that cannot yet say OK is valid disables it; Enter does nothing until it enables it.
#[test]
fn ok_can_be_disabled_by_the_host() {
    let mut harness = harness(ParamDialog::new("blur", "Blur", params()).ok_enabled(false));
    harness.run();
    assert!(harness.get_by_label("OK").accesskit_node().is_disabled());
    harness.key_press(Key::Enter);
    harness.run();
    assert!(harness.state().answers.is_empty());
    open_dialog(&mut harness).expect("open").set_ok_enabled(true);
    harness.run();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    harness.run();
    assert!(matches!(harness.state().answers[..], [DialogResult::Ok(_)]));
}

// A row with `enabled_when` is disabled until the named value matches.
#[test]
fn a_row_is_enabled_only_while_its_condition_holds() {
    let rows = vec![
        Param::new("localized", "Localized:", ParamKind::Bool { default: false }),
        Param::new("range", "Range:", ParamKind::Int { min: Some(0), max: Some(100), default: 40 }).enabled_when("localized", true),
    ];
    let mut harness = harness(ParamDialog::new("c", "C", rows));
    harness.run();
    assert!(harness.get_by_role_and_label(Role::SpinButton, "Range:").accesskit_node().is_disabled());
    let label = harness.get_by_role_and_label(Role::Label, "Range:");
    assert!(label.accesskit_node().is_disabled(), "the row label is disabled too");
    harness.get_by_label("Localized").click();
    harness.run();
    harness.run();
    assert!(!harness.get_by_role_and_label(Role::SpinButton, "Range:").accesskit_node().is_disabled());
}

// A custom row draws the host's control and writes the dialog's values.
#[test]
fn a_custom_row_writes_the_fields() {
    let rows = vec![Param::custom("center", "Center:", |ui, fields, _| {
        let response = ui.button("Set center");
        if response.clicked() {
            fields.set("center", FieldValue::Point([25.0, 75.0]));
        }
        response
    })];
    let mut harness = harness(ParamDialog::new("zoom", "Zoom", rows));
    harness.run();
    harness.get_by_label("Center:").click();
    harness.run();
    assert_eq!(open_dialog(&mut harness).expect("open").fields().point("center"), Some([25.0, 75.0]));
}

// A Browse button after a text row reports its key and kind once; the host writes the path.
#[test]
fn a_browse_button_reports_the_row() {
    let rows = vec![Param::new("folder", "Folder:", ParamKind::Text { default: String::new() }).browse(PathKind::Folder)];
    let mut harness = harness(ParamDialog::new("batch", "Batch", rows));
    harness.run();
    harness.get_by_label("Browse…").click();
    harness.run();
    assert_eq!(open_dialog(&mut harness).expect("open").pick_path(), Some(PathRequest { key: "folder".into(), kind: PathKind::Folder }));
    assert_eq!(open_dialog(&mut harness).expect("open").pick_path(), None);
}

// The Custom check box of an optional number sets or removes the key; the number keeps its value
// across an off/on.
#[test]
fn an_optional_number_is_absent_until_custom_is_checked() {
    let rows = vec![Param::new("quality", "Quality:", ParamKind::OptionalNumber { min: 0.0, max: 12.0, default: None, initial: 8.0, decimals: 2 })];
    let mut harness = harness(ParamDialog::new("batch", "Batch", rows));
    harness.run();
    assert_eq!(open_dialog(&mut harness).expect("open").fields().number("quality"), None);
    assert!(harness.get_by_role_and_label(Role::SpinButton, "Quality:").accesskit_node().is_disabled());
    harness.get_by_label("Custom").click();
    harness.run();
    assert_eq!(open_dialog(&mut harness).expect("open").fields().number("quality"), Some(8.0));
    harness.get_by_label("Custom").click();
    harness.run();
    assert_eq!(open_dialog(&mut harness).expect("open").fields().number("quality"), None);
}

// A hint shows in an empty text field only; a tooltip rides on the row and is described to
// AccessKit on any row kind.
#[test]
fn a_text_row_shows_its_hint_while_empty_and_the_row_has_a_tooltip() {
    let rows = vec![
        Param::new("naming", "File Naming:", ParamKind::Text { default: String::new() }).hint("{name}").tooltip("{name} · {serial} · {date} (UTC)"),
        Param::new("size", "Size:", ParamKind::Int { min: Some(1), max: None, default: 4 }).tooltip("Pixels"),
    ];
    let mut harness = harness(ParamDialog::new("batch", "Batch", rows));
    harness.run();
    let field = |harness: &Harness<'_, App>| {
        harness.get_by_role_and_label(Role::TextInput, "File Naming:").accesskit_node().data().placeholder().map(str::to_owned)
    };
    assert_eq!(field(&harness).as_deref(), Some("{name}"));
    harness.get_by_role_and_label(Role::TextInput, "File Naming:").focus();
    harness.run();
    harness.event(Event::Text("x".to_owned()));
    harness.run();
    harness.get_by_role_and_label(Role::TextInput, "File Naming:").hover();
    for _ in 0..30 {
        harness.step();
    }
    harness.get_by_label("{name} · {serial} · {date} (UTC)");
}

// An optional number with 0 decimals shows and keeps whole numbers.
#[test]
fn an_optional_number_with_no_decimals_shows_an_integer() {
    let rows = vec![
        Param::new("quality", "Quality:", ParamKind::OptionalNumber { min: 0.0, max: 12.0, default: Some(8.0), initial: 8.0, decimals: 2 })
            .decimals(0),
    ];
    let mut harness = harness(ParamDialog::new("batch", "Batch", rows));
    harness.run();
    harness.get_by_role_and_label(Role::SpinButton, "Quality:").focus();
    harness.run();
    harness.event(Event::Text("5.7".to_owned()));
    harness.key_press(Key::Tab);
    harness.run();
    let shown = harness.get_by_role_and_label(Role::SpinButton, "Quality:").value();
    assert_eq!(shown.as_deref(), Some("6"), "{shown:?}");
}

// The footer hook draws before the buttons; `close_label(None)` removes the close box.
#[test]
fn the_footer_hook_and_the_close_box_follow_the_options() {
    let mut harness = harness(ParamDialog::new("table", "Table", params()).footer_extra(|ui| {
        let _ = ui.button("Load…");
    }));
    harness.run();
    harness.get_by_label("Load…");
    harness.get_by_label("Close");
    let mut harness = self::harness(ParamDialog::new("table", "Table", params()).close_label(None));
    harness.run();
    assert!(harness.query_by_label("Close").is_none());
}

// `confirm` stores the remembered values as OK does.
#[test]
fn confirm_stores_the_remembered_values() {
    let mut harness = harness(ParamDialog::new("blur", "Blur", params()).remember("filter.blur"));
    harness.run();
    open_dialog(&mut harness).expect("open").fields_mut().set("radius", FieldValue::Number(9.0));
    let ctx = harness.ctx.clone();
    assert!(harness.state_mut().host.confirm::<ParamDialog>(&ctx));
    harness.state_mut().host.open(ParamDialog::new("blur", "Blur", params()).remember("filter.blur"));
    harness.run();
    assert_eq!(radius(&harness), Some(9.0));
}

// A bare number row has no slider, accepts any value on an open side and clamps to a bound.
#[test]
fn a_number_row_is_unbounded_on_open_sides_and_clamped_on_bounded_ones() {
    let rows = vec![Param::number("offset", "Offset:", 0.0, 0.5, 2), Param::number("count", "Count:", 3.0, 1.0, 0).bounds(Some(1.0), Some(10.0))];
    let mut harness = harness(ParamDialog::new("shift", "Shift", rows));
    harness.run();
    assert!(harness.query_by_role(Role::Slider).is_none());
    harness.get_by_role_and_label(Role::SpinButton, "Offset:").focus();
    harness.run();
    harness.event(Event::Text("-5000.25".to_owned()));
    harness.key_press(Key::Tab);
    harness.run();
    harness.get_by_role_and_label(Role::SpinButton, "Count:").focus();
    harness.run();
    harness.event(Event::Text("99".to_owned()));
    harness.key_press(Key::Tab);
    harness.run();
    let fields = open_dialog(&mut harness).expect("open").fields().clone();
    assert_eq!((fields.number("offset"), fields.number("count")), (Some(-5000.25), Some(10.0)));
}

// `ParamRows` draws one form row per parameter outside a dialog: a range as slider and field, a
// choice as a select, a bool as a check box, a colour as a swatch whose press comes back as
// `ParamRowsResponse::color`; `changed` reports an edit.
#[test]
fn param_rows_draw_range_choice_bool_and_colour() {
    use super::ParamRows;
    struct Rows {
        params: Vec<Param>,
        fields: DialogFields,
        changed: bool,
        color: Option<String>,
    }
    let mut params = params();
    params.push(Param::new(
        "method",
        "Method:",
        ParamKind::Choice {
            options: vec![("spin".to_owned(), "Spin".to_owned()), ("zoom".to_owned(), "Zoom".to_owned())],
            default: Some("zoom".to_owned()),
        },
    ));
    let mut harness = HarnessBuilder::default()
        .with_size(vec2(400.0, 400.0))
        .with_os(OperatingSystem::Nix)
        .with_context_setup(|ctx| install(ctx, &Base::suite_default()))
        .build_ui_state(
            |ui, rows: &mut Rows| {
                let shown = ParamRows::new("rows", &rows.params).show(ui, &mut rows.fields);
                rows.changed = shown.changed;
                rows.color = shown.color.map(|request| request.key);
            },
            Rows { params, fields: DialogFields::new(), changed: false, color: None },
        );
    harness.run();
    let state = harness.state();
    assert!(!state.changed, "filling defaults is not an edit");
    assert_eq!((state.fields.number("radius"), state.fields.bool("monochromatic")), (Some(1.0), Some(false)));
    assert_eq!(state.fields.text("method"), Some("zoom"));
    harness.get_by_role_and_label(Role::SpinButton, "Radius:");
    harness.get_by_role_and_label(Role::CheckBox, "Monochromatic");
    harness.get_by_label("Method:");

    harness.get_by_role_and_label(Role::CheckBox, "Monochromatic").click();
    harness.run_steps(1);
    assert!(harness.state().changed);
    assert_eq!(harness.state().fields.bool("monochromatic"), Some(true));
    harness.run_steps(2);
    assert!(!harness.state().changed);

    harness.get_by_label("Tint:").click();
    harness.run_steps(1);
    assert_eq!(harness.state().color.as_deref(), Some("tint"));
}

// A custom row asks the host for a colour under a key with no row; the answer lands under it and
// is read back as the dialog's value.
#[test]
fn a_custom_row_asks_for_a_colour_under_its_own_key() {
    let rows = vec![Param::custom("result", "Result:", |ui, _, custom| {
        let response = ui.button("Pick color…");
        if response.clicked() {
            custom.pick_color("resultColor");
        }
        response
    })];
    let mut harness = harness(ParamDialog::new("replace", "Replace Color", rows));
    harness.run();
    harness.get_by_label("Result:").click();
    harness.run();
    let dialog = open_dialog(&mut harness).expect("open");
    let request = dialog.pick_color().expect("the press is reported");
    assert_eq!(request.key, "resultColor");
    assert_eq!(request.title.as_deref(), Some("Result"));
    assert_eq!(dialog.pick_color(), None, "once");
    dialog.set_color("resultColor", craft_theme::Srgba::from_rgb_array([0.0, 1.0, 0.0]));
    assert_eq!(dialog.fields().color("resultColor"), Some([0.0, 1.0, 0.0]));
}

// Browse on an open-file row reports the extensions the host's file dialog filters by.
#[test]
fn an_open_file_row_reports_its_extensions() {
    let rows = vec![Param::new("lut", "Look-up table:", ParamKind::Text { default: String::new() }).browse(PathKind::open_file(&["cube", "3dl"]))];
    let mut harness = harness(ParamDialog::new("lookup", "Lookup", rows));
    harness.run();
    harness.get_by_label("Browse…").click();
    harness.run();
    let request = open_dialog(&mut harness).expect("open").pick_path().expect("reported");
    assert_eq!(request.kind, PathKind::OpenFile { extensions: vec!["cube".into(), "3dl".into()] });
}

// Two dialogs of one command, told apart by `instance`, are open together and a swatch press in
// one (the later, drawn on top) is reported to that one only.
#[test]
fn two_dialogs_of_one_command_keep_separate_state() {
    use super::{DialogStack, StackDialog as _};
    let make = |label: &str, instance: u32| {
        let tint = Param::new("tint", label, ParamKind::Color { default: Some([1.0, 0.0, 0.0]) });
        ParamDialog::new("tint-command", label.trim_end_matches(" tint:"), vec![tint]).instance(instance)
    };
    let mut harness = HarnessBuilder::default()
        .with_size(vec2(900.0, 600.0))
        .with_os(OperatingSystem::Nix)
        .with_context_setup(|ctx| {
            install(ctx, &Base::suite_default());
            craft_host::Host::builder(craft_host::AppInfo { id: "test", name: "Test", version: "1.0" }).install(ctx);
        })
        .build_ui_state(
            |ui, dialogs: &mut Vec<ParamDialog>| {
                let stack = DialogStack::default();
                for dialog in dialogs {
                    dialog.show(ui.ctx(), &stack);
                }
            },
            vec![make("Beta tint:", 2), make("Alpha tint:", 1)],
        );
    harness.run();
    assert_eq!(harness.query_all_by_role(Role::Dialog).count(), 2);
    harness.state_mut()[1].fields_mut().set("tint", FieldValue::Color([0.0, 0.0, 1.0]));
    harness.get_by_label("Alpha tint:").click();
    harness.run();
    let [beta, alpha] = &mut harness.state_mut()[..] else { unreachable!() };
    assert_eq!(alpha.pick_color().map(|r| r.key), Some("tint".to_owned()));
    assert_eq!(beta.pick_color(), None);
    assert_eq!(beta.fields().color("tint"), Some([1.0, 0.0, 0.0]));
}
