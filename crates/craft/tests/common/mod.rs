//! Reads the member crates' manifests for the facade tests.
#![allow(dead_code)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Crates the facade leaves out, by directory, with the reason. Everything else under `crates/` is a member.
pub const NOT_IN_FACADE: &[(&str, &str)] = &[
    ("behavior", "layer 1: pure behaviours with no egui, below the new craft-ui"),
    ("design-lint", "a lint template and checker for apps' clippy.toml, run from xtask"),
    ("fonts-build", "a build-script helper; apps call it from build.rs"),
    ("media-testkit", "test support: ffmpeg discovery and fixture locations for the media crates"),
    ("plate-test", "a test harness: reference PDFs and PDF structure checks"),
    ("ui", "the new craft-ui, not yet behind the facade"),
    ("ui-gallery", "a tool: the story gallery application"),
    ("ui-gallery-legacy", "a tool: the story gallery application on craft-ui-legacy"),
    ("ui-macros", "a proc-macro crate that the new craft-ui re-exports"),
    ("ui-macros-legacy", "a proc-macro crate that craft-ui-legacy re-exports"),
];

/// Directories whose crate the facade exposes under another name until cut-over: the facade's `craft-ui` is `crates/ui-legacy`.
const FACADE_NAME_OF_DIR: &[(&str, &str)] = &[("ui-legacy", "ui")];

/// Crates every app gets without a feature.
pub const ALWAYS_ON: &[&str] = &["host", "i18n", "theme"];

pub struct Member {
    /// The facade name; the package the facade depends on is `craft-<dir>`.
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
        .map(|folder| {
            let manifest = read_toml(&crates.join(&folder).join("Cargo.toml"));
            let dir = FACADE_NAME_OF_DIR.iter().find(|(f, _)| *f == folder).map_or(folder.clone(), |(_, n)| (*n).to_owned());
            let name = manifest["package"]["name"].as_str().unwrap();
            let expected = if dir == folder { format!("craft-{dir}") } else { format!("craft-{folder}") };
            assert_eq!(name, expected, "crates/{folder} is not named {expected}");
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
