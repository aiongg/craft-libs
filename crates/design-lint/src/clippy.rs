//! The clippy side: check that an app's `clippy.toml` carries the template
//! entries and merge them in.
//!
//! Entries are matched by `path`. The template owns the entries it lists; any
//! other entry in the app's arrays is kept after them, comments included.

use std::fmt;
use std::path::{Path, PathBuf};

use toml_edit::{Array, DocumentMut, Item, Value};

use crate::{CLIPPY_TEMPLATE, Error};

const KEYS: [&str; 2] = ["disallowed-methods", "disallowed-types"];
const BEGIN: &str = "BEGIN craft-design-system";
const END: &str = "END craft-design-system";
const TEMPLATE_ORIGIN: &str = "clippy.template.toml";

/// How an app's entry differs from the template's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DriftKind {
    /// The app has no entry with the template's path.
    Missing,
    /// The app's entry has a different `reason` or `allow-invalid`.
    Differs,
}

/// One template entry the app's `clippy.toml` lacks or has changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClippyDrift {
    /// `disallowed-methods` or `disallowed-types`.
    pub key: &'static str,
    /// The entry's `path`.
    pub path: String,
    pub kind: DriftKind,
}

impl fmt::Display for ClippyDrift {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (id, what) = match self.kind {
            DriftKind::Missing => ("clippy/missing", "is missing"),
            DriftKind::Differs => ("clippy/differs", "differs from the template"),
        };
        write!(f, "clippy.toml: error[{id}]: {} entry `{}` {what}", self.key, self.path)
    }
}

/// Fails with [`Error::ClippyDrift`] unless the `clippy.toml` at `clippy_toml`
/// (or in that directory) carries every template entry. A missing file counts
/// as empty.
pub fn check_clippy(clippy_toml: impl AsRef<Path>) -> Result<(), Error> {
    let path = resolve(clippy_toml.as_ref());
    let drift = clippy_drift(&path, &read_or_empty(&path)?)?;
    if drift.is_empty() { Ok(()) } else { Err(Error::ClippyDrift(drift)) }
}

/// Merges the template into the `clippy.toml` at `clippy_toml` (or in that
/// directory), creating it when absent. Returns whether the file changed.
pub fn write_clippy(clippy_toml: impl AsRef<Path>) -> Result<bool, Error> {
    let path = resolve(clippy_toml.as_ref());
    let old = read_or_empty(&path)?;
    let new = merge_clippy(&path, &old)?;
    if new == old {
        return Ok(false);
    }
    std::fs::write(&path, new).map_err(|source| Error::Io { path, source })?;
    Ok(true)
}

/// The template entries that `text` (the content of the `clippy.toml` at `origin`) lacks or has changed.
pub fn clippy_drift(origin: &Path, text: &str) -> Result<Vec<ClippyDrift>, Error> {
    let doc = parse(origin, text)?;
    let template = template_doc()?;
    let mut drift = Vec::new();
    for key in KEYS {
        let have = doc.get(key).and_then(Item::as_array);
        for entry in template_entries(&template, key)? {
            let path = entry_path(entry).unwrap_or_default();
            let found = have.and_then(|a| a.iter().find(|v| entry_path(v) == Some(path)));
            let kind = match found {
                None => DriftKind::Missing,
                Some(v) if same_entry(v, entry) => continue,
                Some(_) => DriftKind::Differs,
            };
            drift.push(ClippyDrift { key, path: path.to_owned(), kind });
        }
    }
    Ok(drift)
}

/// `text` (the content of the `clippy.toml` at `origin`) with the template
/// merged in: per array, the template entries first, then the app's other
/// entries. Everything else in the file is untouched. Merging twice gives the
/// same text.
pub fn merge_clippy(origin: &Path, text: &str) -> Result<String, Error> {
    let mut doc = parse(origin, text)?;
    let template = template_doc()?;
    for key in KEYS {
        let wanted = template_entries(&template, key)?;
        let mut merged = Array::new();
        for entry in &wanted {
            merged.push_formatted((*entry).clone());
        }
        if let Some(item) = doc.get(key) {
            let have = item.as_array().ok_or_else(|| toml_error(origin, format!("`{key}` is not an array")))?;
            for value in have {
                if !wanted.iter().any(|w| entry_path(w) == entry_path(value) && entry_path(value).is_some()) {
                    let mut extra = value.clone();
                    if !extra.decor().prefix().and_then(|p| p.as_str()).is_some_and(|p| p.contains('\n')) {
                        extra.decor_mut().set_prefix("\n    ");
                    }
                    merged.push_formatted(extra);
                }
            }
        }
        merged.set_trailing("\n");
        merged.set_trailing_comma(true);
        doc[key] = Item::Value(Value::Array(merged));
    }
    add_markers(&mut doc, &template);
    Ok(doc.to_string())
}

/// Puts the template's header comment before `disallowed-methods` and its
/// closing comment at the end, unless the file has them.
fn add_markers(doc: &mut DocumentMut, template: &DocumentMut) {
    let text = doc.to_string();
    if !text.contains(BEGIN) {
        let begin = template.key("disallowed-methods").and_then(|k| k.leaf_decor().prefix()).and_then(|p| p.as_str()).unwrap_or_default().to_owned();
        if let Some(mut key) = doc.key_mut("disallowed-methods") {
            let old = key.leaf_decor().prefix().and_then(|p| p.as_str()).unwrap_or_default().to_owned();
            key.leaf_decor_mut().set_prefix(format!("{old}{begin}"));
        }
    }
    if !text.contains(END) {
        let end = template.trailing().as_str().unwrap_or_default();
        let old = doc.trailing().as_str().unwrap_or_default().to_owned();
        doc.set_trailing(format!("{old}{end}"));
    }
}

fn resolve(path: &Path) -> PathBuf {
    if path.is_dir() { path.join("clippy.toml") } else { path.to_owned() }
}

fn read_or_empty(path: &Path) -> Result<String, Error> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(source) => Err(Error::Io { path: path.to_owned(), source }),
    }
}

fn parse(origin: &Path, text: &str) -> Result<DocumentMut, Error> {
    text.parse().map_err(|e: toml_edit::TomlError| toml_error(origin, e.to_string()))
}

fn toml_error(origin: &Path, message: String) -> Error {
    Error::Toml { path: origin.to_owned(), message }
}

fn template_doc() -> Result<DocumentMut, Error> {
    parse(Path::new(TEMPLATE_ORIGIN), CLIPPY_TEMPLATE)
}

fn template_entries<'a>(template: &'a DocumentMut, key: &str) -> Result<Vec<&'a Value>, Error> {
    let array = template.get(key).and_then(Item::as_array).ok_or_else(|| toml_error(Path::new(TEMPLATE_ORIGIN), format!("no `{key}` array")))?;
    Ok(array.iter().collect())
}

/// The `path` of an entry, written as a table or as a bare string.
fn entry_path(entry: &Value) -> Option<&str> {
    match entry {
        Value::String(s) => Some(s.value()),
        Value::InlineTable(t) => t.get("path")?.as_str(),
        _ => None,
    }
}

fn same_entry(have: &Value, want: &Value) -> bool {
    let field = |v: &Value, name: &str| match v {
        Value::InlineTable(t) => t.get(name).map(|x| x.to_string().trim().to_owned()),
        _ => None,
    };
    ["reason", "allow-invalid"].iter().all(|name| field(have, name) == field(want, name))
}
