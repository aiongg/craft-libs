//! The glossary names only items that exist: every backticked term of `docs/glossary.md` that names code resolves
//! to a definition in the workspace's crates.

use craft_ui::testing::trial;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Backticked terms that name something other than an item of the crates.
const NOT_CODE: [&str; 1] = [
    // egui's `Ui`, which the contract hands to components.
    "Ui",
];

const ITEM_KEYWORDS: [&str; 9] = ["struct ", "enum ", "trait ", "fn ", "type ", "const ", "static ", "mod ", "macro_rules! "];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn rust_files(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

/// The identifier after `keyword` when `line` defines an item with it.
fn defined_name<'a>(line: &'a str, keyword: &str) -> Option<&'a str> {
    let at = line.find(keyword)?;
    let boundary = !line[..at].ends_with(|c: char| c.is_alphanumeric() || c == '_');
    let rest = &line[at + keyword.len()..];
    let end = rest.find(|c: char| !c.is_alphanumeric() && c != '_').unwrap_or(rest.len());
    (boundary && end > 0).then(|| &rest[..end])
}

/// The names of the items defined in the new crates: types, traits, functions, modules, macros and derives.
fn defined_names() -> BTreeSet<String> {
    let mut files = Vec::new();
    let Ok(crates) = fs::read_dir(workspace_root().join("crates")) else { return BTreeSet::new() };
    for entry in crates.flatten() {
        if !entry.file_name().to_string_lossy().ends_with("-legacy") {
            rust_files(&entry.path().join("src"), &mut files);
        }
    }
    let mut names = BTreeSet::new();
    for file in files {
        let Ok(text) = fs::read_to_string(file) else { continue };
        for line in text.lines() {
            let line = line.trim_start();
            for keyword in ITEM_KEYWORDS {
                names.extend(defined_name(line, keyword).map(str::to_owned));
            }
            if let Some(derive) = line.strip_prefix("#[proc_macro_derive(") {
                names.extend(derive.split(|c: char| !c.is_alphanumeric() && c != '_').next().map(str::to_owned));
            }
        }
    }
    names
}

/// The backticked terms of the first column of a glossary row.
fn terms(row: &str) -> Vec<&str> {
    let Some(cell) = row.split('|').nth(1) else { return Vec::new() };
    cell.split('`').skip(1).step_by(2).collect()
}

/// The item a backticked term names, if it names one: its first segment when that is a type, else its last.
/// File names, glob patterns and expressions name none.
fn item_of(term: &str) -> Option<&str> {
    let path = term.split(['(', '<']).next()?.trim_end_matches('!');
    if path.is_empty() || !path.chars().all(|c| c.is_alphanumeric() || c == '_' || c == ':') {
        return None;
    }
    let mut segments = path.split("::").filter(|segment| !segment.is_empty());
    let first = segments.next()?;
    if first.starts_with(|c: char| c.is_uppercase()) { Some(first) } else { segments.last().or(Some(first)) }
}

#[trial]
fn every_glossary_term_that_names_code_resolves_to_an_item() {
    let glossary = fs::read_to_string(workspace_root().join("docs/glossary.md")).unwrap_or_default();
    let rows: Vec<&str> = glossary.lines().filter(|line| line.starts_with("| ") && !line.starts_with("| Term")).collect();
    assert!(rows.len() > 100, "the glossary has its rows; found {}", rows.len());
    let defined = defined_names();
    let missing: BTreeSet<&str> = rows
        .iter()
        .flat_map(|row| terms(row))
        .filter(|term| !NOT_CODE.contains(term))
        .filter_map(item_of)
        .filter(|name| !defined.contains(*name))
        .collect();
    assert!(missing.is_empty(), "glossary terms with no item in the crates: {missing:?}");
}
