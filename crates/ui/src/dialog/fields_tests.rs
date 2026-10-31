//! The field map's snapshot, reset and remembered-value filtering.

use super::{DialogFields, FieldValue};

fn sample() -> DialogFields {
    let mut fields = DialogFields::from_pairs([("radius", FieldValue::Number(2.5)), ("method", "spin".into())]);
    fields.set("seed", 7_i64);
    fields.snapshot();
    fields
}

// After `snapshot`, edits then `reset` bring every value back and report the change; `changed`
// is false again.
#[test]
fn reset_restores_the_opening_values() {
    let mut fields = sample();
    assert!(!fields.changed());
    assert!(!fields.reset());

    fields.set("radius", 9.0);
    fields.remove("method");
    fields.set("extra", true);
    assert!(fields.changed());

    assert!(fields.reset());
    assert_eq!(fields.number("radius"), Some(2.5));
    assert_eq!(fields.text("method"), Some("spin"));
    assert_eq!(fields.bool("extra"), Some(true), "keys added since the snapshot stay");
    assert!(!fields.changed());
}

// A remembered map with a renamed key, an out-of-range number and a choice no longer offered
// keeps only the fitting values.
#[test]
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
    let keys: Vec<_> = fields.iter().map(|(k, _)| k).collect();
    assert_eq!(keys, ["seed"]);
}

// `int` reads `3.0` as 3 and `number` reads `Int(3)` as 3.0, as a script writes either.
#[test]
fn numbers_and_ints_read_each_other() {
    let fields = DialogFields::from_pairs([("a", FieldValue::Number(3.0)), ("b", FieldValue::Int(3)), ("c", FieldValue::Number(3.5))]);
    assert_eq!(fields.int("a"), Some(3));
    assert_eq!(fields.number("b"), Some(3.0));
    assert_eq!(fields.int("c"), None);
    assert_eq!(fields.int("missing"), None);
}

// The map round-trips through TOML with every value kind, for `Remembered`.
#[test]
fn the_map_round_trips_through_toml() {
    let mut fields = DialogFields::new();
    fields.set("bool", true);
    fields.set("number", 2.5);
    fields.set("whole_number", 3.0);
    fields.set("int", 4_i64);
    fields.set("text", "spin");
    fields.set("color", FieldValue::Color([0.25, 0.5, 1.0]));
    fields.set("point", FieldValue::Point([12.5, 50.0]));
    fields.set("grid", FieldValue::Grid(vec![0, 0, 0, 0, 1, 0, 0, 0, -1]));

    let text = toml::to_string(&fields).expect("serialises");
    let back: DialogFields = toml::from_str(&text).expect("parses");
    assert_eq!(back, fields, "{text}");
}

// Equality compares the values; the opening snapshot is not part of it.
#[test]
fn equality_ignores_the_opening_snapshot() {
    let mut opened = DialogFields::from_pairs([("a", 1.0)]);
    opened.snapshot();
    opened.set("a", 2.0);
    let fresh = DialogFields::from_pairs([("a", 2.0)]);
    assert_eq!(opened, fresh);
    assert_ne!(opened, DialogFields::from_pairs([("a", 3.0)]));
}
