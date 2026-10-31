//! The parameter dialog in a harness.

use super::{FieldValue, Param, ParamKind};
use crate::controls::SliderScale;

// PhotoCraft's notation parses into the kinds, bounds, defaults and labels the dialog shows.
#[test]
fn the_registry_notation_parses_into_params() {
    let params = Param::parse_list(
        r#"{"radius":0.1..1000=1,"method":"spin|zoom"="zoom","monochromatic":bool,"seed":u32=0,"cellSize":px=4,"tint":color=#ff8000,"kernel":int[25],"layer":layer id,"curve":json,"note":text} -> {"x":1}"#,
    );
    let keys: Vec<_> = params.iter().map(|p| p.key.as_str()).collect();
    assert_eq!(keys, ["radius", "method", "monochromatic", "seed", "cellSize", "tint", "kernel", "note"]);
    assert_eq!(params[0].label, "Radius:");
    assert_eq!(params[4].label, "Cell size:");

    assert_eq!(params[0].kind, ParamKind::Range { min: 0.1, max: 1000.0, default: 1.0, scale: SliderScale::Log });
    let options = vec![("spin".to_owned(), "Spin".to_owned()), ("zoom".to_owned(), "Zoom".to_owned())];
    assert_eq!(params[1].kind, ParamKind::Choice { options, default: Some("zoom".to_owned()) });
    assert_eq!(params[2].kind, ParamKind::Bool { default: false });
    assert_eq!(params[3].kind, ParamKind::Int { min: None, max: None, default: 0 });
    assert_eq!(params[5].kind, ParamKind::Color { default: Some([1.0, 128.0 / 255.0, 0.0]) });
    assert_eq!(params[6].kind, ParamKind::Grid { side: 5, default: 0 });

    let kernel = params[6].kind.default_value().expect("a grid has a default");
    assert_eq!(kernel, FieldValue::Grid((0..25).map(|i| i64::from(i == 12)).collect()));
    assert!(params[6].kind.fits(&kernel));
    assert!(!params[0].kind.fits(&FieldValue::Number(5000.0)));
    assert!(!params[1].kind.fits(&FieldValue::Text("wobble".into())));
    assert_eq!(params[5].kind.default_value(), Some(FieldValue::Color([1.0, 128.0 / 255.0, 0.0])));
}

// The first field opens selected; typing replaces its value and Enter answers OK with it.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn typing_into_the_first_field_and_enter_answers_ok() {
    todo!()
}

// Alt turns Cancel into Reset; a click restores the opening values and the dialog stays open.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn alt_cancel_resets_to_the_opening_values() {
    todo!()
}

// With `remember`, OK stores the values and the next open starts from them; Cancel does not store.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn ok_remembers_the_values_and_cancel_does_not() {
    todo!()
}

// Preview off, or the label held, makes `preview_on` false; the host's image shows in the pane
// otherwise.
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn the_preview_check_box_and_hold_to_compare_drive_preview_on() {
    todo!()
}

// Tab walks the fields only, never the footer buttons (dialog rule).
#[test]
#[ignore = "skeleton"]
#[expect(clippy::todo, reason = "skeleton: removed when the body lands")]
fn tab_visits_the_fields_only() {
    todo!()
}
