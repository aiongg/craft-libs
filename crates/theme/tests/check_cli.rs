//! Runs the `craft-theme` binary on theme files.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::PathBuf;
use std::process::{Command, Output};

use serde_json::{Value, json};

fn theme(extra: Value) -> Value {
    let mut v = json!({
        "schema": "craft-design-theme", "schema_version": 1, "id": "test.cli",
        "seeds": { "neutral": "#8b8d98", "accent": "#3e63dd",
                   "status": { "good": "#30a46c", "warn": "#ffc53d", "bad": "#e5484d", "info": "#0090ff" } }
    });
    v.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
    v
}

fn file(name: &str, v: &Value) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("check_cli");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, serde_json::to_vec(v).unwrap()).unwrap();
    path
}

fn run(args: &[&str], files: &[&PathBuf]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_craft-theme")).arg("check").args(args).args(files).output().unwrap()
}

fn stdout(o: &Output) -> String {
    String::from_utf8(o.stdout.clone()).unwrap()
}

/// These seeds derive an `accent.solid` that misses 3:1 on the dark `surface.raised`, where check
/// boxes draw their on fill: `accent.solid` has a rule on `surface.panel` only. The component pass
/// reports it in both dark sizes.
const SEED_WARNINGS: usize = 2;

#[test]
fn valid_file_has_no_errors_and_reports_every_failing_pair() {
    let ok = file("ok.json", &theme(json!({})));
    let out = run(&[], &[&ok]);
    assert_eq!(out.status.code(), Some(0), "{}", stdout(&out));
    assert!(stdout(&out).ends_with(&format!("1 file(s) checked: 0 error(s), {SEED_WARNINGS} warning(s)\n")), "{}", stdout(&out));

    let out = run(&["--json"], &[&ok]);
    let v: Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(v[0]["errors"], json!([]));
    let warnings = v[0]["warnings"].as_array().unwrap();
    assert_eq!(warnings.len(), SEED_WARNINGS);
    for w in warnings {
        assert_eq!((&w["fg"], &w["bg"]), (&json!("check.on.fill.rest"), &json!("surface.raised")), "{w}");
        assert!(w["appearance"].as_str().unwrap().starts_with("dark/standard/"), "{w}");
    }
    assert_eq!(v[0]["file"], ok.to_str().unwrap());
}

#[test]
fn pinned_low_contrast_text_warns_and_exits_zero() {
    let weak = file("weak.json", &theme(json!({ "light": { "text.faint": "#e0e0e0" } })));
    let out = run(&[], &[&weak]);
    assert_eq!(out.status.code(), Some(0));
    let text = stdout(&out);
    assert!(text.contains("warning: light/standard/compact: text.faint on "), "{text}");
    assert!(!text.lines().any(|l| l.contains("dark/") && l.contains("text.faint")), "the light pin doesn't warn in dark: {text}");

    let out = run(&["--json"], &[&weak]);
    let v: Value = serde_json::from_str(&stdout(&out)).unwrap();
    let w = &v[0]["warnings"][0];
    assert_eq!(w["appearance"], "light/standard/compact");
    assert_eq!(w["fg"], "text.faint");
    assert!(w["ratio"].as_f64().unwrap() < w["minimum"].as_f64().unwrap());
}

#[test]
fn invalid_file_exits_one_with_every_error() {
    let bad = file("bad.json", &theme(json!({ "dark": { "surface.panel": "nope" }, "id": "Bad" })));
    let ok = file("ok2.json", &theme(json!({})));
    let out = run(&[], &[&bad, &ok]);
    assert_eq!(out.status.code(), Some(1));
    let text = stdout(&out);
    assert!(text.contains("error: id: "), "{text}");
    assert!(text.contains("error: dark.\"surface.panel\": not a colour"), "{text}");
    assert!(text.ends_with(&format!("2 file(s) checked: 2 error(s), {SEED_WARNINGS} warning(s)\n")), "{text}");

    let out = run(&["--json"], &[&bad]);
    let v: Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(v[0]["errors"][1], json!({ "path": "dark.\"surface.panel\"", "message": "not a colour" }));

    let missing = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("check_cli/absent.json");
    assert_eq!(run(&[], &[&missing]).status.code(), Some(1));
    assert_eq!(Command::new(env!("CARGO_BIN_EXE_craft-theme")).output().unwrap().status.code(), Some(2));
}

#[test]
fn built_in_theme_files_have_no_errors() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("themes");
    let files: Vec<PathBuf> = std::fs::read_dir(dir).unwrap().map(|e| e.unwrap().path()).collect();
    assert_eq!(files.len(), 12);
    let out = run(&[], &files.iter().collect::<Vec<_>>());
    assert_eq!(out.status.code(), Some(0), "{}", stdout(&out));
    assert!(stdout(&out).contains("12 file(s) checked: 0 error(s)"), "{}", stdout(&out));
}
