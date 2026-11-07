//! The API map files (`migration/api-map/*.toml`) have the shape the codemods and the app scan read: known keys,
//! a status that agrees with the paths, and `[[call]]` entries that are Rust expressions.

use craft_ui::testing::trial;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const STATUSES: [&str; 5] = ["pending", "same", "moved", "replaced", "removed"];

fn text<'a>(entry: &'a toml::Table, key: &str) -> &'a str {
    entry.get(key).and_then(toml::Value::as_str).unwrap_or_default()
}

fn entries<'a>(file: &'a toml::Table, kind: &str) -> impl Iterator<Item = &'a toml::Table> {
    file.get(kind).and_then(toml::Value::as_array).into_iter().flatten().filter_map(toml::Value::as_table)
}

fn keys_within(entry: &toml::Table, allowed: &[&str], at: &str, problems: &mut Vec<String>) {
    for key in entry.keys().filter(|key| !allowed.contains(&key.as_str())) {
        problems.push(format!("{at}: unknown key `{key}`"));
    }
}

fn is_path(text: &str) -> bool {
    text.is_empty() || syn::parse_str::<syn::Path>(text).is_ok()
}

fn is_expression(text: &str) -> bool {
    syn::parse_str::<syn::Expr>(text).is_ok()
}

fn check_types(name: &str, file: &toml::Table, problems: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    for entry in entries(file, "type") {
        let (item, old, new, status) = (text(entry, "name"), text(entry, "old"), text(entry, "new"), text(entry, "status"));
        let at = format!("{name}: type `{item}`");
        keys_within(entry, &["name", "old", "new", "status", "note"], &at, problems);
        if item.is_empty() || !seen.insert(item.to_owned()) {
            problems.push(format!("{at}: the name is empty or repeated"));
        }
        if !is_path(old) || !is_path(new) {
            problems.push(format!("{at}: `old` and `new` are paths"));
        }
        let last = if new.is_empty() { old } else { new }.rsplit("::").next().unwrap_or_default();
        if !last.is_empty() && last != item {
            problems.push(format!("{at}: the paths end in `{last}`"));
        }
        let consistent = match status {
            "same" => !old.is_empty() && old == new,
            "moved" => !old.is_empty() && !new.is_empty() && old != new,
            "removed" => new.is_empty(),
            "replaced" => !text(entry, "note").is_empty(),
            "pending" => true,
            _ => false,
        };
        if !STATUSES.contains(&status) || !consistent {
            problems.push(format!("{at}: status `{status}` does not fit `old` and `new`"));
        }
    }
}

fn check_rewrites(name: &str, file: &toml::Table, problems: &mut Vec<String>) {
    for entry in entries(file, "call") {
        let at = format!("{name}: call `{}`", text(entry, "old"));
        keys_within(entry, &["old", "new", "note"], &at, problems);
        let (old, new) = (text(entry, "old"), text(entry, "new"));
        if !is_expression(old) || (!new.is_empty() && !is_expression(new)) {
            problems.push(format!("{at}: `old` and `new` are Rust expressions"));
        }
        if text(entry, "note").is_empty() {
            problems.push(format!("{at}: a call entry has a `note`"));
        }
    }
    for entry in entries(file, "token") {
        let at = format!("{name}: token `{}`", text(entry, "old"));
        keys_within(entry, &["old", "new", "note"], &at, problems);
        let dotted = |token: &str| token.split('.').all(|part| !part.is_empty() && part.chars().all(|c| c.is_alphanumeric() || c == '_'));
        if text(entry, "old").is_empty() || !dotted(text(entry, "old")) || (!text(entry, "new").is_empty() && !dotted(text(entry, "new"))) {
            problems.push(format!("{at}: `old` and `new` are dotted token names"));
        }
    }
    for entry in entries(file, "setter") {
        let at = format!("{name}: setter `{}`", text(entry, "old"));
        keys_within(entry, &["type", "old", "new", "note"], &at, problems);
        if text(entry, "type").is_empty() || text(entry, "old").is_empty() {
            problems.push(format!("{at}: a setter entry names its `type` and `old`"));
        }
    }
}

#[trial]
fn api_map_files_have_the_shape_the_codemods_read() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../migration/api-map");
    let mut problems = Vec::new();
    let mut count = 0;
    for entry in fs::read_dir(&dir).expect("migration/api-map").flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|extension| extension != "toml") {
            continue;
        }
        count += 1;
        let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        let file: toml::Table = match fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|t| t.parse().map_err(|e: toml::de::Error| e.to_string())) {
            Ok(file) => file,
            Err(error) => {
                problems.push(format!("{name}: {error}"));
                continue;
            }
        };
        keys_within(&file, &["family", "new_module", "type", "call", "token", "setter"], &name, &mut problems);
        if name != "common.toml" && (text(&file, "family").is_empty() || !is_path(text(&file, "new_module")) || !file.contains_key("new_module")) {
            problems.push(format!("{name}: a family file has `family` and a `new_module` path"));
        }
        check_types(&name, &file, &mut problems);
        check_rewrites(&name, &file, &mut problems);
    }
    assert!(count > 10, "the api-map directory has its files; found {count}");
    assert!(problems.is_empty(), "api-map problems:\n{}", problems.join("\n"));
}
