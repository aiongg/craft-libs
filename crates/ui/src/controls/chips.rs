#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]
//! [`ChipField`]: removable chips laid out in lines, with "+N more" where a long list stops.

use std::borrow::Cow;
use std::ops::Range;

use egui::{AsIdSalt, IdSalt, Response, Ui};

use super::ControlSize;

crate::words! {
    /// The chips' words, English by default.
    #[non_exhaustive]
    pub struct ChipLabels {
        /// A chip's remove button, for AccessKit and its tooltip: `{name}` the chip's full name
        /// (LightCraft's "Remove “{name}”").
        remove: "chips.remove" = "Remove “{name}”";
        /// The button for the chips that don't fit: `{n}` how many.
        more: "chips.more" = "+{n} more";
        clear_all: "chips.clearAll" = "Clear All";
        /// A partial chip's accessible name (the asterisk is only drawn): `{name}`.
        partial: "chips.partial" = "{name}, some of the selection";
    }
}

/// One chip: a tag, a keyword, a filter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Chip<'a> {
    label: Cow<'a, str>,
    full_name: Option<Cow<'a, str>>,
    partial: bool,
    removable: bool,
}

impl<'a> Chip<'a> {
    /// A removable chip showing `label`.
    pub fn new(label: impl Into<Cow<'a, str>>) -> Self {
        Self { label: label.into(), full_name: None, partial: false, removable: true }
    }

    /// The name screen readers and the tooltip give when the label is shortened
    /// ("Places › Europe › Paris" for a chip showing "Paris").
    pub fn full_name(mut self, name: impl Into<Cow<'a, str>>) -> Self {
        self.full_name = Some(name.into());
        self
    }

    /// Only some of the selected items have it: drawn in `text.muted` with an asterisk after the
    /// label (LightCraft's keywords of a multi-selection).
    pub fn partial(mut self, partial: bool) -> Self {
        self.partial = partial;
        self
    }

    /// It has a × at its end (default on).
    pub fn removable(mut self, removable: bool) -> Self {
        self.removable = removable;
        self
    }
}

/// What the chips reported this pass.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ChipResponse {
    pub response: Response,
    /// The chip whose × was clicked, or which Delete or Backspace removed: the host removes it.
    pub removed: Option<usize>,
    /// The chip whose name was clicked or activated with Enter or Space.
    pub clicked: Option<usize>,
    /// The chip whose context menu was asked for (secondary click, Menu key, Shift+F10); the host
    /// opens its menu on [`ChipResponse::response`] at the chip.
    pub context_menu: Option<(usize, egui::Rect)>,
    /// Clear All was pressed.
    pub cleared: bool,
}

/// A field of removable chips, after LightCraft's Keywording box and its grid's filter chips.
///
/// ```ignore
/// let chips: Vec<Chip> = keywords.iter().map(|k| Chip::new(&k.name).partial(k.partial)).collect();
/// let out = ChipField::new("keywords", "Keywords", &chips).max_lines(3).show(ui);
/// if let Some(i) = out.removed { app.run("keywords.remove", &keywords[i]) }
/// ```
///
/// - **Look.** Each chip is a pill 22 points high (`chip.height`, the `shape.pill` radius) with a
///   1-point `field.border.rest` edge, the label in the `Label` role 8 points in, and for a
///   removable chip a × target 22 points wide at its end (`chip.remove_width`; arms 3.5, 1.5
///   wide, `text.muted`, `text.normal` under the pointer). Hovering the name fills the chip with
///   `state.hover` at half strength; hovering the × lights only the × end, so the user sees which
///   part a click hits. A label too long for a line is cut short with "…" to fit.
/// - **Lines.** Chips flow in lines 6 points apart (`chip.gap`), each moved whole to the next
///   line when it doesn't fit ([`chip_lines`]). With [`max_lines`](Self::max_lines), the chips
///   past the last line are counted on a "+N more" button there, which opens a
///   [`Menu`](crate::menu::Menu) with a row per hidden chip (its full name; choosing it removes
///   it). With [`clear_all`](Self::clear_all), a Clear All button ends the last line (the grid's
///   filter strip).
/// - **Keys.** The field is one Tab stop; ← and → (mirrored in right-to-left) move the focused
///   chip, Home and End go to the ends, Delete and Backspace remove the focused chip and the
///   focus moves to the next one (the previous one at the end), Enter and Space click it, the
///   Menu key and Shift+F10 ask for its menu. The focused chip draws the focus ring.
/// - **AccessKit.** A `List` named by its label; each chip a `ListItem` named by its full name
///   (and [`ChipLabels::partial`]), with a child `Button` [`ChipLabels::remove`].
pub struct ChipField<'a> {
    id_salt: IdSalt,
    label: String,
    chips: &'a [Chip<'a>],
    max_lines: Option<usize>,
    clear_all: bool,
    size: ControlSize,
    enabled: bool,
}

impl<'a> ChipField<'a> {
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, chips: &'a [Chip<'a>]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), label: label.into(), chips, max_lines: None, clear_all: false, size: ControlSize::Md, enabled: true }
    }

    /// At most `lines` lines; the rest go behind "+N more".
    pub fn max_lines(mut self, lines: usize) -> Self {
        self.max_lines = Some(lines);
        self
    }

    /// A Clear All button after the chips.
    pub fn clear_all(mut self, on: bool) -> Self {
        self.clear_all = on;
        self
    }

    pub fn size(mut self, size: ControlSize) -> Self {
        self.size = size;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    // lightcraft: crates/ui-egui/src/panels/keywording.rs:80 chip, :243 chip_groups; lightcraft: crates/ui-egui/src/panels/chips.rs:92 show
    pub fn show(self, ui: &mut Ui) -> ChipResponse {
        todo!()
    }
}

/// The chips of each line when chips `widths` wide flow into lines `row_width` wide, `gap`
/// apart: a chip that doesn't fit what is left of a line starts the next; a chip wider than a
/// line has a line of its own. Ranges index `widths`.
// lightcraft: crates/ui-egui/src/panels/keywording.rs:601 lines
pub fn chip_lines(widths: &[f32], row_width: f32, gap: f32) -> Vec<Range<usize>> {
    todo!()
}
