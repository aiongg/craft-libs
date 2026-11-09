//! The parameter model: the spec-string parser, kinds, defaults and fit rules, row conditions, the value store, and
//! the rows' use of a bound value.

use craft_ui::contract::Bind;
use craft_ui::patterns::form_dialogs::{DialogFields, FieldValue, NumberUnit, Param, ParamDialog, ParamKind, ParamResult, ParamRows, open_fields};
use craft_ui::runtime::env::UnitKind;
use craft_ui::testing::{Fixture, trial};
use craft_ui::value_drag::SliderScale;
use egui::{Key, Modifiers};
use std::cell::RefCell;
use std::rc::Rc;

const SPEC: &str = r#"{"radius":0.1..1000=1,"method":"spin|zoom"="zoom","monochromatic":bool,"seed":u32=0,"cellSize":px=4,"tint":color=#ff8000,"kernel":int[25],"layer":layer id,"curve":json,"note":text} -> {"x":1}"#;
const KERNEL_CELLS: usize = 25;
const LOG_LIMIT: f64 = 500.0;
const RADIUS_DEFAULT: f64 = 1.0;

/// The registry notation gives the kinds, bounds, defaults and labels the rows show; entries the rows do not edit are
/// left out.
#[trial]
fn the_registry_notation_parses_into_params() {
    let params = Param::parse_list(SPEC);
    let keys: Vec<_> = params.iter().map(|p| p.key.as_str()).collect();
    assert_eq!(keys, ["radius", "method", "monochromatic", "seed", "cellSize", "tint", "kernel", "note"]);
    assert_eq!(params[0].label.as_str(), "Radius");
    assert_eq!(params[4].label.as_str(), "Cell Size");

    assert_eq!(params[0].kind, ParamKind::range(0.1, 1000.0, 1.0, SliderScale::Log));
    let ParamKind::Choice { options, default, .. } = &params[1].kind else { panic!("a choice") };
    assert_eq!(options.iter().map(|(id, label)| (id.as_str(), label.as_str())).collect::<Vec<_>>(), [("spin", "Spin"), ("zoom", "Zoom")]);
    assert_eq!(default.as_deref(), Some("zoom"));
    assert_eq!(params[3].kind, ParamKind::int(None, None, 0));
    assert_eq!(params[5].kind.default_value(), Some(FieldValue::Color([1.0, 128.0 / 255.0, 0.0])));

    let kernel = params[6].kind.default_value().expect("a grid has a default");
    assert_eq!(kernel, FieldValue::Grid((0..KERNEL_CELLS).map(|i| i64::from(i == KERNEL_CELLS / 2)).collect()));
    assert!(params[6].kind.fits(&kernel));
    assert!(!params[0].kind.fits(&FieldValue::Number(5000.0)));
    assert!(!params[1].kind.fits(&FieldValue::Text("wobble".into())));
}

/// Labels read camelCase as Title Case with two fixed names; a key with a space is already text.
#[trial]
fn labels_follow_the_source_label_rule() {
    let params = Param::parse_list(r#"{"cellSize":px=4,"localized":bool,"bpc":bool,"iPhone 14":bool,"intent":"perceptual|relativeColorimetric"}"#);
    let labels: Vec<_> = params.iter().map(|p| p.label.as_str()).collect();
    assert_eq!(labels, ["Cell Size", "Localized Color Clusters", "Black Point Compensation", "iPhone 14", "Intent"]);
    let ParamKind::Choice { options, .. } = &params[4].kind else { panic!("a choice") };
    assert_eq!(options[1].1.as_str(), "Relative Colorimetric");
}

/// A range gets a log slider only when its max is over 500 times its min.
#[trial]
fn the_log_slider_starts_above_a_ratio_of_500() {
    let scale = |spec: &str| match Param::parse_list(spec).remove(0).kind {
        ParamKind::Range { scale, .. } => scale,
        _ => panic!("a range"),
    };
    let ratio = |max: f64| format!(r#"{{"r":1..{max}=1}}"#);
    assert_eq!(scale(&ratio(LOG_LIMIT)), SliderScale::Linear);
    assert_eq!(scale(&ratio(LOG_LIMIT + 1.0)), SliderScale::Log);
    assert_eq!(scale(r#"{"r":0..1000=1}"#), SliderScale::Linear);
}

/// A kernel that is not square keeps every cell and wraps `round(sqrt(n))` to a row; a kernel at 0 is the identity.
#[trial]
fn a_non_square_kernel_keeps_its_cells() {
    let params = Param::parse_list(r#"{"k":int[10]}"#);
    assert_eq!(params[0].kind, ParamKind::grid(10, 0));
    assert_eq!((ParamKind::grid_side(10), ParamKind::grid_side(25)), (3, 5));
    let Some(FieldValue::Grid(cells)) = params[0].kind.default_value() else { panic!("a grid") };
    assert_eq!(cells, [0, 0, 0, 0, 0, 1, 0, 0, 0, 0]);
    assert!(params[0].kind.fits(&FieldValue::Grid(vec![0; 10])));
}

/// A row is shown while its `visible_when` holds and takes input while its `enabled_when` holds; a row with units
/// accepts its own unit ids under its unit key.
#[trial]
fn row_conditions_and_unit_keys_follow_the_values() {
    let units = vec![
        NumberUnit::new("ppi", UnitKind::PixelsPerInch, "pixels/inch", 1.0),
        NumberUnit::new("ppcm", UnitKind::PixelsPerCentimetre, "pixels/cm", 2.54),
    ];
    let row = Param::number_with_units("output", "Output", 72.0, 1.0, 2, units);
    assert_eq!(row.unit_key(), Some("output.unit"));
    assert!(row.accepts("output.unit", &"ppcm".into()) && !row.accepts("output.unit", &"dpi".into()));

    let cell = Param::number("cell", "Cell size", 8.0, 1.0, 0).visible_when("screen", true).enabled_when("fine", true);
    let mut fields = DialogFields::from_pairs([("screen", false), ("fine", false)]);
    assert!(!cell.visible(&fields) && !cell.enabled(&fields));
    fields.set("screen", true);
    assert!(cell.visible(&fields) && !cell.enabled(&fields));
    fields.set("fine", true);
    assert!(cell.enabled(&fields));
}

#[trial]
fn opening_fills_defaults_unbound_and_through_the_binding() {
    {
        let units = vec![
            NumberUnit::new("ppi", UnitKind::PixelsPerInch, "pixels/inch", 1.0),
            NumberUnit::new("ppcm", UnitKind::PixelsPerCentimetre, "pixels/cm", 2.54),
        ];
        let params = vec![
            Param::new("method", "Method", ParamKind::choice(vec![("a".into(), "A".into()), ("b".into(), "B".into())], None)),
            Param::number_with_units("output", "Output", 72.0, 1.0, 2, units).default_unit("ppcm"),
            Param::number("size", "Size", 3.0, 1.0, 0),
        ];
        let mut fields = DialogFields::from_pairs([("size", 9.0)]);
        open_fields(&params, &mut fields);
        assert_eq!(fields.text("method"), Some("a"));
        assert_eq!((fields.number("output"), fields.text("output.unit")), (Some(72.0), Some("ppcm")));
        assert_eq!(fields.number("size"), Some(9.0));
        assert!(!fields.changed(), "the opening values are the snapshot");
    }
    {
        let params = Rc::new(Param::parse_list(r#"{"radius":0.1..1000=1,"method":"spin|zoom"="zoom","note":text}"#));
        let fields = Rc::new(RefCell::new(DialogFields::from_pairs([("note", "kept")])));
        let (shown_params, shown_fields) = (params.clone(), fields.clone());
        let mut fixture = Fixture::new(move |ui| {
            let mut held = shown_fields.borrow_mut();
            ParamRows::new("Filter", &shown_params, Bind::bound(&mut *held)).show(ui);
        });
        fixture.settle();
        let fields = fields.borrow();
        assert_eq!((fields.number("radius"), fields.text("method"), fields.text("note")), (Some(RADIUS_DEFAULT), Some("zoom"), Some("kept")));
    }
}

fn sample() -> DialogFields {
    let mut fields = DialogFields::from_pairs([("radius", FieldValue::Number(2.5)), ("method", "spin".into())]);
    fields.set("seed", 7_i64);
    fields.snapshot();
    fields
}

/// After a snapshot, edits then a reset bring every value back and report the change; keys added since stay.
#[trial]
fn reset_restores_the_opening_values() {
    let mut fields = sample();
    assert!(!fields.changed() && !fields.reset());
    fields.set("radius", 9.0);
    fields.remove("method");
    fields.set("extra", true);
    assert!(fields.changed());
    assert!(fields.reset());
    assert_eq!((fields.number("radius"), fields.text("method"), fields.bool("extra")), (Some(2.5), Some("spin"), Some(true)));
    assert!(!fields.changed());
}

/// A remembered map with a renamed key, an out-of-range number and a choice no longer offered keeps only the fitting
/// values.
#[trial]
fn remembered_values_that_no_longer_fit_are_dropped() {
    let mut fields = DialogFields::from_pairs([
        ("radius", FieldValue::Number(5000.0)),
        ("oldName", FieldValue::Number(1.0)),
        ("method", "wobble".into()),
        ("seed", 3_i64.into()),
    ]);
    fields.retain_fitting(&["radius", "method", "seed"], |key, value| match (key, value) {
        ("radius", FieldValue::Number(n)) => *n <= 1000.0,
        ("method", FieldValue::Text(t)) => t == "spin" || t == "zoom",
        _ => true,
    });
    assert_eq!(fields.iter().map(|(k, _)| k).collect::<Vec<_>>(), ["seed"]);
}

#[trial]
fn numbers_ints_and_stored_tables_read_back() {
    {
        let fields = DialogFields::from_pairs([("a", FieldValue::Number(3.0)), ("b", FieldValue::Int(3)), ("c", FieldValue::Number(3.5))]);
        assert_eq!((fields.int("a"), fields.number("b"), fields.int("c"), fields.int("missing")), (Some(3), Some(3.0), None, None));
    }
    {
        let text = "whole = 3\nreal = 2.5\nname = \"spin\"\ncolour = [0.25, 0.5, 1.0]\nlist = [1, 2, 3, 4]\nnone = []\n";
        let fields: DialogFields = toml::from_str(text).expect("a plain table parses");
        assert_eq!((fields.get("whole"), fields.get("real")), (Some(&FieldValue::Int(3)), Some(&FieldValue::Number(2.5))));
        assert_eq!(fields.text("name"), Some("spin"));
        assert_eq!(fields.color("colour"), Some([0.25, 0.5, 1.0]));
        assert_eq!(fields.list("list").map(|list| list.len()), Some(4));
        assert_eq!(fields.list("none").map(|list| list.len()), Some(0));
        assert_eq!(fields.list("whole"), None, "a scalar is no list");

        let mut opened = DialogFields::from_pairs([("a", 1.0)]);
        opened.snapshot();
        opened.set("a", 2.0);
        assert_eq!(opened, DialogFields::from_pairs([("a", 2.0)]));
    }
}

/// An arrow key in a radio param moves focus to the next option, Space picks it, and OK answers with the picked option.
#[trial]
fn an_arrow_key_and_space_move_a_radio_pick_and_the_result_carries_it() {
    let radio =
        ParamKind::radio(vec![("none".into(), "None".into()), ("rle".into(), "RLE".into()), ("zip".into(), "ZIP".into())], Some("none".into()));
    let params = vec![Param::new("compression", "Compression", radio)];
    let fields = Rc::new(RefCell::new(DialogFields::new()));
    let answer = Rc::new(RefCell::new(None::<ParamResult>));
    let (held, sink) = (Rc::clone(&fields), Rc::clone(&answer));
    let mut fixture = Fixture::new(move |ui| {
        let mut fields = held.borrow_mut();
        let mut open = true;
        let out =
            ParamDialog::new("targa", "Targa Options", params.clone()).values(Bind::bound(&mut *fields)).opened(Bind::bound(&mut open)).show(ui);
        if out.result.is_some() {
            *sink.borrow_mut() = out.result;
        }
    });
    fixture.settle();
    let none = fixture.by_label("None").expect("the first option is shown");
    fixture.click(&none);
    fixture.idle(2);
    fixture.key(Key::ArrowDown, Modifiers::NONE);
    fixture.idle(2);
    fixture.key(Key::Space, Modifiers::NONE);
    fixture.idle(2);
    let ok = fixture.by_label("OK").expect("the OK button is shown");
    fixture.click(&ok);
    fixture.idle(3);
    let result = answer.borrow().clone().expect("OK answers");
    assert_eq!(result.fields.text("compression"), Some("rle"), "the arrow and Space moved the pick from the first option to the next");
}
