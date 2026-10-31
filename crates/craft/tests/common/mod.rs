//! Reads the member crates' manifests for the facade tests.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Crates the facade leaves out, with the reason. Everything else under `crates/` is a member.
pub const NOT_IN_FACADE: &[(&str, &str)] = &[
    ("design-lint", "a lint template and checker for apps' clippy.toml, run from xtask"),
    ("fonts-build", "a build-script helper; apps call it from build.rs"),
    ("plate-test", "a test harness: reference PDFs and PDF structure checks"),
    ("ui-gallery", "a tool: the story gallery application"),
    ("ui-macros", "a proc-macro crate that craft-ui re-exports"),
];

/// Crates every app gets without a feature.
pub const ALWAYS_ON: &[&str] = &["host", "i18n", "theme"];

pub struct Member {
    /// Directory name; the package is `craft-<dir>`.
    pub dir: String,
    pub manifest: toml::Table,
    pub features: BTreeMap<String, Vec<String>>,
}

impl Member {
    pub fn always_on(&self) -> bool {
        ALWAYS_ON.contains(&self.dir.as_str())
    }
}

pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn read_toml(path: &Path) -> toml::Table {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    text.parse().unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// Every facade member except the facade itself, sorted by directory name.
pub fn members() -> Vec<Member> {
    let crates = workspace_root().join("crates");
    let mut dirs: Vec<String> = std::fs::read_dir(&crates)
        .unwrap()
        .map(|e| e.unwrap())
        .filter(|e| e.path().join("Cargo.toml").is_file())
        .map(|e| e.file_name().into_string().unwrap())
        .filter(|d| d != "craft" && !NOT_IN_FACADE.iter().any(|(n, _)| n == d))
        .collect();
    dirs.sort();
    dirs.into_iter()
        .map(|dir| {
            let manifest = read_toml(&crates.join(&dir).join("Cargo.toml"));
            let name = manifest["package"]["name"].as_str().unwrap();
            assert_eq!(name, format!("craft-{dir}"), "crates/{dir} is not named craft-{dir}");
            let features = manifest
                .get("features")
                .and_then(|f| f.as_table())
                .map(|t| t.iter().map(|(k, v)| (k.clone(), v.as_array().unwrap().iter().map(|s| s.as_str().unwrap().to_owned()).collect())).collect())
                .unwrap_or_default();
            Member { dir, manifest, features }
        })
        .collect()
}

/// The crate directory a feature entry points at: `dep:craft-x`, `craft-x/f` and `craft-x?/f` give `x`.
pub fn crate_of(entry: &str) -> Option<&str> {
    let entry = entry.strip_prefix("dep:").unwrap_or(entry);
    let name = entry.split('/').next()?.trim_end_matches('?');
    name.strip_prefix("craft-")
}
