//! Writes `docs/components-index.md`: every registered component type, every craft-behavior module and every
//! pattern, with its module and one line of purpose from its doc comment.
//!
//! `cargo run -p craft-ui-gallery --bin components-index` rewrites the file; with `--check` it exits with an error
//! when the file differs from what the sources give.

use craft_ui::testing::registered;
use std::collections::BTreeMap;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const OUTPUT: &str = "docs/components-index.md";
const SKIPPED_DIRECTORIES: [&str; 2] = ["target", "vendor"];

/// One `pub` item found in the sources.
struct Item {
    /// The defining file, relative to the workspace root.
    file: PathBuf,
    purpose: String,
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The sources of the crates that are not frozen copies.
fn sources(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(crates) = fs::read_dir(root.join("crates")) else { return found };
    for entry in crates.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.ends_with("-legacy") && !SKIPPED_DIRECTORIES.contains(&name.as_str()) {
            collect(&entry.path().join("src"), &mut found);
        }
    }
    found.sort();
    found
}

fn collect(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

/// The first sentence of the first paragraph of the doc comment lines.
fn purpose(lines: &[&str]) -> String {
    let paragraph: Vec<&str> = lines.iter().map(|line| line.trim()).take_while(|line| !line.is_empty()).collect();
    let text = paragraph.join(" ");
    let sentence = text.split_once(". ").map_or(text.as_str(), |(first, _)| first);
    sentence.trim_end_matches('.').replace('|', "\\|")
}

/// The doc comment ending directly above line `at`, attributes skipped.
fn doc_above(lines: &[&str], at: usize) -> String {
    let mut docs = Vec::new();
    for line in lines[..at].iter().rev() {
        let line = line.trim();
        if let Some(doc) = line.strip_prefix("///") {
            docs.push(doc);
        } else if !line.starts_with("#[") && !line.starts_with(")]") && !line.starts_with(']') {
            break;
        }
    }
    docs.reverse();
    purpose(&docs)
}

fn definition_name(line: &str) -> Option<&str> {
    let line = line.trim_start().strip_prefix("pub ")?;
    let line = line.strip_prefix("(crate) ").unwrap_or(line);
    let rest = ["struct ", "enum ", "trait ", "type "].iter().find_map(|keyword| line.strip_prefix(keyword))?;
    let end = rest.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(rest.len());
    Some(&rest[..end])
}

fn pattern_name(line: &str) -> Option<&str> {
    let line = line.trim_start().strip_prefix("impl")?;
    let rest = line.split_once("Pattern for ")?.1;
    let end = rest.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(rest.len());
    Some(&rest[..end])
}

/// The public types and the `Pattern` impls of every source file, by type name.
fn scan(root: &Path) -> (BTreeMap<String, Vec<Item>>, Vec<String>) {
    let mut types: BTreeMap<String, Vec<Item>> = BTreeMap::new();
    let mut patterns = Vec::new();
    for file in sources(root) {
        let Ok(text) = fs::read_to_string(&file) else { continue };
        let relative = file.strip_prefix(root).unwrap_or(&file).to_path_buf();
        let lines: Vec<&str> = text.lines().collect();
        for (at, line) in lines.iter().enumerate() {
            if let Some(name) = definition_name(line) {
                types.entry(name.to_owned()).or_default().push(Item { file: relative.clone(), purpose: doc_above(&lines, at) });
            }
            if let Some(name) = pattern_name(line) {
                patterns.push(name.to_owned());
            }
        }
    }
    patterns.sort();
    patterns.dedup();
    (types, patterns)
}

/// The module path of a source file: `crates/ui/src/controls/check.rs` is `craft_ui::controls::check`.
fn module_of(file: &Path) -> String {
    let parts: Vec<String> = file.iter().map(|part| part.to_string_lossy().into_owned()).collect();
    let crate_dir = parts.get(1).map_or("", String::as_str);
    let mut module = vec![format!("craft_{}", crate_dir.replace('-', "_"))];
    for part in parts.iter().skip(3) {
        let part = part.trim_end_matches(".rs");
        if !matches!(part, "mod" | "lib" | "main") {
            module.push(part.to_owned());
        }
    }
    module.join("::")
}

/// The definition of `name` that best matches `module`: the one whose file path is the module, else the first.
fn pick<'a>(items: &'a [Item], module: &str) -> Option<&'a Item> {
    items.iter().find(|item| module_of(&item.file) == module).or_else(|| items.first())
}

fn short_name(type_name: &str) -> &str {
    let path = type_name.split('<').next().unwrap_or(type_name);
    path.rsplit("::").next().unwrap_or(path)
}

fn table_row(cells: &[&str]) -> String {
    format!("| {} |\n", cells.join(" | "))
}

fn render(root: &Path) -> String {
    let (types, patterns) = scan(root);
    let mut out = String::from("# Built components\n\n");
    out.push_str("Generated by `cargo run -p craft-ui-gallery --bin components-index`; edit the doc comments, not this file.\n\n");

    out.push_str("## Registered types\n\n| Type | Shape | Module | Purpose |\n|---|---|---|---|\n");
    let mut rows: Vec<(String, String, String, String)> = registered()
        .iter()
        .filter_map(|registration| registration.probe())
        .map(|probe| {
            let name = probe.type_name();
            let purpose = types.get(short_name(name)).and_then(|items| pick(items, probe.module)).map(|item| item.purpose.clone());
            let shown = name.rsplit_once("::").map_or(name, |(_, last)| last);
            (probe.module.to_owned(), shown.to_owned(), format!("{:?}", probe.shape), purpose.unwrap_or_default())
        })
        .collect();
    rows.sort();
    rows.dedup();
    for (module, name, shape, purpose) in rows {
        out.push_str(&table_row(&[&format!("`{name}`"), &shape, &format!("`{module}`"), &purpose]));
    }

    out.push_str("\n## Behaviours\n\n| Module | Purpose |\n|---|---|\n");
    let behavior_src = root.join("crates/behavior/src");
    for file in sources(root).into_iter().filter(|file| file.starts_with(&behavior_src)) {
        let relative = file.strip_prefix(root).unwrap_or(&file);
        let module = module_of(relative);
        if module.matches("::").count() != 1 {
            continue;
        }
        let Ok(text) = fs::read_to_string(&file) else { continue };
        let docs: Vec<&str> = text.lines().take_while(|line| line.starts_with("//!")).map(|line| &line[3..]).collect();
        out.push_str(&table_row(&[&format!("`{module}`"), &purpose(&docs)]));
    }

    out.push_str("\n## Patterns\n\n| Pattern | Module | Purpose |\n|---|---|---|\n");
    for name in patterns {
        let item = types.get(&name).and_then(|items| items.first());
        let module = item.map(|item| module_of(&item.file)).unwrap_or_default();
        let purpose = item.map(|item| item.purpose.clone()).unwrap_or_default();
        out.push_str(&table_row(&[&format!("`{name}`"), &format!("`{module}`"), &purpose]));
    }
    out
}

fn main() -> ExitCode {
    let root = workspace_root();
    let text = render(&root);
    let target = root.join(OUTPUT);
    if std::env::args().any(|argument| argument == "--check") {
        let current = fs::read_to_string(&target).unwrap_or_default();
        if current == text {
            return ExitCode::SUCCESS;
        }
        let _ = writeln!(std::io::stderr(), "{OUTPUT} is stale; run the generator");
        return ExitCode::FAILURE;
    }
    match fs::write(&target, text) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = writeln!(std::io::stderr(), "cannot write {OUTPUT}: {error}");
            ExitCode::FAILURE
        }
    }
}
