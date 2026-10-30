use serde_json::{Value, json};

use crate::format::{ErrorKind, FormatErrors, parse_sizes, write_sizes};
use crate::sizes::builtin;
use crate::theme::Size;

const COMPACT: &str = include_str!("../sizes/compact.json");
const COMFORTABLE: &str = include_str!("../sizes/comfortable.json");

/// The compact file with `path` (dot-separated) set to `value`, or removed when `None`.
fn edited(path: &str, value: Option<Value>) -> Vec<u8> {
    let mut v: Value = serde_json::from_str(COMPACT).unwrap();
    let keys: Vec<&str> = path.split('.').collect();
    let (last, parents) = keys.split_last().unwrap();
    let mut at = &mut v;
    for k in parents {
        at = at.get_mut(*k).unwrap();
    }
    let obj = at.as_object_mut().unwrap();
    match value {
        Some(value) => obj.insert((*last).to_owned(), value),
        None => obj.remove(*last),
    };
    v.to_string().into_bytes()
}

fn rejected(bytes: &[u8]) -> FormatErrors {
    parse_sizes(bytes).expect_err("file should be rejected")
}

fn error_at<'a>(errors: &'a FormatErrors, path: &str) -> &'a ErrorKind {
    &errors.0.iter().find(|e| e.path_string() == path).unwrap_or_else(|| panic!("no error at {path}: {errors}")).kind
}

#[test]
fn built_in_files_parse_and_name_their_size() {
    assert_eq!(builtin(Size::Compact).id, "compact");
    assert_eq!(builtin(Size::Comfortable).id, "comfortable");
    for size in [Size::Compact, Size::Comfortable] {
        builtin(size).validate().unwrap();
    }
}

#[test]
fn files_round_trip_and_keep_unknown_top_level_keys() {
    for text in [COMPACT, COMFORTABLE] {
        let file = parse_sizes(text.as_bytes()).unwrap();
        let written = write_sizes(&file).unwrap();
        assert_eq!(written, text, "the built-in files are in the written form");
        assert_eq!(parse_sizes(written.as_bytes()).unwrap().set, file.set);
    }
    let mut v: Value = serde_json::from_str(COMPACT).unwrap();
    v["note"] = json!({"kept": true});
    let written = write_sizes(&parse_sizes(v.to_string().as_bytes()).unwrap()).unwrap();
    assert!(written.contains("\"note\""), "{written}");
}

#[test]
fn out_of_range_unknown_and_wrong_kind_values_are_rejected_naming_the_path() {
    let e = rejected(&edited("base.size.control.md", Some(json!(60))));
    assert!(matches!(error_at(&e, "base.size.control.md"), ErrorKind::OutOfRange { .. }));
    let e = rejected(&edited("components.toolbar.pad", Some(json!([6, 99]))));
    assert!(matches!(error_at(&e, "components.toolbar.pad"), ErrorKind::OutOfRange { .. }));
    let e = rejected(&edited("base.size.bogus", Some(json!(12))));
    assert_eq!(error_at(&e, "base.size.bogus"), &ErrorKind::UnknownToken);
    let e = rejected(&edited("components.button.nope", Some(json!({"x": 1}))));
    assert_eq!(error_at(&e, "components.button.nope"), &ErrorKind::UnknownToken);
    let e = rejected(&edited("base.size.list.rounded", Some(json!(1))));
    assert!(matches!(error_at(&e, "base.size.list.rounded"), ErrorKind::WrongType { .. }));
    let e = rejected(&edited("base.size.control.md", Some(json!("24"))));
    assert!(matches!(error_at(&e, "base.size.control.md"), ErrorKind::WrongType { .. }));
    let e = rejected(&edited("base.typography.body.font", Some(json!("serif"))));
    assert!(matches!(error_at(&e, "base.typography.body.font"), ErrorKind::BadValue { .. }));
    let e = rejected(&edited("components.toast.pad", None));
    assert_eq!(error_at(&e, "components.toast.pad"), &ErrorKind::Missing);
}

#[test]
fn broken_cross_rules_are_rejected_naming_the_path() {
    let e = rejected(&edited("base.size.control.sm", Some(json!(30))));
    assert!(matches!(error_at(&e, "base.size.control.sm"), ErrorKind::CrossRule { .. }));
    let e = rejected(&edited("base.typography.body.line_height", Some(json!(10))));
    assert!(matches!(error_at(&e, "base.typography.body.line_height"), ErrorKind::CrossRule { .. }));
    let e = rejected(&edited("base.size.list.thumb.md", Some(json!(40))));
    assert!(matches!(error_at(&e, "base.size.list.thumb.md"), ErrorKind::CrossRule { .. }));
}
