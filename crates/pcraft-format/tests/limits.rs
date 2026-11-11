//! Integration tests of craft-pcraft-format: manifest size and depth limits.
use craft_pcraft_format::{Error, Limits, check_manifest, parse_manifest};

fn nested(depth: usize) -> Vec<u8> {
    let mut json = "[".repeat(depth).into_bytes();
    json.extend(std::iter::repeat_n(b']', depth));
    json
}

fn small(max_depth: usize, max_items: usize) -> Limits {
    let mut limits = Limits::default();
    limits.max_depth = max_depth;
    limits.max_items = max_items;
    limits
}

#[test]
fn nesting_up_to_max_depth_passes_and_one_more_level_does_not() {
    let limits = Limits::default();
    let at = limits.max_depth;
    assert!(check_manifest(&nested(at), &limits).is_ok());
    assert!(matches!(check_manifest(&nested(at + 1), &limits), Err(Error::LimitExceeded(_))));
}

#[test]
fn a_manifest_deeper_than_serde_json_s_128_levels_parses() {
    let value = parse_manifest(&nested(300), &Limits::default()).unwrap();
    assert!(value.is_array());
}

#[test]
fn brackets_and_commas_inside_strings_are_not_counted() {
    let limits = small(2, 3);
    assert!(check_manifest(br#"{"a":"[[[{{{,,,,"}"#, &limits).is_ok());
    assert!(check_manifest(br#"{"a":"\"[[[,,,\\"}"#, &limits).is_ok());
    assert!(matches!(check_manifest(br#"{"a":"\\",[[[]]]}"#, &limits), Err(Error::LimitExceeded(_))));
}
