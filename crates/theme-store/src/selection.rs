//! `selection.json`: the suite's selection and the revisions apps reconcile against.

use std::fmt;

use craft_theme::{ContrastChoice, ModeChoice, Selection, Size, SuiteChoice, TextSize, ThemeDef, ThemeId, ThemePair, Variants};
use serde_json::{Map, Value};

/// The `selection.json` format this crate writes. Version 1 (one theme for both modes) is read
/// too and upgraded by the next write.
pub const SELECTION_SCHEMA_VERSION: u64 = 2;
/// Larger selection files are refused without being parsed.
pub const MAX_SELECTION_BYTES: usize = 64 * 1024;

/// `selection.json` as last read or written by this store. With feature `serde`, apps save it in
/// their preferences (`last_suite`) and pass it back with
/// [`StoreOptions::last_accepted`](crate::StoreOptions::last_accepted).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize), serde(default))]
pub struct Accepted {
    /// Empty until some app writes a choice: no file, no `suite` key, or no field in it.
    pub suite: SuiteChoice,
    /// Goes up by one with every write of `selection.json`.
    pub revision: u64,
    /// Goes up by one with every library change (import, delete).
    pub library_revision: u64,
}

impl Accepted {
    /// A state known from elsewhere, such as the copy an app keeps in its preferences.
    pub fn from_selection(suite: Selection) -> Self {
        Accepted { suite: suite.into(), revision: 0, library_revision: 0 }
    }
}

/// The fields of the suite choice the user changed. Only these are written; the fields it leaves
/// out keep what `selection.json` holds, chosen or not.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SelectionChange {
    pub theme_light: Option<ThemeId>,
    pub theme_dark: Option<ThemeId>,
    pub mode: Option<ModeChoice>,
    pub contrast: Option<ContrastChoice>,
    pub size: Option<Size>,
    pub text_size: Option<TextSize>,
}

impl SelectionChange {
    pub fn theme_light(theme: ThemeId) -> Self {
        SelectionChange { theme_light: Some(theme), ..SelectionChange::default() }
    }

    pub fn theme_dark(theme: ThemeId) -> Self {
        SelectionChange { theme_dark: Some(theme), ..SelectionChange::default() }
    }

    /// Both theme slots.
    pub fn themes(pair: ThemePair) -> Self {
        SelectionChange { theme_light: Some(pair.light), theme_dark: Some(pair.dark), ..SelectionChange::default() }
    }

    /// The user picked `theme`: both slots if it has both variants, else its own mode's slot, as
    /// [`SuiteChoice::pick_theme`].
    pub fn pick_theme(theme: &ThemeDef) -> Self {
        let id = || Some(theme.id.clone());
        match theme.variants {
            Variants::Both => SelectionChange { theme_light: id(), theme_dark: id(), ..SelectionChange::default() },
            Variants::LightOnly => SelectionChange { theme_light: id(), ..SelectionChange::default() },
            Variants::DarkOnly => SelectionChange { theme_dark: id(), ..SelectionChange::default() },
        }
    }

    pub fn mode(mode: ModeChoice) -> Self {
        SelectionChange { mode: Some(mode), ..SelectionChange::default() }
    }

    pub fn contrast(contrast: ContrastChoice) -> Self {
        SelectionChange { contrast: Some(contrast), ..SelectionChange::default() }
    }

    pub fn size(size: Size) -> Self {
        SelectionChange { size: Some(size), ..SelectionChange::default() }
    }

    pub fn text_size(text_size: TextSize) -> Self {
        SelectionChange { text_size: Some(text_size), ..SelectionChange::default() }
    }

    /// Every field, for a control that sets the whole selection. After a conflict, "Apply mine"
    /// resends the original change (returned in the conflict) with the disk state as base.
    pub fn whole(selection: Selection) -> Self {
        SelectionChange {
            theme_light: Some(selection.theme.light),
            theme_dark: Some(selection.theme.dark),
            mode: Some(selection.mode),
            contrast: Some(selection.contrast),
            size: Some(selection.size),
            text_size: Some(selection.text_size),
        }
    }

    /// This change followed by `later`: `later`'s fields replace this one's where set.
    pub fn then(self, later: SelectionChange) -> Self {
        SelectionChange {
            theme_light: later.theme_light.or(self.theme_light),
            theme_dark: later.theme_dark.or(self.theme_dark),
            mode: later.mode.or(self.mode),
            contrast: later.contrast.or(self.contrast),
            size: later.size.or(self.size),
            text_size: later.text_size.or(self.text_size),
        }
    }

    pub fn is_empty(&self) -> bool {
        *self == SelectionChange::default()
    }

    /// `choice` with this change applied: what the app shows until the write returns.
    pub fn applied_to(&self, choice: &SuiteChoice) -> SuiteChoice {
        SuiteChoice {
            theme_light: self.theme_light.clone().or_else(|| choice.theme_light.clone()),
            theme_dark: self.theme_dark.clone().or_else(|| choice.theme_dark.clone()),
            mode: self.mode.or(choice.mode),
            contrast: self.contrast.or(choice.contrast),
            size: self.size.or(choice.size),
            text_size: self.text_size.or(choice.text_size),
        }
    }

    /// The themes this change picks, light slot first.
    pub(crate) fn picked(&self) -> impl Iterator<Item = &ThemeId> {
        self.theme_light.iter().chain(&self.theme_dark)
    }
}

/// The choice to write, from `base` (the state the caller's change was made on), what is on
/// disk now and the change. `None` is a conflict.
///
/// A field the change leaves alone keeps disk's value, chosen or not, and never conflicts. A
/// changed field conflicts only when disk holds a chosen value other than mine and base's:
/// another app chose that field after the caller's read. A field unchosen on disk takes mine.
/// Each theme slot is a field of its own.
pub(crate) fn merge(base: &SuiteChoice, disk: &SuiteChoice, change: &SelectionChange) -> Option<SuiteChoice> {
    Some(SuiteChoice {
        theme_light: merge_field(base.theme_light.as_ref(), disk.theme_light.as_ref(), change.theme_light.as_ref())?,
        theme_dark: merge_field(base.theme_dark.as_ref(), disk.theme_dark.as_ref(), change.theme_dark.as_ref())?,
        mode: merge_field(base.mode.as_ref(), disk.mode.as_ref(), change.mode.as_ref())?,
        contrast: merge_field(base.contrast.as_ref(), disk.contrast.as_ref(), change.contrast.as_ref())?,
        size: merge_field(base.size.as_ref(), disk.size.as_ref(), change.size.as_ref())?,
        text_size: merge_field(base.text_size.as_ref(), disk.text_size.as_ref(), change.text_size.as_ref())?,
    })
}

fn merge_field<T: PartialEq + Clone>(base: Option<&T>, disk: Option<&T>, mine: Option<&T>) -> Option<Option<T>> {
    match mine {
        Some(mine) if disk.is_some() && disk != Some(mine) && disk != base => None,
        Some(mine) => Some(Some(mine.clone())),
        None => Some(disk.cloned()),
    }
}

/// The counters of a damaged `selection.json` that still parses as JSON, so the write that
/// replaces it never moves them backwards; `base` stands in for everything else.
pub(crate) fn salvage(bytes: Option<&[u8]>, base: &Accepted) -> Accepted {
    let value = bytes.and_then(|b| serde_json::from_slice::<Value>(b).ok());
    let counter = |key: &str| value.as_ref().and_then(|v| v.get(key)).and_then(Value::as_u64).unwrap_or(0);
    Accepted {
        suite: base.suite.clone(),
        revision: counter("revision").max(base.revision),
        library_revision: counter("library_revision").max(base.library_revision),
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
    let version = match raw.get("schema_version").and_then(Value::as_u64) {
        Some(v @ 1..=SELECTION_SCHEMA_VERSION) => v,
        Some(v) if v > SELECTION_SCHEMA_VERSION => return Err(SelectionError::TooNew { schema_version: v }),
        _ => return Err(corrupt("\"schema_version\" is missing or not 1 or 2")),
    };
    let revision = counter(&raw, "revision")?;
    let library_revision = counter(&raw, "library_revision")?;
    let suite = match raw.get("suite") {
        None | Some(Value::Null) => SuiteChoice::default(),
        Some(Value::Object(suite)) => parse_suite(suite, version)?,
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

/// Version 1 has one theme id for both modes; version 2 an object with a `light` and a `dark`
/// slot, each optional.
fn parse_suite(suite: &Map<String, Value>, version: u64) -> Result<SuiteChoice, SelectionError> {
    let (theme_light, theme_dark) = match suite.get("theme") {
        None | Some(Value::Null) => (None, None),
        Some(Value::String(s)) if version == 1 => {
            let id = theme_id(s, "suite.theme")?;
            (Some(id.clone()), Some(id))
        }
        Some(_) if version == 1 => return Err(corrupt("\"suite.theme\" is not a string")),
        Some(Value::Object(pair)) => (theme_slot(pair, "light")?, theme_slot(pair, "dark")?),
        Some(_) => return Err(corrupt("\"suite.theme\" is not an object")),
    };
    Ok(SuiteChoice {
        theme_light,
        theme_dark,
        mode: choice_field(suite, "mode", &MODES)?,
        contrast: choice_field(suite, "contrast", &CONTRASTS)?,
        size: choice_field(suite, "size", &SIZES)?,
        text_size: choice_field(suite, "text_size", &TEXT_SIZES)?,
    })
}

fn theme_slot(pair: &Map<String, Value>, key: &str) -> Result<Option<ThemeId>, SelectionError> {
    match pair.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => theme_id(s, &format!("suite.theme.{key}")).map(Some),
        Some(_) => Err(corrupt(&format!("\"suite.theme.{key}\" is not a string"))),
    }
}

fn theme_id(s: &str, at: &str) -> Result<ThemeId, SelectionError> {
    ThemeId::parse(s).map_err(|e| corrupt(&format!("\"{at}\": {e}")))
}

/// Each choice's name in the file, as craft-theme's serde writes it.
const MODES: [(ModeChoice, &str); 3] = [(ModeChoice::System, "system"), (ModeChoice::Light, "light"), (ModeChoice::Dark, "dark")];
const CONTRASTS: [(ContrastChoice, &str); 3] =
    [(ContrastChoice::System, "system"), (ContrastChoice::Standard, "standard"), (ContrastChoice::High, "high")];
const SIZES: [(Size, &str); 2] = [(Size::Compact, "compact"), (Size::Comfortable, "comfortable")];
const TEXT_SIZES: [(TextSize, &str); 4] =
    [(TextSize::Small, "small"), (TextSize::Default, "default"), (TextSize::Large, "large"), (TextSize::Larger, "larger")];

fn choice_field<T: Copy>(suite: &Map<String, Value>, key: &str, names: &[(T, &str)]) -> Result<Option<T>, SelectionError> {
    match suite.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_str().and_then(|s| names.iter().find(|(_, n)| *n == s)).map(|(c, _)| Some(*c)).ok_or_else(|| {
            let list: Vec<&str> = names.iter().map(|(_, n)| *n).collect();
            corrupt(&format!("\"suite.{key}\" is not {}", list.join(", ")))
        }),
    }
}

fn choice_name<T: PartialEq>(names: &[(T, &'static str)], value: &T) -> Value {
    names.iter().find(|(c, _)| c == value).map_or(Value::Null, |(_, n)| Value::from(*n))
}

fn corrupt(why: &str) -> SelectionError {
    SelectionError::Corrupt(why.to_owned())
}

/// `raw` (the object last read from disk, or empty) with the known fields set to `next`, as
/// version 2. Fields nobody chose are left out; with none chosen, `suite` stays as it was.
pub(crate) fn render_selection(raw: &mut Map<String, Value>, next: &Accepted, written_by: &str) -> Vec<u8> {
    raw.insert("schema_version".into(), SELECTION_SCHEMA_VERSION.into());
    raw.insert("revision".into(), next.revision.into());
    raw.insert("library_revision".into(), next.library_revision.into());
    let s = &next.suite;
    if !s.is_empty() {
        let mut suite = match raw.remove("suite") {
            Some(Value::Object(suite)) => suite,
            _ => Map::new(),
        };
        let theme = (s.theme_light.is_some() || s.theme_dark.is_some()).then(|| {
            // A version 1 string becomes the object; unknown keys in an object stay.
            let mut pair = match suite.remove("theme") {
                Some(Value::Object(pair)) => pair,
                _ => Map::new(),
            };
            set_or_remove(&mut pair, "light", s.theme_light.as_ref().map(|t| t.as_str().into()));
            set_or_remove(&mut pair, "dark", s.theme_dark.as_ref().map(|t| t.as_str().into()));
            Value::Object(pair)
        });
        set_or_remove(&mut suite, "theme", theme);
        set_or_remove(&mut suite, "mode", s.mode.map(|c| choice_name(&MODES, &c)));
        set_or_remove(&mut suite, "contrast", s.contrast.map(|c| choice_name(&CONTRASTS, &c)));
        set_or_remove(&mut suite, "size", s.size.map(|c| choice_name(&SIZES, &c)));
        set_or_remove(&mut suite, "text_size", s.text_size.map(|c| choice_name(&TEXT_SIZES, &c)));
        raw.insert("suite".into(), Value::Object(suite));
    }
    raw.insert("written_by".into(), written_by.into());
    let mut out = serde_json::to_vec_pretty(&*raw).unwrap_or_default();
    out.push(b'\n');
    out
}

fn set_or_remove(map: &mut Map<String, Value>, key: &str, value: Option<Value>) {
    match value {
        Some(v) => map.insert(key.into(), v),
        None => map.remove(key),
    };
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
    use serde_json::json;

    use super::*;

    fn id(s: &str) -> ThemeId {
        ThemeId::parse(s).unwrap()
    }

    fn sel(theme: &str, mode: ModeChoice) -> Selection {
        Selection { theme: ThemePair::same(id(theme)), mode, contrast: ContrastChoice::System, size: Size::Compact, text_size: TextSize::Default }
    }

    fn choice(theme: Option<&str>, mode: Option<ModeChoice>) -> SuiteChoice {
        SuiteChoice { theme_light: theme.map(id), theme_dark: theme.map(id), mode, ..SuiteChoice::default() }
    }

    fn both(theme: &str) -> SelectionChange {
        SelectionChange::themes(ThemePair::same(id(theme)))
    }

    #[test]
    fn merge_table() {
        let base = choice(Some("craft.artcraft"), Some(ModeChoice::System));
        let dark_disk = choice(Some("craft.artcraft"), Some(ModeChoice::Dark));
        // Only mine changed / another app changed the other field.
        assert_eq!(merge(&base, &base, &both("craft.graphite")), Some(choice(Some("craft.graphite"), Some(ModeChoice::System))));
        assert_eq!(merge(&base, &dark_disk, &both("craft.graphite")), Some(choice(Some("craft.graphite"), Some(ModeChoice::Dark))));
        // Same field: equal value is fine; a different value written elsewhere conflicts.
        assert_eq!(merge(&base, &dark_disk, &SelectionChange::mode(ModeChoice::Dark)), Some(dark_disk.clone()));
        assert_eq!(merge(&base, &dark_disk, &SelectionChange::mode(ModeChoice::Light)), None);
        // "Apply mine" resends the change on the disk state as base.
        assert_eq!(
            merge(&dark_disk, &dark_disk, &SelectionChange::mode(ModeChoice::Light)),
            Some(choice(Some("craft.artcraft"), Some(ModeChoice::Light)))
        );
        // Nothing chosen: a change writes only its own field.
        let none = SuiteChoice::default();
        assert_eq!(merge(&none, &none, &both("craft.graphite")), Some(choice(Some("craft.graphite"), None)));
        // Two apps on a fresh install, one choosing the theme, the other the mode: no conflict.
        let theme_only = choice(Some("craft.graphite"), None);
        assert_eq!(
            merge(&none, &theme_only, &SelectionChange::mode(ModeChoice::Light)),
            Some(choice(Some("craft.graphite"), Some(ModeChoice::Light)))
        );
        // Another app chose the same field first: equal is fine, different conflicts.
        assert_eq!(merge(&none, &theme_only, &both("craft.graphite")), Some(theme_only.clone()));
        assert_eq!(merge(&none, &theme_only, &both("craft.artcraft")), None);
        // A file that lost a field: the change writes into it.
        assert_eq!(merge(&base, &none, &SelectionChange::mode(ModeChoice::Dark)), Some(choice(None, Some(ModeChoice::Dark))));
    }

    #[test]
    fn each_new_field_merges_on_its_own() {
        let base = choice(Some("craft.artcraft"), None);
        // Another app set the dark slot, contrast and text size; this one the light slot and size.
        let disk = SuiteChoice {
            theme_dark: Some(id("craft.pro-dark")),
            contrast: Some(ContrastChoice::High),
            text_size: Some(TextSize::Large),
            ..base.clone()
        };
        let mine = SelectionChange::theme_light(id("craft.pro-light")).then(SelectionChange::size(Size::Comfortable));
        let merged = merge(&base, &disk, &mine).unwrap();
        assert_eq!(
            merged,
            SuiteChoice {
                theme_light: Some(id("craft.pro-light")),
                theme_dark: Some(id("craft.pro-dark")),
                mode: None,
                contrast: Some(ContrastChoice::High),
                size: Some(Size::Comfortable),
                text_size: Some(TextSize::Large),
            }
        );
        // The same field changed elsewhere conflicts, one field at a time.
        for change in [
            SelectionChange::theme_dark(id("craft.studio")),
            SelectionChange::contrast(ContrastChoice::Standard),
            SelectionChange::text_size(TextSize::Small),
        ] {
            assert_eq!(merge(&base, &disk, &change), None, "{change:?}");
        }
        let sized = SuiteChoice { size: Some(Size::Compact), ..base.clone() };
        assert_eq!(merge(&base, &sized, &SelectionChange::size(Size::Comfortable)), None);
    }

    #[test]
    fn picking_a_single_variant_theme_changes_one_slot() {
        let builtin = |name: &str| craft_theme::builtins().iter().find(|t| t.id.as_str() == name).unwrap();
        let soft = SelectionChange::pick_theme(builtin("craft.soft-light"));
        assert_eq!(soft, SelectionChange::theme_light(id("craft.soft-light")));
        assert_eq!(SelectionChange::pick_theme(builtin("craft.graphite")), both("craft.graphite"));
        let shown = soft.applied_to(&choice(Some("craft.graphite"), None));
        assert_eq!((shown.theme_light, shown.theme_dark), (Some(id("craft.soft-light")), Some(id("craft.graphite"))));
        assert_eq!(soft.picked().collect::<Vec<_>>(), [&id("craft.soft-light")]);
    }

    #[test]
    fn damaged_counters_are_salvaged() {
        let base = Accepted { suite: choice(Some("craft.graphite"), None), revision: 3, library_revision: 9 };
        let bytes = br#"{"schema_version":2,"revision":41,"library_revision":2,"suite":{"theme":5}}"#;
        assert_eq!(salvage(Some(bytes), &base), Accepted { suite: base.suite.clone(), revision: 41, library_revision: 9 });
        assert_eq!(salvage(Some(b"{ not json"), &base), base);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn accepted_round_trips_through_serde() {
        let suite =
            SuiteChoice { theme_dark: Some(id("user.navy")), mode: Some(ModeChoice::Dark), size: Some(Size::Comfortable), ..SuiteChoice::default() };
        let accepted = Accepted { suite, revision: 4, library_revision: 1 };
        let json = serde_json::to_string(&accepted).unwrap();
        assert_eq!(serde_json::from_str::<Accepted>(&json).unwrap(), accepted);
        assert_eq!(serde_json::from_str::<Accepted>("{}").unwrap(), Accepted::default());
    }

    #[test]
    fn unknown_keys_survive_a_rewrite() {
        let text = br#"{"schema_version":2,"revision":4,"library_revision":2,"extra":{"a":[1]},
            "suite":{"theme":{"light":"user.navy","dark":"user.navy","later":1},"mode":"light","later":true},"written_by":"x"}"#;
        let doc = parse_selection(text).unwrap();
        assert_eq!(doc.accepted.revision, 4);
        assert_eq!(doc.accepted.suite, choice(Some("user.navy"), Some(ModeChoice::Light)));
        let whole = Selection {
            contrast: ContrastChoice::High,
            size: Size::Comfortable,
            text_size: TextSize::Larger,
            ..sel("craft.graphite", ModeChoice::Dark)
        };
        let next = Accepted { suite: whole.into(), revision: 5, library_revision: 2 };
        let mut raw = doc.raw;
        let out = render_selection(&mut raw, &next, "test 1.0");
        let back: Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(back["extra"], json!({"a": [1]}));
        assert_eq!(back["suite"]["later"], true);
        assert_eq!(
            back["suite"],
            json!({
                "theme": {"light": "craft.graphite", "dark": "craft.graphite", "later": 1},
                "mode": "dark", "contrast": "high", "size": "comfortable", "text_size": "larger", "later": true
            })
        );
        assert_eq!(back["written_by"], "test 1.0");
        assert_eq!(parse_selection(&out).unwrap().accepted, next);
    }

    #[cfg(feature = "serde")]
    #[test]
    fn choice_names_match_craft_themes_serde() {
        for (c, name) in MODES {
            assert_eq!(serde_json::to_value(c).unwrap(), name);
        }
        for (c, name) in CONTRASTS {
            assert_eq!(serde_json::to_value(c).unwrap(), name);
        }
        for (c, name) in SIZES {
            assert_eq!(serde_json::to_value(c).unwrap(), name);
        }
        for (c, name) in TEXT_SIZES {
            assert_eq!(serde_json::to_value(c).unwrap(), name);
        }
    }

    #[test]
    fn version_1_is_read_into_both_slots_and_upgraded_on_write() {
        let v1 = br#"{"schema_version":1,"revision":6,"library_revision":1,"suite":{"theme":"user.navy","mode":"system","x":2}}"#;
        let doc = parse_selection(v1).unwrap();
        assert_eq!(doc.accepted.suite, choice(Some("user.navy"), Some(ModeChoice::System)));
        let next =
            Accepted { suite: SelectionChange::theme_dark(id("craft.pro-dark")).applied_to(&doc.accepted.suite), revision: 7, library_revision: 1 };
        let mut raw = doc.raw;
        let out = render_selection(&mut raw, &next, "t");
        let back: Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(back["schema_version"], 2);
        assert_eq!(back["suite"], json!({"theme": {"light": "user.navy", "dark": "craft.pro-dark"}, "mode": "system", "x": 2}));
        assert_eq!(parse_selection(&out).unwrap().accepted, next);
        // Each version's own theme form only.
        assert!(matches!(parse_selection(br#"{"schema_version":1,"suite":{"theme":{"light":"user.navy"}}}"#), Err(SelectionError::Corrupt(_))));
        assert!(matches!(parse_selection(br#"{"schema_version":2,"suite":{"theme":"user.navy"}}"#), Err(SelectionError::Corrupt(_))));
    }

    #[test]
    fn no_suite_key_means_no_selection_and_bad_fields_are_corrupt() {
        let bare = parse_selection(br#"{"schema_version":2,"library_revision":3}"#).unwrap();
        assert_eq!(bare.accepted, Accepted { suite: SuiteChoice::default(), revision: 0, library_revision: 3 });
        let mut raw = bare.raw;
        let out = render_selection(&mut raw, &Accepted { suite: SuiteChoice::default(), revision: 1, library_revision: 4 }, "t");
        assert!(serde_json::from_slice::<Value>(&out).unwrap().get("suite").is_none());
        // One field chosen: the others are left out, and so is an unchosen slot.
        let mode_only = parse_selection(br#"{"schema_version":2,"suite":{"mode":"dark"}}"#).unwrap();
        assert_eq!(mode_only.accepted.suite, choice(None, Some(ModeChoice::Dark)));
        let mut raw = mode_only.raw;
        let light_only = SuiteChoice { theme_light: Some(id("craft.soft-light")), ..SuiteChoice::default() };
        let out = render_selection(&mut raw, &Accepted { suite: light_only, revision: 1, library_revision: 0 }, "t");
        let written: Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(written["suite"], json!({"theme": {"light": "craft.soft-light"}}));
        for bad in [
            &br#"{"revision":1}"#[..],
            b"[1]",
            b"{",
            br#"{"schema_version":2,"suite":{"mode":"auto"}}"#,
            br#"{"schema_version":2,"suite":{"contrast":"max"}}"#,
            br#"{"schema_version":2,"suite":{"size":1}}"#,
            br#"{"schema_version":2,"suite":{"text_size":"huge"}}"#,
            br#"{"schema_version":2,"suite":{"theme":{"light":"A"}}}"#,
            br#"{"schema_version":2,"suite":{"theme":{"dark":7}}}"#,
            br#"{"schema_version":1,"suite":{"theme":"craft.graphite","mode":"auto"}}"#,
            br#"{"schema_version":1,"suite":{"theme":7}}"#,
        ] {
            assert!(matches!(parse_selection(bad), Err(SelectionError::Corrupt(_))), "{}", String::from_utf8_lossy(bad));
        }
        assert_eq!(parse_selection(br#"{"schema_version":3,"suite":7}"#).unwrap_err(), SelectionError::TooNew { schema_version: 3 });
    }
}
