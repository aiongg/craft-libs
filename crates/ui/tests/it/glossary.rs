//! The glossary names only items that exist, where it says they live: every backticked term of `docs/glossary.md`
//! that names code is defined by an item (parsed, not matched in text) in the module or crate of its "Lives in" cell,
//! and each `craft_ui` module of that cell resolves through the layer map.

use craft_ui::testing::trial;
use proc_macro2::TokenTree;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Backticked terms that name something other than an item of the crates.
const NOT_CODE: [&str; 1] = [
    // egui's `Ui`, which the contract hands to components.
    "Ui",
];

/// Layer-map modules of families whose pilot has not landed: their rows are skipped until it does.
const UNMIGRATED: [&str; 1] = ["graph"];

const ITEM_KEYWORDS: [&str; 9] = ["struct", "enum", "trait", "fn", "type", "const", "static", "mod", "union"];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn rust_files(path: &Path, found: &mut Vec<PathBuf>) {
    if path.is_file() {
        found.push(path.to_owned());
        return;
    }
    let Ok(entries) = fs::read_dir(path) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() || path.extension().is_some_and(|extension| extension == "rs") {
            rust_files(&path, found);
        }
    }
}

/// Where a "Lives in" home keeps its source: a crate's `src`, or a `craft_ui` module's file and directory.
#[derive(Debug)]
enum Home {
    Crate(String),
    Module(Vec<String>),
}

/// The module paths of the layer map of craft-ui, as `[a, b]` for `a::b`.
fn layer_map() -> Vec<Vec<String>> {
    let dir = workspace_root().join("design/layers/craft-ui");
    let mut modules = Vec::new();
    for entry in fs::read_dir(dir).expect("design/layers/craft-ui").flatten() {
        let Ok(text) = fs::read_to_string(entry.path()) else { continue };
        let table: toml::Table = text.parse().expect("layer map parses");
        for module in table.get("module").and_then(|m| m.as_array()).into_iter().flatten() {
            if let Some(path) = module.get("path").and_then(|p| p.as_str()) {
                modules.push(path.split("::").map(str::to_owned).collect());
            }
        }
    }
    modules
}

/// Source directory of the package `name` in the workspace.
fn crate_src(name: &str) -> Option<PathBuf> {
    let crates = fs::read_dir(workspace_root().join("crates")).ok()?;
    crates.flatten().map(|e| e.path()).find_map(|dir| {
        let manifest: toml::Table = fs::read_to_string(dir.join("Cargo.toml")).ok()?.parse().ok()?;
        (manifest.get("package")?.get("name")?.as_str()? == name).then(|| dir.join("src"))
    })
}

/// The homes a "Lives in" cell names. Parenthesised remarks are dropped; a cell naming a family, or no module or
/// crate, has none.
fn homes(cell: &str) -> Vec<Home> {
    let mut plain = String::new();
    let mut depth = 0;
    for c in cell.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ if depth == 0 => plain.push(c),
            _ => {}
        }
    }
    if plain.contains("famil") {
        return Vec::new();
    }
    plain
        .split(',')
        .filter_map(|part| {
            let part = part.trim().trim_matches('`');
            if let Some(path) = part.strip_prefix("craft_ui") {
                let segments: Vec<String> = path.split("::").filter(|s| !s.is_empty()).map(str::to_owned).collect();
                Some(Home::Module(segments))
            } else {
                part.starts_with("craft-").then(|| Home::Crate(part.to_owned()))
            }
        })
        .collect()
}

/// The names an item list defines: types, traits, functions, modules, macros and derives, nested modules included.
fn collect_names(items: &[syn::Item], names: &mut BTreeSet<String>) {
    for item in items {
        match item {
            syn::Item::Struct(i) => drop(names.insert(i.ident.to_string())),
            syn::Item::Enum(i) => drop(names.insert(i.ident.to_string())),
            syn::Item::Union(i) => drop(names.insert(i.ident.to_string())),
            syn::Item::Trait(i) => drop(names.insert(i.ident.to_string())),
            syn::Item::Type(i) => drop(names.insert(i.ident.to_string())),
            syn::Item::Const(i) => drop(names.insert(i.ident.to_string())),
            syn::Item::Static(i) => drop(names.insert(i.ident.to_string())),
            syn::Item::Fn(i) => {
                names.insert(i.sig.ident.to_string());
                for attr in i.attrs.iter().filter(|a| a.path().is_ident("proc_macro_derive")) {
                    let _ = attr.parse_nested_meta(|meta| {
                        names.extend(meta.path.get_ident().map(ToString::to_string));
                        Ok(())
                    });
                }
            }
            syn::Item::Mod(i) => {
                names.insert(i.ident.to_string());
                if let Some((_, inner)) = &i.content {
                    collect_names(inner, names);
                }
            }
            syn::Item::Macro(i) if i.mac.path.is_ident("macro_rules") => names.extend(i.ident.as_ref().map(ToString::to_string)),
            // Types defined by a macro invocation: `keyword Name` among its tokens.
            syn::Item::Macro(i) => keyword_names(i.mac.tokens.clone(), names),
            _ => {}
        }
    }
}

fn keyword_names(tokens: proc_macro2::TokenStream, names: &mut BTreeSet<String>) {
    let mut previous_keyword = false;
    for tree in tokens {
        match tree {
            TokenTree::Ident(ident) => {
                let word = ident.to_string();
                if previous_keyword {
                    names.insert(word.clone());
                }
                previous_keyword = ITEM_KEYWORDS.contains(&word.as_str());
            }
            TokenTree::Group(group) => {
                keyword_names(group.stream(), names);
                previous_keyword = false;
            }
            _ => previous_keyword = false,
        }
    }
}

fn names_in(files: &[PathBuf]) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for file in files {
        let Ok(text) = fs::read_to_string(file) else { continue };
        if let Ok(parsed) = syn::parse_file(&text) {
            collect_names(&parsed.items, &mut names);
        }
    }
    names
}

fn home_names(home: &Home) -> BTreeSet<String> {
    let mut files = Vec::new();
    match home {
        Home::Crate(name) => {
            if let Some(src) = crate_src(name) {
                rust_files(&src, &mut files);
            }
        }
        Home::Module(segments) => {
            let src = crate_src("craft-ui").expect("craft-ui");
            if segments.is_empty() {
                rust_files(&src, &mut files);
                files.extend(crate_src("craft-ui-macros").map(|m| m.join("lib.rs")));
            } else {
                let base = segments.iter().fold(src, |path, s| path.join(s));
                rust_files(&base.with_extension("rs"), &mut files);
                rust_files(&base, &mut files);
            }
        }
    }
    names_in(&files)
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

/// Whether the layer map covers `module`: an entry names it or one of its ancestors.
fn in_layer_map(map: &[Vec<String>], module: &[String]) -> bool {
    map.iter().any(|entry| module.starts_with(entry))
}

#[trial]
fn every_glossary_term_that_names_code_resolves_to_an_item_in_its_home() {
    let glossary = fs::read_to_string(workspace_root().join("docs/glossary.md")).unwrap_or_default();
    let rows: Vec<&str> = glossary.lines().filter(|line| line.starts_with("| ") && !line.starts_with("| Term")).collect();
    assert!(rows.len() > 100, "the glossary has its rows; found {}", rows.len());
    let map = layer_map();
    let mut cache: Vec<(String, BTreeSet<String>)> = Vec::new();
    let (mut missing, mut unmapped) = (BTreeSet::new(), BTreeSet::new());
    for row in rows {
        let cell = row.split('|').nth(3).unwrap_or_default();
        let homes = homes(cell);
        let migrated = |home: &Home| !matches!(home, Home::Module(m) if m.first().is_some_and(|f| UNMIGRATED.contains(&f.as_str())));
        if homes.is_empty() || !homes.iter().all(migrated) {
            continue;
        }
        for home in &homes {
            if let Home::Module(module) = home
                && !module.is_empty()
                && !in_layer_map(&map, module)
            {
                unmapped.insert(cell.trim().to_owned());
            }
        }
        let mut defined = BTreeSet::new();
        for home in &homes {
            let key = format!("{home:?}");
            if cache.iter().all(|(k, _)| *k != key) {
                cache.push((key.clone(), home_names(home)));
            }
            defined.extend(cache.iter().find(|(k, _)| *k == key).map(|(_, names)| names.clone()).unwrap_or_default());
        }
        for name in terms(row).into_iter().filter(|term| !NOT_CODE.contains(term)).filter_map(item_of) {
            if !defined.contains(name) {
                missing.insert(format!("{name} (lives in {})", cell.trim()));
            }
        }
    }
    assert!(unmapped.is_empty(), "'Lives in' modules the layer map does not cover: {unmapped:?}");
    assert!(missing.is_empty(), "glossary terms with no item in their home: {missing:#?}");
}
