#![allow(clippy::unwrap_used, reason = "tests")]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use craft_design_lint::{CLIPPY_TEMPLATE, ClippyDrift, DriftKind, Error, check_clippy, clippy_drift, clippy_shadows, merge_clippy, write_clippy};

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn origin() -> &'static Path {
    Path::new("clippy.toml")
}

/// Every `reason` string in the template.
fn reasons() -> Vec<String> {
    CLIPPY_TEMPLATE.lines().filter_map(|l| l.split_once("reason = \"")?.1.split_once('"').map(|(r, _)| r.to_owned())).collect()
}

/// Every `path` string in the template.
fn paths() -> Vec<String> {
    CLIPPY_TEMPLATE.lines().filter_map(|l| l.split_once("path = \"")?.1.split_once('"').map(|(p, _)| p.to_owned())).collect()
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

fn declares(files: &[PathBuf], name: &str) -> bool {
    let kinds = ["struct", "enum", "trait", "fn", "type"];
    files.iter().any(|f| {
        std::fs::read_to_string(f).unwrap().lines().any(|l| {
            let l = l.trim_start();
            kinds.iter().any(|k| {
                l.strip_prefix("pub ")
                    .and_then(|r| r.strip_prefix(k))
                    .and_then(|r| r.strip_prefix(' '))
                    .is_some_and(|r| r.split(|c: char| !c.is_alphanumeric() && c != '_').next() == Some(name))
            })
        })
    })
}

/// Names a reason mentions that are not craft-ui or craft-theme items.
const NOT_CRAFT: &[&str] = &["Color32"];

#[test]
fn template_replacements_exist() {
    let ui = manifest_dir().join("../ui/src");
    let theme = manifest_dir().join("../theme/src");
    let (mut ui_files, mut all_files) = (Vec::new(), Vec::new());
    rust_files(&ui, &mut ui_files);
    rust_files(&theme, &mut all_files);
    all_files.extend(ui_files.iter().cloned());

    let mut missing = Vec::new();
    for reason in reasons() {
        for token in reason.split(|c: char| !c.is_alphanumeric() && c != '_' && c != ':') {
            let segments: Vec<&str> = token.split("::").filter(|s| !s.is_empty()).collect();
            let upper = |s: &str| s.starts_with(|c: char| c.is_ascii_uppercase());
            if segments.len() > 1 && segments[0] == "craft_ui" {
                let module = segments[1];
                let in_module = [ui.join(module), ui.join(format!("{module}.rs"))].iter().any(|p| p.exists());
                if !in_module {
                    missing.push(format!("{token}: no module `{module}`"));
                }
                if let Some(name) = segments.iter().copied().find(|s| upper(s))
                    && !declares(&ui_files, name)
                {
                    missing.push(format!("{token}: no `pub` item `{name}` in craft-ui"));
                }
            } else if let Some(name) = segments.first().copied().filter(|s| upper(s))
                && !NOT_CRAFT.contains(&name)
                && !declares(&all_files, name)
            {
                missing.push(format!("{token}: no `pub` item `{name}` in craft-ui or craft-theme"));
            }
        }
    }
    assert!(missing.is_empty(), "replacements named in the template that craft-ui does not export:\n{}", missing.join("\n"));
}

#[test]
fn template_paths_are_unique() {
    let mut seen = BTreeMap::new();
    for p in paths() {
        *seen.entry(p).or_insert(0) += 1;
    }
    let dupes: Vec<_> = seen.iter().filter(|(_, n)| **n > 1).collect();
    assert!(dupes.is_empty(), "duplicate template paths: {dupes:?}");
}

#[test]
fn check_clippy_detects_drift() {
    // The full block passes, with extra app entries and unrelated keys.
    let full = format!("allow-unwrap-in-tests = true\n{CLIPPY_TEMPLATE}");
    assert_eq!(clippy_drift(origin(), &full).unwrap(), []);
    let with_extra = merge_clippy(origin(), "disallowed-types = [{ path = \"my::Widget\", reason = \"mine\" }]\n").unwrap();
    assert_eq!(clippy_drift(origin(), &with_extra).unwrap(), []);

    // A missing entry yields exactly one finding naming it.
    let without: String = CLIPPY_TEMPLATE.lines().filter(|l| !l.contains("\"egui::Ui::button\"")).collect::<Vec<_>>().join("\n");
    let drift = clippy_drift(origin(), &without).unwrap();
    assert_eq!(drift, [ClippyDrift { key: "disallowed-methods", path: "egui::Ui::button".into(), kind: DriftKind::Missing }]);
    assert!(drift[0].to_string().contains("`egui::Ui::button`"));

    // A reworded reason is drift too.
    let reworded = CLIPPY_TEMPLATE.replace("use craft_ui::primitives::Divider", "use something else");
    let kinds: Vec<_> = clippy_drift(origin(), &reworded).unwrap().iter().map(|d| d.kind).collect();
    assert_eq!(kinds, [DriftKind::Differs, DriftKind::Differs]);

    // No file at all: everything is missing.
    let dir = tempdir("drift-none");
    let Err(Error::ClippyDrift(all)) = check_clippy(&dir) else { panic!("expected drift") };
    assert_eq!(all.len(), paths().len());
}

#[test]
fn write_merges_and_keeps_comments() {
    let before = "\
# the app's own settings
allow-unwrap-in-tests = true
cognitive-complexity-threshold = 30  # tuned

disallowed-types = [
    { path = \"egui::ComboBox\", reason = \"old wording\" },
    # kept: the app's own entry
    { path = \"my::Widget\", reason = \"mine\" },
]
";
    let merged = merge_clippy(origin(), before).unwrap();
    assert!(merged.contains("# the app's own settings"));
    assert!(merged.contains("cognitive-complexity-threshold = 30  # tuned"));
    assert!(merged.contains("# kept: the app's own entry"));
    assert!(merged.contains("{ path = \"my::Widget\", reason = \"mine\" }"));
    assert!(!merged.contains("old wording"));
    assert!(merged.contains("BEGIN craft-design-system") && merged.contains("END craft-design-system"));
    assert_eq!(clippy_drift(origin(), &merged).unwrap(), []);
    // Template entries come first and the app's entry follows them.
    assert!(merged.find("egui::containers::menu::SubMenuButton") < merged.find("my::Widget"));
    // Merging again changes nothing.
    assert_eq!(merge_clippy(origin(), &merged).unwrap(), merged);

    let dir = tempdir("write");
    let file = dir.join("clippy.toml");
    assert!(write_clippy(&file).unwrap());
    assert!(!write_clippy(&file).unwrap());
    check_clippy(&dir).unwrap();
}

#[test]
fn fixture_clippy_toml_is_the_template() {
    check_clippy(manifest_dir().join("tests/fixtures/clippy-app")).unwrap();
}

/// Runs clippy on the fixture workspace: one diagnostic per template entry in
/// `clippy-app`, none in `clippy-plain` (no egui) and no complaint about
/// unresolved paths. Slow (builds egui), so run on request:
/// `cargo test -p craft-design-lint -- --ignored`.
#[test]
#[ignore = "builds egui for the fixture"]
fn clippy_flags_every_entry() {
    let fixture = manifest_dir().join("tests/fixtures/clippy-app");
    let target = std::env::var_os("CARGO_TARGET_DIR").map_or_else(|| fixture.join("target"), |t| PathBuf::from(t).join("clippy-app"));
    let out = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .args(["clippy", "--workspace", "--message-format", "json"])
        .current_dir(&fixture)
        .env("CARGO_TARGET_DIR", target)
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));

    let mut counts: BTreeMap<String, u32> = BTreeMap::new();
    let mut others = Vec::new();
    for line in String::from_utf8(out.stdout).unwrap().lines() {
        let value: serde_json::Value = serde_json::from_str(line).unwrap();
        if value["reason"] != "compiler-message" {
            continue;
        }
        let text = value["message"]["message"].as_str().unwrap().to_owned();
        match text.strip_prefix("use of a disallowed ").and_then(|t| t.split('`').nth(1)) {
            Some(path) => *counts.entry(path.to_owned()).or_default() += 1,
            None if text.starts_with("warning:") || text.contains("generated") => {}
            None => others.push(text),
        }
    }
    let want: BTreeMap<String, u32> = paths().into_iter().map(|p| (p, 1)).collect();
    assert_eq!(counts, want);
    assert!(others.is_empty(), "unexpected diagnostics: {others:?}");
}

fn tempdir(name: &str) -> PathBuf {
    let base = std::env::var_os("TMPDIR").map_or_else(std::env::temp_dir, PathBuf::from);
    let dir = base.join(format!("craft-design-lint-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn another_clippy_toml_shadows_the_root_one() {
    let write = |root: &Path, rel: &str, text: &str| {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    let root = tempdir("shadow");
    write(&root, "clippy.toml", CLIPPY_TEMPLATE);
    write(&root, "Cargo.toml", "[workspace]\nmembers = [\"crates/*\"]\n");
    write(&root, "crates/a/Cargo.toml", "[package]\nname = \"a\"\n");
    write(&root, "crates/b/Cargo.toml", "[package]\nname = \"b\"\n");
    let file = root.join("clippy.toml");
    assert_eq!(clippy_shadows(&file, None).unwrap(), []);
    check_clippy(&root).unwrap();

    // A member crate's own clippy.toml, or a parent directory's, hides the root one from that crate.
    write(&root, "crates/a/clippy.toml", "");
    let shadows = clippy_shadows(&file, None).unwrap();
    assert_eq!(shadows.iter().map(|s| s.path.clone()).collect::<Vec<_>>(), [root.join("crates/a")]);
    assert!(shadows[0].to_string().contains("error[clippy/shadowed]"), "{}", shadows[0]);
    assert!(matches!(check_clippy(&root), Err(Error::ClippyShadowed(s)) if s.len() == 1));
    std::fs::remove_file(root.join("crates/a/clippy.toml")).unwrap();
    write(&root, "crates/.clippy.toml", "");
    assert_eq!(clippy_shadows(&file, None).unwrap().len(), 1);
    std::fs::remove_file(root.join("crates/.clippy.toml")).unwrap();

    // A clippy.toml in a directory without a package below it, in a nested workspace, or under target/ is not read for the root.
    write(&root, "docs/clippy.toml", "");
    write(&root, "vendor/Cargo.toml", "[workspace]\n");
    write(&root, "vendor/x/Cargo.toml", "[package]\nname = \"x\"\n");
    write(&root, "vendor/x/clippy.toml", "");
    write(&root, "target/y/Cargo.toml", "[package]\nname = \"y\"\n");
    write(&root, "target/y/clippy.toml", "");
    assert_eq!(clippy_shadows(&file, None).unwrap(), []);

    // CLIPPY_CONF_DIR: this directory (or a child without its own file) is fine; elsewhere is not.
    assert_eq!(clippy_shadows(&file, Some(root.as_os_str())).unwrap(), []);
    assert_eq!(clippy_shadows(&file, Some(root.join("crates/a").as_os_str())).unwrap(), []);
    let other = tempdir("shadow-other");
    write(&other, "clippy.toml", "");
    let shadows = clippy_shadows(&file, Some(other.as_os_str())).unwrap();
    assert_eq!(shadows.len(), 1);
    assert!(shadows[0].reason.contains("CLIPPY_CONF_DIR"), "{}", shadows[0].reason);
    let none = tempdir("shadow-none");
    assert_eq!(clippy_shadows(&file, Some(none.as_os_str())).unwrap().len(), 1);
}
