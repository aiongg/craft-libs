//! `ReorderableSections`: the sidebar widget over a `SectionLayout`.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{AsIdSalt, Response, Ui};

use super::{SectionEvent, SectionKey, SectionLayout};
use crate::icons::Icon;

/// A section header's button at the trailing edge (LightCraft: Albums' "+" opens Create Album;
/// Keywords' makes a keyword tag). A press on it never folds or drags the section.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionButton {
    /// The id reported in `SectionEvent::Button`.
    pub id: &'static str,
    /// The glyph.
    pub icon: Icon,
    /// The tooltip and accessible name, translated.
    pub label: String,
}

impl SectionButton {
    /// A header button.
    pub fn new(id: &'static str, icon: Icon, label: impl Into<String>) -> Self {
        // lightcraft: crates/ui-egui/src/panels/left/sections.rs:265 (`header_button`).
        Self { id, icon, label: label.into() }
    }
}

/// What one section shows this pass: the translated title and the rest the key doesn't carry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionInfo<S> {
    /// The section.
    pub section: S,
    /// Its title.
    pub title: String,
    /// Its row count, shown at the trailing edge while folded when `show_counts` is on.
    pub count: Option<usize>,
    /// The hint an open section with no rows shows ("No albums yet").
    pub empty_hint: Option<String>,
    /// The header button.
    pub button: Option<SectionButton>,
    /// `false`: the build lacks it (LightCraft's Local on the web); not shown, keeps its place.
    pub available: bool,
}

impl<S> SectionInfo<S> {
    /// An available section with no count, hint or button.
    pub fn new(section: S, title: impl Into<String>) -> Self {
        // lightcraft: crates/ui-egui/src/sidebar.rs:23 (`SidebarSection` with `title`, `empty_hint`, `available`).
        Self { section, title: title.into(), count: None, empty_hint: None, button: None, available: true }
    }

    /// The row count.
    #[must_use]
    pub fn count(mut self, count: usize) -> Self {
        self.count = Some(count);
        self
    }

    /// The empty hint.
    #[must_use]
    pub fn empty_hint(mut self, hint: impl Into<String>) -> Self {
        self.empty_hint = Some(hint.into());
        self
    }

    /// The header button.
    #[must_use]
    pub fn button(mut self, button: SectionButton) -> Self {
        self.button = Some(button);
        self
    }

    /// Whether the build has the section.
    #[must_use]
    pub fn available(mut self, available: bool) -> Self {
        self.available = available;
        self
    }
}

/// What [`ReorderableSections::show`] returns.
#[derive(Debug)]
#[must_use = "the host applies the sections' events"]
pub struct SectionsOutput<S> {
    /// The area.
    pub response: Response,
    /// What happened this pass, in order.
    pub events: Vec<SectionEvent<S>>,
    /// Each header button's response this pass, to anchor the host's popup after a `Button`.
    pub buttons: Vec<(S, Response)>,
}

crate::words! {
    /// The sections' words, English by default.
    #[non_exhaustive]
    pub struct SectionsLabels {
        hide: "sections.hide" = "Hide This Section";
        move_up: "sections.moveUp" = "Move Up";
        move_down: "sections.moveDown" = "Move Down";
        reset: "sections.reset" = "Reset";
        /// The live-region announcement after a move: `{title}`, `{position}`, `{count}`.
        moved: "sections.moved" = "{title} moved to position {position} of {count}";
    }
}

/// Sidebar sections the user folds, hides and drags (N8.03), over the host's
/// [`SectionLayout`]. The widget reports [`SectionEvent`]s; it never changes the layout.
///
/// - **Headers.** `SectionHeader`'s look (row 24, chevron 11, hairline above), one per shown and
///   available section in layout order; the title; the folded row count in `Caption`
///   `text.faint` at the trailing edge (`SectionInfo::count`, `.show_counts(true)`); the header
///   button at the trailing edge. Click, Enter or Space folds or unfolds (`Folded`).
/// - **Bodies.** `body(ui, section)` for each open section; one that adds no widget shows its
///   `empty_hint` in `Caption` `text.faint` at the row indent.
/// - **Drag.** A header drags with `Sense::click_and_drag` past egui's threshold; the drop line
///   (`list::drop_line`, `list.drop` 2 pt with its halo) shows in the gap above the section it
///   would go before, or below the last; the dragged title follows the pointer as a `DragBadge`;
///   release over the area reports `Moved { section, before }` (none when `move_before` would be
///   a no-op); release outside or Esc leaves it. A press on the header button never drags or
///   folds.
/// - **Keys.** Cmd/Ctrl+] and [ on a focused header move it one shown place later or earlier
///   (`SectionLayout::move_by`), with Shift to the ends; reported as `Moved`. After any move the
///   new place is announced in a polite live region ([`SectionsLabels::moved`]).
/// - **Menu.** On a header (secondary click, Menu key, Shift+F10): Hide This Section, Move Up,
///   Move Down (disabled at the ends), a separator, a check item per available section (shown or
///   hidden), a separator, Reset (disabled when `is_default`). The host's fixed title block opens
///   the same menu through [`ReorderableSections::menu`].
/// - **Pinned header.** The header of the open section whose own header scrolled out of the
///   enclosing `ScrollArea`'s view stays at the top (`surface.panel` fill, `line.divider` hairline
///   under it); fold, drag, menu and keys work on it; the next header pushes it up. Folding from
///   the pinned copy scrolls the section's own header into view in the same pass.
/// - **Unavailable.** A section with `available: false` is not drawn and keeps its place.
/// - **AccessKit.** Each header a `DisclosureTriangle` named by its title with `expanded`; the
///   area a `Group` named by `.label(..)`.
/// - **RTL.** Headers, chevrons, counts, buttons and the drop line mirror.
pub struct ReorderableSections<'a, S> {
    id_salt: egui::Id,
    layout: &'a SectionLayout<S>,
    show_counts: bool,
    label: String,
    labels: Option<SectionsLabels>,
}

impl<'a, S: SectionKey> ReorderableSections<'a, S> {
    /// The sections of `layout`.
    pub fn new(id_salt: impl AsIdSalt, layout: &'a SectionLayout<S>) -> Self {
        // lightcraft: crates/ui-egui/src/panels/left/sections.rs:189 (`section_header`).
        todo!()
    }

    /// Show folded sections' row counts.
    #[must_use]
    pub fn show_counts(mut self, on: bool) -> Self {
        self.show_counts = on;
        self
    }

    /// The area's accessible name.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// The words.
    #[must_use]
    pub fn labels(mut self, labels: SectionsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Draws the shown sections of `sections` (each section's info; a section of the layout
    /// without info is not drawn) with `body` for each open one.
    pub fn show(self, ui: &mut Ui, sections: &[SectionInfo<S>], body: impl FnMut(&mut Ui, S)) -> SectionsOutput<S> {
        // lightcraft: crates/ui-egui/src/panels/left/sections.rs:5 (`section`), :62 (pinned header), :118 (drop), :164 (drag feedback).
        todo!()
    }

    /// The section menu's items inside a menu the host opened (its fixed title block), for
    /// `on` (a section, or `None` from the title block: no Hide, Move Up or Move Down). Returns
    /// the chosen item's event.
    pub fn menu(
        ui: &mut Ui,
        layout: &SectionLayout<S>,
        sections: &[SectionInfo<S>],
        on: Option<S>,
        labels: &SectionsLabels,
    ) -> Option<SectionEvent<S>> {
        // lightcraft: crates/ui-egui/src/panels/left/sections.rs:315 (the header and My Photos menu).
        todo!()
    }
}
