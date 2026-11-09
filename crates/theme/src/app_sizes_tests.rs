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
    resolve_sized(theme, at(size, TextSize::Small), None, sizes).tokens
}

#[test]
fn a_base_override_moves_the_tokens_derived_from_it() {
    let before = resolved(Size::Comfortable, &AppSizes::default());
    let after = resolved(Size::Comfortable, &sizes(&json!({ "all": { "size.control.md": 28, "size.icon.md": 20 } })));
    assert_eq!((before.size.control.md, after.size.control.md), (30.0, 28.0));
    assert_eq!(after.component.slider.thumb_height.md, 16.0, "a slider thumb is the icon size less its inset");
    assert_ne!(after.component.slider.thumb_height.md, before.component.slider.thumb_height.md);
}

#[test]
fn text_size_scales_an_app_type_override() {
    let app = sizes(&json!({ "all": { "typography.body.size": 15, "typography.body.line_height": 20 } }));
    let theme = builtins::suite_default();
    let large = resolve_sized(theme, at(Size::Compact, TextSize::ALL[TextSize::ALL.len() - 1]), None, &app).tokens;
    let mut expected = sizes::builtin(Size::Compact).base.typography;
    expected.body.size = 15.0;
    expected.body.line_height = 20.0;
    let factor = TextSizeSteps::PHOTOSHOP.factor(TextSize::Large);
    assert_eq!(large.typography.body, expected.scaled(factor, values::TYPE_ROUNDING).body);
    let default = resolve_sized(theme, at(Size::Compact, TextSize::Small), None, &app).tokens;
    assert_eq!(default.typography.body.size, 15.0);
}

#[test]
fn app_radii_apply_unless_the_theme_has_its_own_shape() {
    let app = sizes(&json!({ "all": { "radius.control": 2, "radius.button": "full" } }));
    let mut theme = builtins::suite_default().clone();
    theme.shape = None;
    let tokens = resolve_sized(&theme, at(Size::Compact, TextSize::Small), None, &app).tokens;
    assert_eq!((tokens.shape.radius.control, tokens.shape.radius.button), (2.0, scale::Radius::Full.px()));
    let own = RadiusSet { control: 9.0, ..sizes::builtin(Size::Compact).base.radius };
    theme.shape = Some(ShapeSeed { radius: Some(own), radius_offset: None, button_radius: None, edge: None, rows: None });
    let tokens = resolve_sized(&theme, at(Size::Compact, TextSize::Small), None, &app).tokens;
    assert_eq!(tokens.shape.radius, RadiusSet { sm: 5.0, ..own }, "the theme's shape wins over the app's radii; sm is half of control");
}

#[test]
fn bad_paths_and_values_are_rejected_naming_the_path() {
    let cases = [
        (json!({ "all": { "size.control.huge": 28 } }), "all.size.control.huge", ErrorKind::UnknownToken),
        (json!({ "all": { "edge": "flat" } }), "all.edge", ErrorKind::UnknownToken),
        (json!({ "all": { "typography.body.font": "mono" } }), "all.typography.body.font", ErrorKind::UnknownToken),
        (json!({ "compact": { "size.control.md": 90 } }), "compact.size.control.md", ErrorKind::OutOfRange { min: 16.0, max: 48.0 }),
        (json!({ "compact": { "toolbar.button": 40 } }), "compact.toolbar.button", ErrorKind::UnknownToken),
        (json!({ "all": { "size.list.rounded": true } }), "all.size.list.rounded", ErrorKind::UnknownToken),
        (json!({ "all": { "size.control.md": true } }), "all.size.control.md", ErrorKind::WrongType { expected: "a number" }),
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
    for path in ["toolbar.button", "button.height.md", "size.control.md", "button.radius"] {
        let errors = format::parse(theme(path).to_string().as_bytes()).unwrap_err();
        assert!(errors.0.iter().any(|e| e.kind == ErrorKind::UnknownToken && e.path_string().contains(path)), "{path}: {errors}");
    }
    format::parse(theme("slider.thumb_width").to_string().as_bytes()).unwrap();
}

#[test]
fn written_sizes_parse_back_to_the_same_overrides() {
    let original = sizes(&json!({
        "all": { "size.control.md": 28.5, "radius.button": "full" },
        "compact": { "space.pad.panel": 6, "typography.body.size": 15 },
    }));
    let text = original.to_json();
    assert!(text.starts_with("{\n  \"all\": {"), "{text}");
    assert!(!text.contains("comfortable"), "an empty section is left out");
    assert_eq!(AppSizes::parse(text.as_bytes()).unwrap(), original);
    assert_eq!(AppSizes::default().to_json(), "{}\n");
}

