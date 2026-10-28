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

#[test]
fn valid_file_has_no_errors_or_warnings() {
    let ok = file("ok.json", &theme(json!({})));
    let out = run(&[], &[&ok]);
    assert_eq!(out.status.code(), Some(0), "{}", stdout(&out));
    assert!(stdout(&out).ends_with("1 file(s) checked: 0 error(s), 0 warning(s)\n"), "{}", stdout(&out));

    let out = run(&["--json"], &[&ok]);
    let v: Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(v[0]["errors"], json!([]));
    assert_eq!(v[0]["warnings"], json!([]));
    assert_eq!(v[0]["file"], ok.to_str().unwrap());
}

#[test]
fn pinned_low_contrast_text_warns_and_exits_zero() {
    let weak = file("weak.json", &theme(json!({ "light": { "text.faint": "#e0e0e0" } })));
    let out = run(&[], &[&weak]);
    assert_eq!(out.status.code(), Some(0));
    let text = stdout(&out);
    assert!(text.contains("warning: light/standard/compact: text.faint on "), "{text}");
    assert!(!text.contains("dark/"), "{text}");

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
    assert!(text.ends_with("2 file(s) checked: 2 error(s), 0 warning(s)\n"), "{text}");

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
    assert_eq!(files.len(), craft_theme::builtins().len());
    let out = run(&[], &files.iter().collect::<Vec<_>>());
    assert_eq!(out.status.code(), Some(0), "{}", stdout(&out));
    let summary = format!("{} file(s) checked: 0 error(s)", files.len());
    assert!(stdout(&out).contains(&summary), "{}", stdout(&out));
}
