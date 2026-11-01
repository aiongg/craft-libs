//! `SectionKey`, `SectionState`, `SectionLayout` and `SectionEvent`: the user's arrangement of a
//! sidebar's sections, made whole whatever is read.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

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

/// What [`ReorderableSections`](super::ReorderableSections) reported, in order. View state: never
/// an undo step. The host applies them ([`SectionLayout::apply`]) or routes them through its
/// commands (LightCraft's `view.sidebar*`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SectionEvent<S> {
    /// Move `section` before `before` (`None`: to the end). From a drag, Cmd/Ctrl+] and [, or Move
    /// Up and Move Down.
    Moved { section: S, before: Option<S> },
    /// Hide This Section, or a check item turned off.
    Hidden(S),
    /// A check item turned on.
    Shown(S),
    /// The header folded or unfolded.
    Folded { section: S, folded: bool },
    /// The header button with `id` was pressed.
    Button { section: S, id: &'static str },
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
        todo!()
    }
}

impl<S: SectionKey> SectionLayout<S> {
    /// The layout these states describe, made whole (see the type).
    pub fn from_states(states: impl IntoIterator<Item = SectionState<S>>) -> Self {
        // lightcraft: crates/ui-egui/src/sidebar.rs:150 (`from_states`).
        todo!()
    }

    /// Every section top to bottom, hidden ones included.
    pub fn states(&self) -> &[SectionState<S>] {
        // lightcraft: crates/ui-egui/src/sidebar.rs:161 (`sections`).
        todo!()
    }

    /// The shown sections, top to bottom.
    pub fn shown(&self) -> impl Iterator<Item = S> + '_ {
        // lightcraft: crates/ui-egui/src/sidebar.rs:166 (`shown`).
        self.0.iter().filter(|s| !s.hidden).map(|s| s.section)
    }

    /// Whether `section` is hidden.
    pub fn is_hidden(&self, section: S) -> bool {
        // lightcraft: crates/ui-egui/src/sidebar.rs:178 (`is_hidden`).
        todo!()
    }

    /// Whether `section` is folded.
    pub fn is_folded(&self, section: S) -> bool {
        // lightcraft: crates/ui-egui/src/sidebar.rs:182 (`is_collapsed`).
        todo!()
    }

    /// Hides or shows `section`.
    pub fn set_hidden(&mut self, section: S, hidden: bool) {
        // lightcraft: crates/ui-egui/src/sidebar.rs:187 (`set_hidden`).
        todo!()
    }

    /// Folds or unfolds `section`.
    pub fn set_folded(&mut self, section: S, folded: bool) {
        // lightcraft: crates/ui-egui/src/sidebar.rs:194 (`set_collapsed`).
        todo!()
    }

    /// Shows and unfolds `section` (a command that needs it in view).
    pub fn reveal(&mut self, section: S) {
        // lightcraft: crates/ui-egui/src/sidebar.rs:205 (`reveal`).
        todo!()
    }

    /// Moves `section` before `before` (`None`: to the end). Returns false and changes nothing
    /// when the shown order would stay the same (a hidden section between them doesn't count).
    pub fn move_before(&mut self, section: S, before: Option<S>) -> bool {
        // lightcraft: crates/ui-egui/src/sidebar.rs:214 (`move_before`).
        todo!()
    }

    /// Moves `section` one shown place earlier (`up`) or later, stepping over hidden sections so
    /// every move is visible; with `to_end`, to the first or last shown place. Returns whether it
    /// moved.
    pub fn move_by(&mut self, section: S, up: bool, to_end: bool) -> bool {
        // lightcraft: crates/ui-egui/src/sidebar.rs:234 (`move_by`).
        todo!()
    }

    /// Whether [`move_by`](Self::move_by) would move `section` (Move Up and Move Down are
    /// disabled at the ends).
    pub fn can_move(&self, section: S, up: bool) -> bool {
        // lightcraft: crates/ui-egui/src/sidebar.rs:250 (`can_move`).
        todo!()
    }

    /// Whether this is the usual arrangement (Reset is disabled then).
    pub fn is_default(&self) -> bool {
        // lightcraft: crates/ui-egui/src/sidebar.rs:260 (`is_default`).
        todo!()
    }

    /// Applies one event; returns whether the layout changed (the host saves it then). `Button`
    /// changes nothing.
    pub fn apply(&mut self, event: &SectionEvent<S>) -> bool {
        // new: the host applies the widget's events (lead call 43).
        todo!()
    }
}

impl<S: SectionKey> serde::Serialize for SectionLayout<S> {
    fn serialize<Z: serde::Serializer>(&self, serializer: Z) -> Result<Z::Ok, Z::Error> {
        // lightcraft: crates/ui-egui/src/sidebar.rs:102 (`From<SidebarLayout> for Vec<SectionState>`).
        todo!()
    }
}

impl<'de, S: SectionKey> serde::Deserialize<'de> for SectionLayout<S> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // lightcraft: crates/ui-egui/src/sidebar.rs:110 (lenient `From<serde_json::Value>`), without serde_json: any format.
        todo!()
    }
}
