//! `selection.json`: the suite's selection and the revisions apps reconcile against.

use std::fmt;

use craft_theme::{Mode, Selection, ThemeId};
use serde_json::{Map, Value};

/// The `selection.json` format this crate reads and writes.
pub const SELECTION_SCHEMA_VERSION: u64 = 1;
/// Larger selection files are refused without being parsed.
pub const MAX_SELECTION_BYTES: usize = 64 * 1024;

/// `selection.json` as last read or written by this store.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Accepted {
    /// `None` until some app writes a suite selection: no file, or a file without `suite`.
    pub suite: Option<Selection>,
    /// Goes up by one with every write of `selection.json`.
    pub revision: u64,
    /// Goes up by one with every library change (import, delete).
    pub library_revision: u64,
}

impl Accepted {
    /// A state known from elsewhere, such as the copy an app keeps in its preferences.
    pub fn from_selection(suite: Selection) -> Self {
        Accepted { suite: Some(suite), revision: 0, library_revision: 0 }
    }
}

/// The suite selection fields the user changed, over the selection the app showed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionChange {
    /// What the app showed when the user made the change: the suite selection it followed, or
    /// its own default when no app had written one. It fills the fields this change leaves alone
    /// when `selection.json` has no selection yet.
    pub shown: Selection,
    pub theme: Option<ThemeId>,
    pub mode: Option<Mode>,
}

impl SelectionChange {
    /// No field changed yet; add them with [`with_theme`](Self::with_theme) and
    /// [`with_mode`](Self::with_mode).
    pub fn new(shown: Selection) -> Self {
        SelectionChange { shown, theme: None, mode: None }
    }

    /// Both fields set to `selection`, such as "Apply mine" after a conflict.
    pub fn whole(selection: Selection) -> Self {
        SelectionChange { theme: Some(selection.theme.clone()), mode: Some(selection.mode), shown: selection }
    }

    pub fn with_theme(mut self, theme: ThemeId) -> Self {
        self.theme = Some(theme);
        self
    }

    pub fn with_mode(mut self, mode: Mode) -> Self {
        self.mode = Some(mode);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.theme.is_none() && self.mode.is_none()
    }

    /// The shown selection with the change applied: what the app shows until the write returns.
    pub fn wanted(&self) -> Selection {
        Selection { theme: self.theme.clone().unwrap_or_else(|| self.shown.theme.clone()), mode: self.mode.unwrap_or(self.shown.mode) }
    }
}

/// The selection to write, from `base` (the state the caller's change was made on), what is on
/// disk now and the change. `None` is a conflict.
///
/// A field the change leaves alone takes disk's value, or the shown value when disk has no
/// selection; it never conflicts. A changed field conflicts only when disk holds a different
/// value that another app wrote after the caller's read: disk's value differs from base's, or,
/// with no base selection, disk's revision is past base's.
pub(crate) fn merge(base: &Accepted, disk: &Accepted, change: &SelectionChange) -> Option<Selection> {
    let Some(on_disk) = &disk.suite else {
        return Some(change.wanted());
    };
    let raced = disk.revision > base.revision;
    let base = base.suite.as_ref();
    Some(Selection {
        theme: merge_field(base.map(|b| &b.theme), &on_disk.theme, change.theme.as_ref(), raced)?,
        mode: merge_field(base.map(|b| &b.mode), &on_disk.mode, change.mode.as_ref(), raced)?,
    })
}

fn merge_field<T: PartialEq + Clone>(base: Option<&T>, disk: &T, mine: Option<&T>, raced: bool) -> Option<T> {
    match mine {
        Some(mine) if mine != disk => {
            let written_elsewhere = base.map_or(raced, |base| base != disk);
            (!written_elsewhere).then(|| mine.clone())
        }
        _ => Some(disk.clone()),
    }
}

/// Why `selection.json` couldn't be used.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectionError {
    /// Not JSON, not an object, or a known key with a value of the wrong type.
    Corrupt(String),
    /// Written by a newer build; never overwritten.
    TooNew {
        schema_version: u64,
    },
    TooLarge {
        len: usize,
    },
}

/// A parsed `selection.json`: the known fields and the whole object, so that keys this build
/// doesn't know are written back.
#[derive(Clone, Debug, Default)]
pub(crate) struct SelectionDoc {
    pub raw: Map<String, Value>,
    pub accepted: Accepted,
}

pub(crate) fn parse_selection(bytes: &[u8]) -> Result<SelectionDoc, SelectionError> {
    if bytes.len() > MAX_SELECTION_BYTES {
        return Err(SelectionError::TooLarge { len: bytes.len() });
    }
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let value: Value = serde_json::from_slice(bytes).map_err(|e| SelectionError::Corrupt(e.to_string()))?;
    let Value::Object(raw) = value else {
        return Err(corrupt("not a JSON object"));
    };
    match raw.get("schema_version").and_then(Value::as_u64) {
        Some(SELECTION_SCHEMA_VERSION) => {}
        Some(v) if v > SELECTION_SCHEMA_VERSION => return Err(SelectionError::TooNew { schema_version: v }),
        _ => return Err(corrupt("\"schema_version\" is missing or not 1")),
    }
    let revision = counter(&raw, "revision")?;
    let library_revision = counter(&raw, "library_revision")?;
    let suite = match raw.get("suite") {
        None | Some(Value::Null) => None,
        Some(Value::Object(suite)) => Some(parse_suite(suite)?),
        Some(_) => return Err(corrupt("\"suite\" is not an object")),
    };
    Ok(SelectionDoc { accepted: Accepted { suite, revision, library_revision }, raw })
}

fn counter(raw: &Map<String, Value>, key: &str) -> Result<u64, SelectionError> {
    match raw.get(key) {
        None => Ok(0),
        Some(v) => v.as_u64().ok_or_else(|| corrupt(&format!("\"{key}\" is not a whole number"))),
    }
}

fn parse_suite(suite: &Map<String, Value>) -> Result<Selection, SelectionError> {
    let theme = match suite.get("theme") {
        Some(Value::String(s)) => ThemeId::parse(s).map_err(|e| corrupt(&format!("\"suite.theme\": {e}")))?,
        _ => return Err(corrupt("\"suite.theme\" is missing or not a string")),
    };
    let mode = suite
        .get("mode")
        .and_then(Value::as_str)
        .and_then(mode_from_key)
        .ok_or_else(|| corrupt("\"suite.mode\" is missing or not system, light or dark"))?;
    Ok(Selection { theme, mode })
}

fn corrupt(why: &str) -> SelectionError {
    SelectionError::Corrupt(why.to_owned())
}

pub(crate) fn mode_from_key(s: &str) -> Option<Mode> {
    match s {
        "system" => Some(Mode::System),
        "light" => Some(Mode::Light),
        "dark" => Some(Mode::Dark),
        _ => None,
    }
}

pub(crate) fn mode_key(mode: Mode) -> &'static str {
    match mode {
        Mode::System => "system",
        Mode::Light => "light",
        Mode::Dark => "dark",
    }
}

/// `raw` (the object last read from disk, or empty) with the known fields set to `next`. With
/// no suite selection, no `suite` key is written.
pub(crate) fn render_selection(raw: &mut Map<String, Value>, next: &Accepted, written_by: &str) -> Vec<u8> {
    raw.insert("schema_version".into(), SELECTION_SCHEMA_VERSION.into());
    raw.insert("revision".into(), next.revision.into());
    raw.insert("library_revision".into(), next.library_revision.into());
    if let Some(sel) = &next.suite {
        let mut suite = match raw.remove("suite") {
            Some(Value::Object(suite)) => suite,
            _ => Map::new(),
        };
        suite.insert("theme".into(), sel.theme.as_str().into());
        suite.insert("mode".into(), mode_key(sel.mode).into());
        raw.insert("suite".into(), Value::Object(suite));
    }
    raw.insert("written_by".into(), written_by.into());
    let mut out = serde_json::to_vec_pretty(&*raw).unwrap_or_default();
    out.push(b'\n');
    out
}

impl fmt::Display for SelectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SelectionError::Corrupt(why) => write!(f, "the suite theme selection file is damaged: {why}"),
            SelectionError::TooNew { schema_version } => write!(
                f,
                "the suite theme selection was saved by a newer Craft app (format {schema_version}); this app won't change it until it is updated"
            ),
            SelectionError::TooLarge { len } => {
                write!(f, "the suite theme selection file is {} KiB; at most {} KiB is read", len.div_ceil(1024), MAX_SELECTION_BYTES / 1024)
            }
        }
    }
}

impl std::error::Error for SelectionError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn sel(theme: &str, mode: Mode) -> Selection {
        Selection { theme: ThemeId::parse(theme).unwrap(), mode }
    }

    fn state(suite: Option<Selection>, revision: u64) -> Accepted {
        Accepted { suite, revision, library_revision: 0 }
    }

    #[test]
    fn merge_table() {
        let base_sel = sel("craft.artcraft", Mode::System);
        let base = state(Some(base_sel.clone()), 3);
        let to_graphite = SelectionChange::new(base_sel.clone()).with_theme(ThemeId::parse("craft.graphite").unwrap());
        let to_dark = SelectionChange::new(base_sel.clone()).with_mode(Mode::Dark);
        let to_light = SelectionChange::new(base_sel.clone()).with_mode(Mode::Light);
        let dark_disk = state(Some(sel("craft.artcraft", Mode::Dark)), 4);
        // Only mine changed / another app changed the other field.
        assert_eq!(merge(&base, &base, &to_graphite), Some(sel("craft.graphite", Mode::System)));
        assert_eq!(merge(&base, &dark_disk, &to_graphite), Some(sel("craft.graphite", Mode::Dark)));
        // An untouched field never conflicts, whatever disk holds.
        assert_eq!(merge(&base, &state(Some(sel("craft.graphite", Mode::System)), 4), &to_dark), Some(sel("craft.graphite", Mode::Dark)));
        // Same field: equal value is fine; a different value written elsewhere conflicts.
        assert_eq!(merge(&base, &dark_disk, &to_dark), Some(sel("craft.artcraft", Mode::Dark)));
        assert_eq!(merge(&base, &dark_disk, &to_light), None);
        // "Apply mine" repeats the change on the disk state as base.
        assert_eq!(merge(&dark_disk, &dark_disk, &to_light), Some(sel("craft.artcraft", Mode::Light)));
        // No selection on disk: the shown selection with the change, whatever the base.
        let fresh = state(None, 0);
        assert_eq!(merge(&fresh, &fresh, &to_dark), Some(sel("craft.artcraft", Mode::Dark)));
        assert_eq!(merge(&base, &state(None, 5), &to_dark), Some(sel("craft.artcraft", Mode::Dark)));
        // No base selection, another app wrote first: only changed fields with another value
        // conflict; a mode-only change takes the theme from disk.
        let shown_default = SelectionChange::new(sel("craft.high-contrast", Mode::Dark));
        let first = state(Some(sel("craft.graphite", Mode::System)), 1);
        assert_eq!(merge(&fresh, &first, &shown_default.clone().with_mode(Mode::Light)), None);
        assert_eq!(merge(&fresh, &first, &shown_default.clone().with_mode(Mode::System)), Some(sel("craft.graphite", Mode::System)));
        assert_eq!(
            merge(&fresh, &first, &shown_default.clone().with_theme(ThemeId::parse("craft.graphite").unwrap())),
            Some(sel("craft.graphite", Mode::System))
        );
        // Not raced: disk's revision is not past base's (base came from the same file).
        assert_eq!(merge(&state(None, 1), &first, &shown_default.with_mode(Mode::Light)), Some(sel("craft.graphite", Mode::Light)));
    }

    #[test]
    fn unknown_keys_survive_a_rewrite() {
        let text = br#"{"schema_version":1,"revision":4,"library_revision":2,"extra":{"a":[1]},
            "suite":{"theme":"user.navy","mode":"light","later":true},"written_by":"x"}"#;
        let doc = parse_selection(text).unwrap();
        assert_eq!(doc.accepted.revision, 4);
        assert_eq!(doc.accepted.suite, Some(sel("user.navy", Mode::Light)));
        let next = Accepted { suite: Some(sel("craft.graphite", Mode::Dark)), revision: 5, library_revision: 2 };
        let mut raw = doc.raw;
        let out = render_selection(&mut raw, &next, "test 1.0");
        let back: Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(back["extra"], serde_json::json!({"a": [1]}));
        assert_eq!(back["suite"]["later"], true);
        assert_eq!(back["written_by"], "test 1.0");
        assert_eq!(parse_selection(&out).unwrap().accepted, next);
    }

    #[test]
    fn no_suite_key_means_no_selection_and_bad_fields_are_corrupt() {
        let bare = parse_selection(br#"{"schema_version":1,"library_revision":3}"#).unwrap();
        assert_eq!(bare.accepted, Accepted { suite: None, revision: 0, library_revision: 3 });
        let mut raw = bare.raw;
        let out = render_selection(&mut raw, &Accepted { suite: None, revision: 1, library_revision: 4 }, "t");
        assert!(serde_json::from_slice::<Value>(&out).unwrap().get("suite").is_none());
        for bad in [
            &br#"{"revision":1}"#[..],
            b"[1]",
            b"{",
            br#"{"schema_version":1,"suite":{"theme":"craft.graphite","mode":"auto"}}"#,
            br#"{"schema_version":1,"suite":{"theme":"A","mode":"dark"}}"#,
            br#"{"schema_version":1,"suite":{"mode":"dark"}}"#,
        ] {
            assert!(matches!(parse_selection(bad), Err(SelectionError::Corrupt(_))), "{}", String::from_utf8_lossy(bad));
        }
        assert_eq!(parse_selection(br#"{"schema_version":2,"suite":7}"#).unwrap_err(), SelectionError::TooNew { schema_version: 2 });
    }
}
