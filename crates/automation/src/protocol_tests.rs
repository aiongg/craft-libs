#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
//! Behaviour tests of `protocol`.

use super::*;
use serde_json::json;

// Unknown fields, wrong types, unknown choices and out-of-range numbers are errors naming the field.
#[test]
fn params_errors_name_the_field() {
    let v = json!({"x": 3, "n": 1.5, "flag": 1, "name": "up", "big": 1e39, "count": -1, "extra": null});
    let p = Params(&v);
    assert_eq!(p.num_field("x"), Ok(Some(3.0)));
    assert_eq!(p.num_field("missing"), Ok(None));
    assert_eq!(p.uint_field("x"), Ok(Some(3)));
    assert!(p.uint_field("count").unwrap_err().contains("count"));
    assert!(p.uint_field("n").unwrap_err().contains("n"));
    assert!(p.bool_field("flag").unwrap_err().contains("flag"));
    assert!(p.f32_field("big").unwrap_err().contains("big"));
    assert!(p.str_field("x").unwrap_err().contains("x"));
    assert!(p.choice("name", "button", &BUTTONS).unwrap_err().contains("`up`"));
    assert_eq!(p.choice("name", "name", &["up", "down"]), Ok(Some("up")));
    assert!(p.number_in("x", "x", Some((0.0, 2.0))).unwrap_err().contains("x"));
    assert!(p.number_in("name", "x", None).unwrap_err().contains("x"));

    // Null fields mean omitted; unknown ones are named.
    assert_eq!(p.only(None, &["x", "n", "flag", "name", "big", "count", "extra"]).unwrap().get("extra"), None);
    assert!(p.only(None, &["x"]).unwrap_err().contains("`big`"));
    assert!(Params(&json!(5)).only(None, &[]).unwrap_err().contains("params"));
    assert_eq!(Params(&json!(null)).only(None, &["x"]).unwrap(), json!({}));
    assert!(Params(&json!({"z": 1})).only(Some("modifiers"), &["shift"]).unwrap_err().contains("`z`"));

    #[cfg(feature = "ui")]
    {
        let grouped = json!({"modifiers": {"shift": true, "ctrl": true}, "alt": true});
        let m = Params(&grouped).modifiers(&KEY_MODIFIERS).unwrap();
        assert!(m.shift && (m.cmd || m.ctrl) && !m.alt);
        let bad = json!({"modifiers": {"shift": "yes"}});
        assert!(Params(&bad).modifiers(&KEY_MODIFIERS).unwrap_err().contains("shift"));
        let space = json!({"modifiers": {"space": true}});
        assert!(Params(&space).modifiers(&KEY_MODIFIERS).unwrap_err().contains("space"));
        assert!(Params(&json!({"alt": 1})).modifiers(&KEY_MODIFIERS).unwrap_err().contains("alt"));
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    struct Panels {
        a: bool,
        b: bool,
    }
    let current = Panels { a: true, b: false };
    let patch = json!({"b": true});
    assert_eq!(Params(&patch).merged_into("panels", &current), Ok(Panels { a: true, b: true }));
    assert!(Params(&json!({"c": true})).merged_into("panels", &current).unwrap_err().contains("`c`"));
    assert!(Params(&json!({"a": 3})).merged_into("panels", &current).unwrap_err().contains("panels"));
    assert!(Params(&json!([])).merged_into("panels", &current).unwrap_err().contains("panels"));

    // `whole` replaces the value but still names a typo (the type does not deny unknown fields).
    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Default)]
    #[serde(default)]
    struct Layout {
        a: bool,
        b: bool,
    }
    let layout = Layout { a: true, b: false };
    assert_eq!(Params(&json!({"b": true})).whole("dock", &layout), Ok(Layout { a: false, b: true }));
    assert!(Params(&json!({"typo": 1})).whole("dock", &layout).unwrap_err().contains("`typo`"));
    assert!(Params(&json!(3)).whole("dock", &layout).unwrap_err().contains("dock"));
}

// A reply over MAX_RESPONSE_BYTES is replaced by an error reply.
#[test]
fn oversized_reply_becomes_an_error() {
    let mut out = Vec::new();
    write_reply(&mut out, &reply_ok(&json!(4), json!({"n": 1}))).unwrap();
    assert_eq!(out, b"{\"id\":4,\"ok\":true,\"result\":{\"n\":1}}\n");

    let mut out = Vec::new();
    let huge = reply_ok(&json!(9), json!("x".repeat(MAX_RESPONSE_BYTES)));
    write_reply(&mut out, &huge).unwrap();
    let written: Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(written, reply_err(&json!(9), format!("response exceeds {MAX_RESPONSE_BYTES} bytes; operation may have completed")));
    assert_eq!(out.last(), Some(&b'\n'));

    let request = Request::parse(r#"{"id": "a", "method": "ui.click"}"#).unwrap();
    assert_eq!((request.id, request.method, request.params), (json!("a"), "ui.click".to_owned(), json!({})));
    assert!(Request::parse("{").unwrap_err().starts_with("bad JSON: "));
}
