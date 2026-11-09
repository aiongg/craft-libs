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

fn sample() -> DialogFields {
    let mut fields = DialogFields::from_pairs([("radius", FieldValue::Number(2.5)), ("method", "spin".into())]);
    fields.set("seed", 7_i64);
    fields.snapshot();
    fields
}

/// A stored table reads back with its integers as integers and its lists as lists, and equality ignores the opening
/// snapshot.
#[trial]
fn a_stored_table_reads_back_by_kind() {
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

/// The rows write the defaults of the parameters the bound values lack, through the binding, and keep a value that is
/// there.
#[trial]
fn rows_fill_defaults_through_the_binding() {
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

