#![allow(clippy::unwrap_used, clippy::expect_used)]

use serde_json::{Value, json};

use super::format::{self, ErrorKind, FormatErrors};
use super::semantic_tests::app;
use super::*;

fn sizes(v: &Value) -> AppSizes {
    AppSizes::parse(v.to_string().as_bytes()).unwrap()
}

fn rejected(v: &Value) -> FormatErrors {
    AppSizes::parse(v.to_string().as_bytes()).expect_err("sizes should be rejected")
}

fn at(size: Size, text_size: TextSize) -> Appearance {
    Appearance { size, text_size, ..app(Mode::Dark, Contrast::Standard) }
}

fn resolved(size: Size, sizes: &AppSizes) -> Tokens {
    let theme = builtins::suite_default();
    resolve_sized(theme, at(size, TextSize::Default), None, sizes).tokens
}

#[test]
fn a_base_override_moves_the_tokens_derived_from_it() {
    let before = resolved(Size::Comfortable, &AppSizes::default());
    let after = resolved(Size::Comfortable, &sizes(&json!({ "all": { "size.control.md": 28 } })));
    assert_eq!((before.size.control.md, after.size.control.md), (30.0, 28.0));
    assert_eq!(after.component.list.row_height, 28.0, "a list row is a control high");
    assert_eq!(after.component.slider.height.md, 22.0, "a slider is the control less its inset");
    assert_ne!(after.component.slider.width.md, before.component.slider.width.md, "a family scaled by the control height follows");
    assert_ne!(after.component.form.normal.row_gap, before.component.form.normal.row_gap);
    assert_eq!(after.component.toolbar, before.component.toolbar, "a value from the size set's own component data stays");
}

#[test]
fn a_pin_moves_one_token() {
    let before = resolved(Size::Comfortable, &AppSizes::default());
    let mut after = resolved(Size::Comfortable, &sizes(&json!({ "comfortable": { "toolbar.button": 40, "toolbar.pad.1": 12 } })));
    assert_eq!((after.component.toolbar.button, after.component.toolbar.pad), (40.0, [8.0, 12.0]));
    after.component.toolbar = before.component.toolbar;
    assert_eq!(after, before, "nothing else moves");
}

#[test]
fn a_pin_applies_after_a_base_override_derives_the_same_token() {
    let both = sizes(&json!({ "all": { "size.control.md": 28, "list.row_height": 40 } }));
    assert_eq!(resolved(Size::Compact, &both).component.list.row_height, 40.0);
}

#[test]
fn text_size_scales_an_app_type_override() {
    let app = sizes(&json!({ "all": { "typography.body.size": 15, "typography.body.line_height": 20 } }));
    let theme = builtins::suite_default();
    let large = resolve_sized(theme, at(Size::Compact, TextSize::ALL[TextSize::ALL.len() - 1]), None, &app).tokens;
    let mut expected = sizes::builtin(Size::Compact).base.typography;
    expected.body.size = 15.0;
    expected.body.line_height = 20.0;
    let factor = values::text_size_factor(TextSize::ALL[TextSize::ALL.len() - 1]);
    assert_eq!(large.typography.body, expected.scaled(factor, values::TYPE_ROUNDING).body);
    let default = resolve_sized(theme, at(Size::Compact, TextSize::Default), None, &app).tokens;
    assert_eq!(default.typography.body.size, 15.0);
}

#[test]
fn a_size_sets_own_map_wins_over_all() {
    let app = sizes(&json!({ "all": { "size.control.md": 28, "toolbar.gap": 2 }, "comfortable": { "size.control.md": 26 } }));
    assert_eq!(resolved(Size::Compact, &app).size.control.md, 28.0);
    assert_eq!(resolved(Size::Comfortable, &app).size.control.md, 26.0);
    assert_eq!(resolved(Size::Comfortable, &app).component.toolbar.gap, 2.0, "a path only in all applies to both");
}

#[test]
fn app_radii_apply_unless_the_theme_has_its_own_shape() {
    let app = sizes(&json!({ "all": { "radius.control": 2, "radius.button": "full" } }));
    let mut theme = builtins::suite_default().clone();
    theme.shape = None;
    let tokens = resolve_sized(&theme, at(Size::Compact, TextSize::Default), None, &app).tokens;
    assert_eq!((tokens.shape.radius.control, tokens.shape.radius.button), (2.0, scale::Radius::Full.px()));
    let own = RadiusSet { control: 9.0, ..sizes::builtin(Size::Compact).base.radius };
    theme.shape = Some(ShapeSeed { radius: Some(own), radius_offset: None, edge: None });
    let tokens = resolve_sized(&theme, at(Size::Compact, TextSize::Default), None, &app).tokens;
    assert_eq!(tokens.shape.radius, own, "the theme's shape wins over the app's radii");
}

#[test]
fn bad_paths_and_values_are_rejected_naming_the_path() {
    let cases = [
        (json!({ "all": { "size.control.huge": 28 } }), "all.size.control.huge", ErrorKind::UnknownToken),
        (json!({ "all": { "edge": "flat" } }), "all.edge", ErrorKind::UnknownToken),
        (json!({ "all": { "typography.body.font": "mono" } }), "all.typography.body.font", ErrorKind::UnknownToken),
        (json!({ "compact": { "size.control.md": 90 } }), "compact.size.control.md", ErrorKind::OutOfRange { min: 16.0, max: 48.0 }),
        (json!({ "compact": { "toolbar.button": 9000 } }), "compact.toolbar.button", ErrorKind::OutOfRange { min: 0.0, max: 1024.0 }),
        (json!({ "all": { "size.control.md": true } }), "all.size.control.md", ErrorKind::WrongType { expected: "a number" }),
        (json!({ "all": { "size.list.rounded": 1 } }), "all.size.list.rounded", ErrorKind::WrongType { expected: "true or false" }),
        (json!({ "all": { "toolbar.button": true } }), "all.toolbar.button", ErrorKind::WrongType { expected: "a number" }),
    ];
    for (file, path, kind) in cases {
        let errors = rejected(&file);
        let found = errors.0.iter().find(|e| e.path_string() == path).unwrap_or_else(|| panic!("no error at {path}: {errors}"));
        assert_eq!(found.kind, kind, "{path}");
    }
    assert_eq!(rejected(&json!({ "everyone": {} })).0[0].path_string(), "everyone");
}

#[test]
fn a_broken_cross_rule_is_rejected() {
    let errors = rejected(&json!({ "compact": { "size.control.sm": 30 } }));
    assert!(matches!(errors.0[0].kind, ErrorKind::CrossRule { .. }), "{errors}");
    assert_eq!(errors.0[0].path_string(), "compact.size.control.sm");
}

#[test]
fn a_theme_file_still_rejects_a_size_path() {
    let theme = |path: &str| {
        json!({ "schema": "craft-design-theme", "schema_version": 1, "id": "test.sizes", "name": "Sizes",
            "seeds": { "neutral": "#535353", "accent": "#78ADF6", "status": { "good": "#85DC6A", "warn": "#E8B046", "bad": "#FF8080", "info": "#78ADF6" } },
            "components": { path: 30 } })
    };
    for path in ["toolbar.button", "button.height.md", "size.control.md"] {
        let errors = format::parse(theme(path).to_string().as_bytes()).unwrap_err();
        assert!(errors.0.iter().any(|e| e.kind == ErrorKind::UnknownToken && e.path_string().contains(path)), "{path}: {errors}");
    }
    format::parse(theme("button.radius").to_string().as_bytes()).unwrap();
}

#[test]
fn every_size_path_round_trips_and_its_defaults_lie_in_range() {
    let fields = SizeField::all();
    let paths: std::collections::BTreeSet<String> = fields.iter().map(|f| f.path()).collect();
    assert_eq!(paths.len(), fields.len(), "paths are unique");
    for size in [Size::Compact, Size::Comfortable] {
        let tokens = resolved(size, &AppSizes::default());
        for &field in &fields {
            assert_eq!(SizeField::from_path(&field.path()), Some(field));
            assert_eq!(field.kind(), FieldKind::Length);
            let (lo, hi) = field.range();
            let value = field.get(&tokens.component);
            assert!((lo..=hi).contains(&value), "{} is {value} at {size:?}, outside {lo} to {hi}", field.path());
        }
    }
}

#[test]
fn a_size_path_shared_with_a_theme_field_is_that_length() {
    for field in SizeField::all() {
        let path = field.path();
        if let Some(themed) = ComponentField::from_path(&path) {
            assert_eq!(themed.kind(), FieldKind::Length, "{path}: a theme field with a size's path is a length");
        }
    }
}

#[test]
fn written_sizes_parse_back_to_the_same_overrides() {
    let original = sizes(&json!({
        "all": { "size.control.md": 28.5, "radius.button": "full", "size.list.rounded": true },
        "compact": { "toolbar.pad.0": 6, "typography.body.size": 15 },
    }));
    let text = original.to_json();
    assert!(text.starts_with("{\n  \"all\": {"), "{text}");
    assert!(!text.contains("comfortable"), "an empty section is left out");
    assert_eq!(AppSizes::parse(text.as_bytes()).unwrap(), original);
    assert_eq!(AppSizes::default().to_json(), "{}\n");
}
