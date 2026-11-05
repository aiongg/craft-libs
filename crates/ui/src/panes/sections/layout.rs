//! `SectionKey`, `SectionState`, `SectionLayout` and `SectionEvent`: the user's arrangement of a
//! sidebar's sections, made whole whatever is read.

use std::hash::Hash;

/// The app's sections (LightCraft's `SidebarSection`). Titles, counts and hints are in
/// [`SectionInfo`](super::SectionInfo), so translations change at run time; the key carries ids
/// only.
pub trait SectionKey: Copy + Eq + Hash + Send + Sync + 'static {
    /// Every section, in the usual order.
    const ALL: &'static [Self];
    /// The id saved and used in widget ids (`"albums"`).
    fn id(self) -> &'static str;
    /// The section with id `id`.
    fn from_id(id: &str) -> Option<Self>;
}

/// One section's place in the arrangement.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SectionState<S> {
    /// The section.
    pub section: S,
    /// Hidden from the sidebar (Hide This Section).
    pub hidden: bool,
    /// Folded shut.
    pub folded: bool,
}

impl<S> SectionState<S> {
    const fn new(section: S) -> Self {
        Self { section, hidden: false, folded: false }
    }
}

/// What [`ReorderableSections`](super::ReorderableSections) reported, in order. View state: never
/// an undo step. The host applies them ([`SectionLayout::apply`]) or routes them through its
/// commands (LightCraft's `view.sidebar*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum SectionEvent<S> {
    /// Move `section` before `before` (`None`: to the end). From a drag, Cmd/Ctrl+] and [, or Move
    /// Up and Move Down.
    Moved {
        /// The section moved.
        section: S,
        /// The section it now precedes; `None` for the end.
        before: Option<S>,
    },
    /// Hide This Section, or a check item turned off.
    Hidden(S),
    /// A check item turned on.
    Shown(S),
    /// The header folded or unfolded.
    Folded {
        /// The section whose header was pressed.
        section: S,
        /// Whether the section is now folded.
        folded: bool,
    },
    /// The header button with `id` was pressed.
    Button {
        /// The section whose header holds the button.
        section: S,
        /// The button's id.
        id: &'static str,
    },
    /// Reset: the usual order, every section shown and open.
    Reset,
}

/// The arrangement of a sidebar's sections, top to bottom, holding every section of `S::ALL`
/// exactly once whatever was read: unknown and repeated entries are dropped, sections not named
/// come last in the usual order (LightCraft's `SidebarLayout`, lead call 42).
///
/// Serde: written as a list of `{ id, hidden, folded }`. Read leniently from any self-describing
/// format (TOML, JSON): a list whose entries are an id string or a map with `id` and optional
/// `hidden`, `folded` (also `collapsed`, LightCraft's name); a flag that is not a bool is off; an
/// entry without a known id is dropped; anything that is not a list is the usual arrangement. A
/// file always opens. Strict parsing for agents stays in the app (`from_states` with its own
/// errors).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionLayout<S>(Vec<SectionState<S>>);

impl<S: SectionKey> Default for SectionLayout<S> {
    fn default() -> Self {
        // lightcraft: crates/ui-egui/src/sidebar.rs:96 (`Default`: every section open, usual order).
        Self::from_states([])
    }
}

impl<S: SectionKey> SectionLayout<S> {
    /// The layout these states describe, made whole (see the type).
    pub fn from_states(states: impl IntoIterator<Item = SectionState<S>>) -> Self {
        // lightcraft: crates/ui-egui/src/sidebar.rs:150 (`from_states`).
        let mut all: Vec<SectionState<S>> = Vec::with_capacity(S::ALL.len());
        for state in states.into_iter().chain(S::ALL.iter().map(|&section| SectionState::new(section))) {
            if !all.iter().any(|s| s.section == state.section) {
                all.push(state);
            }
        }
        Self(all)
    }

    /// Every section top to bottom, hidden ones included.
    pub fn states(&self) -> &[SectionState<S>] {
        // lightcraft: crates/ui-egui/src/sidebar.rs:161 (`sections`).
        &self.0
    }

    /// The shown sections, top to bottom.
    pub fn shown(&self) -> impl Iterator<Item = S> + '_ {
        // lightcraft: crates/ui-egui/src/sidebar.rs:166 (`shown`).
        self.0.iter().filter(|s| !s.hidden).map(|s| s.section)
    }

    /// Whether `section` is hidden.
    pub fn is_hidden(&self, section: S) -> bool {
        // lightcraft: crates/ui-egui/src/sidebar.rs:178 (`is_hidden`).
        self.state(section).hidden
    }

    /// Whether `section` is folded.
    pub fn is_folded(&self, section: S) -> bool {
        // lightcraft: crates/ui-egui/src/sidebar.rs:182 (`is_collapsed`).
        self.state(section).folded
    }

    /// Hides or shows `section`.
    pub fn set_hidden(&mut self, section: S, hidden: bool) {
        // lightcraft: crates/ui-egui/src/sidebar.rs:187 (`set_hidden`).
        if let Some(s) = self.state_mut(section) {
            s.hidden = hidden;
        }
    }

    /// Folds or unfolds `section`.
    pub fn set_folded(&mut self, section: S, folded: bool) {
        // lightcraft: crates/ui-egui/src/sidebar.rs:194 (`set_collapsed`).
        if let Some(s) = self.state_mut(section) {
            s.folded = folded;
        }
    }

    /// Shows and unfolds `section` (a command that needs it in view).
    pub fn reveal(&mut self, section: S) {
        // lightcraft: crates/ui-egui/src/sidebar.rs:205 (`reveal`).
        self.set_hidden(section, false);
        self.set_folded(section, false);
    }

    /// Moves `section` before `before` (`None`: to the end). Returns false and changes nothing
    /// when the shown order would stay the same (a hidden section between them doesn't count).
    pub fn move_before(&mut self, section: S, before: Option<S>) -> bool {
        // lightcraft: crates/ui-egui/src/sidebar.rs:214 (`move_before`).
        if before == Some(section) {
            return false;
        }
        let was = self.clone();
        let moved = self.state(section);
        self.0.retain(|s| s.section != section);
        let at = before.and_then(|b| self.0.iter().position(|s| s.section == b)).unwrap_or(self.0.len());
        self.0.insert(at, moved);
        if !moved.hidden && self.shown().eq(was.shown()) {
            *self = was;
            return false;
        }
        *self != was
    }

    /// Moves `section` one shown place earlier (`up`) or later, stepping over hidden sections so
    /// every move is visible; with `to_end`, to the first or last shown place. Returns whether it
    /// moved.
    pub fn move_by(&mut self, section: S, up: bool, to_end: bool) -> bool {
        // lightcraft: crates/ui-egui/src/sidebar.rs:234 (`move_by`).
        let shown: Vec<S> = self.shown().collect();
        let Some(at) = shown.iter().position(|s| *s == section) else { return false };
        if up {
            let Some(above) = at.checked_sub(1).and_then(|i| shown.get(if to_end { 0 } else { i })) else { return false };
            self.move_before(section, Some(*above))
        } else {
            if at + 1 >= shown.len() {
                return false;
            }
            // below the next one: above the one after it, or last
            self.move_before(section, if to_end { None } else { shown.get(at + 2).copied() })
        }
    }

    /// Whether [`move_by`](Self::move_by) would move `section` (Move Up and Move Down are
    /// disabled at the ends).
    pub fn can_move(&self, section: S, up: bool) -> bool {
        // lightcraft: crates/ui-egui/src/sidebar.rs:250 (`can_move`).
        let shown: Vec<S> = self.shown().collect();
        match shown.iter().position(|s| *s == section) {
            Some(at) if up => at > 0,
            Some(at) => at + 1 < shown.len(),
            None => false,
        }
    }

    /// Whether this is the usual arrangement (Reset is disabled then).
    pub fn is_default(&self) -> bool {
        // lightcraft: crates/ui-egui/src/sidebar.rs:260 (`is_default`).
        *self == Self::default()
    }

    /// Applies one event; returns whether the layout changed (the host saves it then). `Button`
    /// changes nothing.
    pub fn apply(&mut self, event: &SectionEvent<S>) -> bool {
        // new: the host applies the widget's events (lead call 43).
        match *event {
            SectionEvent::Moved { section, before } => self.move_before(section, before),
            SectionEvent::Hidden(section) => self.change(section, |s| &mut s.hidden, true),
            SectionEvent::Shown(section) => self.change(section, |s| &mut s.hidden, false),
            SectionEvent::Folded { section, folded } => self.change(section, |s| &mut s.folded, folded),
            SectionEvent::Button { .. } => false,
            SectionEvent::Reset => {
                let changed = !self.is_default();
                *self = Self::default();
                changed
            }
        }
    }
}

impl<S: SectionKey> SectionLayout<S> {
    fn state(&self, section: S) -> SectionState<S> {
        self.0.iter().copied().find(|s| s.section == section).unwrap_or_else(|| SectionState::new(section))
    }

    fn state_mut(&mut self, section: S) -> Option<&mut SectionState<S>> {
        self.0.iter_mut().find(|s| s.section == section)
    }

    /// Sets one flag of `section`; returns whether it changed.
    fn change(&mut self, section: S, flag: impl FnOnce(&mut SectionState<S>) -> &mut bool, value: bool) -> bool {
        let Some(state) = self.state_mut(section) else { return false };
        let flag = flag(state);
        let changed = *flag != value;
        *flag = value;
        changed
    }
}

impl<S: SectionKey> serde::Serialize for SectionLayout<S> {
    fn serialize<Z: serde::Serializer>(&self, serializer: Z) -> Result<Z::Ok, Z::Error> {
        // lightcraft: crates/ui-egui/src/sidebar.rs:102 (`From<SidebarLayout> for Vec<SectionState>`).
        #[derive(serde::Serialize)]
        struct Saved {
            id: &'static str,
            hidden: bool,
            folded: bool,
        }
        serializer.collect_seq(self.0.iter().map(|s| Saved { id: s.section.id(), hidden: s.hidden, folded: s.folded }))
    }
}

impl<'de, S: SectionKey> serde::Deserialize<'de> for SectionLayout<S> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // lightcraft: crates/ui-egui/src/sidebar.rs:110 (lenient `From<serde_json::Value>`), without serde_json: any format.
        let entries = deserializer.deserialize_any(lenient::List)?;
        Ok(Self::from_states(
            entries.into_iter().filter_map(|e| Some(SectionState { section: S::from_id(e.id.as_deref()?)?, hidden: e.hidden, folded: e.folded })),
        ))
    }
}

/// Reading a saved arrangement from any format without failing: whatever is not understood is
/// left out. A value is taken by `deserialize_any`, so each visitor says what it accepts and
/// drains the rest.
mod lenient {
    use std::fmt;

    use serde::de::{Deserialize, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};

    /// One saved entry, before its id is looked up.
    pub(super) struct Entry {
        pub id: Option<String>,
        pub hidden: bool,
        pub folded: bool,
    }

    /// A value that is a bool, a string or something else.
    enum Scalar {
        Bool(bool),
        Str(String),
        Other,
    }

    impl Scalar {
        fn flag(&self) -> bool {
            matches!(self, Scalar::Bool(true))
        }
    }

    impl<'de> Deserialize<'de> for Scalar {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            struct V;
            impl<'de> Visitor<'de> for V {
                type Value = Scalar;
                fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    f.write_str("anything")
                }
                fn visit_bool<E>(self, v: bool) -> Result<Scalar, E> {
                    Ok(Scalar::Bool(v))
                }
                fn visit_str<E>(self, v: &str) -> Result<Scalar, E> {
                    Ok(Scalar::Str(v.to_owned()))
                }
                fn visit_i64<E>(self, _: i64) -> Result<Scalar, E> {
                    Ok(Scalar::Other)
                }
                fn visit_u64<E>(self, _: u64) -> Result<Scalar, E> {
                    Ok(Scalar::Other)
                }
                fn visit_f64<E>(self, _: f64) -> Result<Scalar, E> {
                    Ok(Scalar::Other)
                }
                fn visit_unit<E>(self) -> Result<Scalar, E> {
                    Ok(Scalar::Other)
                }
                fn visit_none<E>(self) -> Result<Scalar, E> {
                    Ok(Scalar::Other)
                }
                fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Scalar, D::Error> {
                    d.deserialize_any(self)
                }
                fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Scalar, A::Error> {
                    while seq.next_element::<IgnoredAny>()?.is_some() {}
                    Ok(Scalar::Other)
                }
                fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Scalar, A::Error> {
                    while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
                    Ok(Scalar::Other)
                }
            }
            deserializer.deserialize_any(V)
        }
    }

    impl<'de> Deserialize<'de> for Entry {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            struct V;
            impl<'de> Visitor<'de> for V {
                type Value = Entry;
                fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                    f.write_str("a section id or a map")
                }
                fn visit_str<E>(self, v: &str) -> Result<Entry, E> {
                    Ok(Entry { id: Some(v.to_owned()), hidden: false, folded: false })
                }
                fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Entry, A::Error> {
                    let mut entry = Entry { id: None, hidden: false, folded: false };
                    while let Some(key) = map.next_key::<Scalar>()? {
                        let value = map.next_value::<Scalar>()?;
                        let Scalar::Str(key) = key else { continue };
                        match key.as_str() {
                            "id" if entry.id.is_none() => {
                                if let Scalar::Str(id) = value {
                                    entry.id = Some(id);
                                }
                            }
                            "hidden" => entry.hidden = value.flag(),
                            "folded" | "collapsed" => entry.folded |= value.flag(),
                            _ => {}
                        }
                    }
                    Ok(entry)
                }
                fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Entry, A::Error> {
                    while seq.next_element::<IgnoredAny>()?.is_some() {}
                    Ok(Entry { id: None, hidden: false, folded: false })
                }
                fn visit_bool<E>(self, _: bool) -> Result<Entry, E> {
                    Ok(Entry { id: None, hidden: false, folded: false })
                }
                fn visit_i64<E>(self, _: i64) -> Result<Entry, E> {
                    Ok(Entry { id: None, hidden: false, folded: false })
                }
                fn visit_u64<E>(self, _: u64) -> Result<Entry, E> {
                    Ok(Entry { id: None, hidden: false, folded: false })
                }
                fn visit_f64<E>(self, _: f64) -> Result<Entry, E> {
                    Ok(Entry { id: None, hidden: false, folded: false })
                }
                fn visit_unit<E>(self) -> Result<Entry, E> {
                    Ok(Entry { id: None, hidden: false, folded: false })
                }
                fn visit_none<E>(self) -> Result<Entry, E> {
                    Ok(Entry { id: None, hidden: false, folded: false })
                }
                fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Entry, D::Error> {
                    d.deserialize_any(self)
                }
            }
            deserializer.deserialize_any(V)
        }
    }

    /// The saved list; anything that is not a list reads as an empty one.
    pub(super) struct List;

    impl<'de> Visitor<'de> for List {
        type Value = Vec<Entry>;
        fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("a list of sections")
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Vec<Entry>, A::Error> {
            let mut entries = Vec::new();
            while let Some(entry) = seq.next_element::<Entry>()? {
                entries.push(entry);
            }
            Ok(entries)
        }
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Vec<Entry>, A::Error> {
            while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
            Ok(Vec::new())
        }
        fn visit_bool<E>(self, _: bool) -> Result<Vec<Entry>, E> {
            Ok(Vec::new())
        }
        fn visit_i64<E>(self, _: i64) -> Result<Vec<Entry>, E> {
            Ok(Vec::new())
        }
        fn visit_u64<E>(self, _: u64) -> Result<Vec<Entry>, E> {
            Ok(Vec::new())
        }
        fn visit_f64<E>(self, _: f64) -> Result<Vec<Entry>, E> {
            Ok(Vec::new())
        }
        fn visit_str<E>(self, _: &str) -> Result<Vec<Entry>, E> {
            Ok(Vec::new())
        }
        fn visit_unit<E>(self) -> Result<Vec<Entry>, E> {
            Ok(Vec::new())
        }
        fn visit_none<E>(self) -> Result<Vec<Entry>, E> {
            Ok(Vec::new())
        }
        fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Vec<Entry>, D::Error> {
            d.deserialize_any(self)
        }
    }
}
