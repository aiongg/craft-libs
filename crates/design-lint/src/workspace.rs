//! The workspace's [`RuleScope`]: the layer map, `cargo metadata` and the allowances of spec 24.1 in one place.
//!
//! Paths the rules ask about are relative to the repository root. A file's module comes from the module
//! tree of its crate (`Index`), its layer from the layer map, and the egui facts from the crates'
//! dependencies and `[[test]]` tables.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::contract::RuleScope;
use crate::graph::{Metadata, Package};
use crate::layers::{LayerMaps, relative};
use crate::resolve::Index;
use crate::structure::LAYERS_DIR;
use crate::{Config, Error, Finding, check_structure, check_with_scope, exempt_sites};

/// Types whose cases or fields are fixed by meaning (spec 24.1, `api/non-exhaustive`).
const CLOSED: [&str; 9] = ["Mixed", "Politeness", "Numeric", "HitPriority", "ScrollPos", "PartRef", "SetPosition", "States", "StateMix"];

/// Permission keys whose only allowed location is craft-ui's `runtime::egui`.
const EGUI_KEYS: [&str; 9] = [
    "contract/egui-import",
    "contract/allocate",
    "contract/egui-widget",
    "contract/access",
    "contract/keys",
    "contract/pointer",
    "contract/motion",
    "contract/containers",
    "contract/platform",
];

/// One allowed location: files of `krate` whose module path starts with `module` (the whole crate when empty).
struct Place {
    krate: &'static str,
    module: &'static [&'static str],
}

const fn place(krate: &'static str, module: &'static [&'static str]) -> Place {
    Place { krate, module }
}

/// The locations that may use what `key` forbids. A key not listed here is allowed nowhere.
fn places(key: &str) -> Vec<Place> {
    match key {
        k if EGUI_KEYS.contains(&k) => vec![place("craft-ui", &["runtime", "egui"])],
        "style-mutation" => vec![place("craft-ui", &["runtime", "egui", "style"]), place("craft-ui", &["runtime", "egui", "scale"])],
        "contract/memory" => vec![place("craft-ui", &["runtime", "egui", "memory"])],
        "contract/layers" => vec![place("craft-ui", &["runtime", "egui", "areas"])],
        "contract/motion:clock" => vec![place("craft-ui", &["runtime", "motion"])],
        "contract/platform:target" => vec![place("craft-platform", &[]), place("craft-host", &["target"])],
        "test/kit" => vec![place("craft-ui", &["testing"])],
        _ => Vec::new(),
    }
}

struct CrateInfo {
    name: String,
    dir: PathBuf,
    egui_linked: bool,
    /// Test sources of `harness = false` targets: a file, or the directory of a `main.rs`.
    harness_free: Vec<PathBuf>,
    /// Module path of each source file of the library tree.
    modules: BTreeMap<PathBuf, Vec<String>>,
}

/// A [`RuleScope`] read from the repository: `design/layers/`, `cargo metadata` and spec 24.1.
pub struct WorkspaceScope {
    maps: LayerMaps,
    crates: Vec<CrateInfo>,
}

fn is_egui_crate(name: &str) -> bool {
    name == "eframe" || name.starts_with("egui") || name.starts_with("epaint")
}

/// Crates that depend on an egui crate, directly or through another workspace crate.
fn egui_linked(meta: &Metadata) -> BTreeSet<String> {
    let mut linked: BTreeSet<String> =
        meta.packages.iter().filter(|p| p.dependencies.iter().any(|d| is_egui_crate(&d.name))).map(|p| p.name.clone()).collect();
    loop {
        let more: Vec<String> = meta
            .packages
            .iter()
            .filter(|p| !linked.contains(&p.name) && p.dependencies.iter().any(|d| linked.contains(&d.name)))
            .map(|p| p.name.clone())
            .collect();
        if more.is_empty() {
            return linked;
        }
        linked.extend(more);
    }
}

/// Test and bench targets with `harness = false`, as paths relative to the crate directory.
fn harness_free_targets(dir: &Path) -> Result<Vec<PathBuf>, Error> {
    let manifest = dir.join("Cargo.toml");
    let Ok(text) = std::fs::read_to_string(&manifest) else { return Ok(Vec::new()) };
    let table: toml::Table = toml::from_str(&text).map_err(|e| Error::Toml { path: manifest.clone(), message: e.to_string() })?;
    let mut out = Vec::new();
    for (kind, folder) in [("test", "tests"), ("bench", "benches")] {
        for target in table.get(kind).and_then(toml::Value::as_array).into_iter().flatten().filter_map(toml::Value::as_table) {
            if target.get("harness").and_then(toml::Value::as_bool) != Some(false) {
                continue;
            }
            let name = target.get("name").and_then(toml::Value::as_str).unwrap_or_default();
            let path = match target.get("path").and_then(toml::Value::as_str) {
                Some(p) => PathBuf::from(p),
                None if dir.join(folder).join(name).join("main.rs").is_file() => Path::new(folder).join(name).join("main.rs"),
                None => Path::new(folder).join(format!("{name}.rs")),
            };
            out.push(if path.file_name().is_some_and(|f| f == "main.rs") { path.parent().map(Path::to_path_buf).unwrap_or_default() } else { path });
        }
    }
    Ok(out)
}

impl WorkspaceScope {
    /// Reads the layer maps below `root` and the module trees of the mapped crates.
    pub fn load(root: &Path, meta: &Metadata) -> Result<Self, Error> {
        let canonical = std::fs::canonicalize(root).map_err(|source| Error::Io { path: root.to_owned(), source })?;
        let rel = |p: &Path| relative(&canonical, &std::fs::canonicalize(p).unwrap_or_else(|_| p.to_owned()));
        let (maps, _) = LayerMaps::load(root, &root.join(LAYERS_DIR))?;
        let mapped: Vec<(String, PathBuf)> =
            meta.packages.iter().filter(|p| maps.get(&p.name).is_some()).filter_map(|p| p.lib_root.clone().map(|r| (p.name.clone(), r))).collect();
        let index = Index::build(&mapped)?;
        let linked = egui_linked(meta);
        let mut crates = Vec::new();
        for package in &meta.packages {
            let Some(dir) = package.dir.as_ref() else { continue };
            crates.push(CrateInfo {
                name: package.name.clone(),
                dir: rel(dir),
                egui_linked: linked.contains(&package.name),
                harness_free: harness_free_targets(dir)?.into_iter().map(|p| rel(dir).join(p)).collect(),
                modules: module_files(&index, package, &rel),
            });
        }
        Ok(Self { maps, crates })
    }

    fn crate_of(&self, file: &Path) -> Option<&CrateInfo> {
        self.crates.iter().filter(|c| file.starts_with(&c.dir)).max_by_key(|c| c.dir.components().count())
    }
}

/// Each source file of the package's library tree with the path of the module it holds; for a file
/// that also holds inline modules, the shortest path.
fn module_files(index: &Index, package: &Package, rel: &dyn Fn(&Path) -> PathBuf) -> BTreeMap<PathBuf, Vec<String>> {
    let mut files: BTreeMap<PathBuf, Vec<String>> = BTreeMap::new();
    if let Some(tree) = index.trees.get(&package.name.replace('-', "_")) {
        for module in tree.modules.values() {
            let slot = files.entry(rel(&module.file)).or_insert_with(|| module.path.clone());
            if module.path.len() < slot.len() {
                slot.clone_from(&module.path);
            }
        }
    }
    files
}

fn has_prefix(path: &[String], prefix: &[&str]) -> bool {
    path.len() >= prefix.len() && path.iter().zip(prefix).all(|(a, b)| a == b)
}

impl RuleScope for WorkspaceScope {
    fn allows(&self, key: &str, file: &Path) -> bool {
        let Some(krate) = self.crate_of(file) else { return false };
        if key == "test/layout" {
            return file.starts_with(krate.dir.join("tests").join("it"));
        }
        let module = krate.modules.get(file);
        places(key).iter().any(|p| p.krate == krate.name && (p.module.is_empty() || module.is_some_and(|m| has_prefix(m, p.module))))
    }

    fn layer(&self, file: &Path) -> Option<u8> {
        let krate = self.crate_of(file)?;
        let module = krate.modules.get(file)?;
        self.maps.get(&krate.name)?.layer_info(module).map(|i| i.layer)
    }

    fn egui_linked(&self, file: &Path) -> bool {
        self.crate_of(file).is_some_and(|c| c.egui_linked)
    }

    fn harness_free(&self, file: &Path) -> bool {
        self.crate_of(file).is_some_and(|c| c.harness_free.iter().any(|t| file.starts_with(t)))
    }

    fn is_closed(&self, name: &str) -> bool {
        CLOSED.contains(&name)
    }
}

/// The whole check of a repository: the structure checks of [`check_structure`] and the egui-ban and API
/// rules of [`check_with_scope`] over the config's paths, with the layer map as the rules' scope.
///
/// Findings are sorted by path and line.
pub fn check_workspace(root: &Path, config: &Config, meta: &Metadata) -> Result<Vec<Finding>, Error> {
    let mut findings = check_structure(root, meta, &exempt_sites(config))?;
    let scope = WorkspaceScope::load(root, meta)?;
    findings.extend(check_with_scope(config, root, &scope)?.findings);
    findings.sort_by(|a, b| (&a.path, a.line, a.rule).cmp(&(&b.path, b.line, b.rule)));
    Ok(findings)
}
