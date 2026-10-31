//! [`GlyphsPanel`] (F14.05): Photoshop's Glyphs panel. A family and style picker, a category
//! list (Entire Font, Punctuation, Currency, …), a grid of glyph cells drawn by the app's text
//! engine, a zoom slider, and the recently used glyphs in a row at the top. Double-click or Enter
//! inserts the glyph into the text being edited.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{ColorImage, Ui};
use serde::{Deserialize, Serialize};

craft_ui::words! {
    /// The panel's words, English by default.
    #[non_exhaustive]
    pub struct GlyphsLabels {
        show: "glyphs.show" = "Show:";
        entire_font: "glyphs.entireFont" = "Entire Font";
        recent: "glyphs.recent" = "Recently Used";
        style: "glyphs.style" = "Style";
        zoom: "glyphs.zoom" = "Zoom";
        no_text: "glyphs.noText" = "Edit text to insert glyphs";
    }
}

/// The glyph categories, Photoshop's list in order.
pub const CATEGORIES: &[&str] = &[
    "Entire Font",
    "Recently Used",
    "Punctuation",
    "Alternates for Selection",
    "Numbers",
    "Currency",
    "Symbols",
    "Math Symbols",
    "Arrows",
    "Latin",
    "Greek",
    "Cyrillic",
    "Letterlike Symbols",
    "Superscripts and Subscripts",
    "Ornaments",
];

/// Where glyph cells come from: the app's text engine draws a glyph of a face at a pixel size and
/// lists the characters a face has. Apps plug in their own (PhotoCraft's `photocraft_text`).
pub trait GlyphSource: Send + Sync {
    /// Every character the face (family, style) has a glyph for, in code point order.
    fn charmap(&self, family: &str, style: &str) -> Vec<char>;
    /// The glyph of `c` in the face, `px` pixels tall, as an image for a texture; `None` when the
    /// face lacks it.
    fn glyph(&self, family: &str, style: &str, c: char, px: u32) -> Option<ColorImage>;
    /// Whether `c` is in `category` (one of [`CATEGORIES`]).
    fn in_category(&self, c: char, category: &str) -> bool;
}

/// The panel's remembered state (`[glyphs_panel]`): category, zoom and the recent glyphs.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GlyphsPanelSettings {
    pub category: String,
    /// Cell size in points, 24..=96.
    pub zoom: f32,
    /// Most recent first, at most 25 (Photoshop).
    pub recent: Vec<String>,
}

impl craft_host::Section for GlyphsPanelSettings {
    const KEY: &'static str = "glyphs_panel";
}

/// What the panel reports.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GlyphsPanelResponse {
    /// A glyph to insert at the caret (double-click, Enter on the focused cell).
    pub insert: Option<String>,
    /// The family or style changed in the panel's own pickers.
    pub font_changed: bool,
}

/// The Glyphs panel body. The family comes from the text selection while editing (the panel
/// follows the caret), else from the panel's own picker.
///
/// - Keys: the grid is one Tab stop; arrows move the focused cell, Page Up/Down a screen, Home/End
///   to the ends, Enter inserts. Hovering a cell shows its name and code point as a tooltip.
/// - A cell is drawn from [`GlyphSource::glyph`] at the zoom size, cached per (face, char, px);
///   a face without the character is skipped. The grid shows only the current category.
/// - Without text being edited the cells still show, Enter and double-click do nothing and the
///   footer says so (`no_text`).
pub struct GlyphsPanel<'a> {
    source: &'a dyn GlyphSource,
    family: &'a mut String,
    style: &'a mut String,
    settings: &'a mut GlyphsPanelSettings,
    editing: bool,
    labels: Option<GlyphsLabels>,
}

impl<'a> GlyphsPanel<'a> {
    pub fn new(source: &'a dyn GlyphSource, family: &'a mut String, style: &'a mut String, settings: &'a mut GlyphsPanelSettings) -> Self {
        GlyphsPanel { source, family, style, settings, editing: false, labels: None }
    }

    /// Whether text is being edited (inserting is possible).
    pub fn editing(mut self, editing: bool) -> Self {
        self.editing = editing;
        self
    }

    pub fn labels(mut self, labels: GlyphsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Draw the panel body in `ui`.
    // pc: crates/ui-egui/src/panels/type_panels_ui.rs:627 `glyphs_panel` (pickers, category
    // list, grid, zoom) and `:578 glyph_texture`, `:595 charmap`; recents through `:608 insert_glyph`.
    pub fn show(self, ui: &mut Ui) -> GlyphsPanelResponse {
        todo!()
    }
}
