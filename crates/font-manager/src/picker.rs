//! [`FontPicker`] (F14.01, gap 17): the font family field of the type tool, the Character panel,
//! the style panels and the Glyphs panel. A text field that filters the installed families as the
//! user types, with a list under it of family rows drawn in their own face, favourites (stars),
//! recent families at the top, class and feature filters above the list and a "similar fonts"
//! filter. Behaviour follows Photoshop's font menu; PhotoCraft's `family_picker_in` is the source.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::sync::Arc;

use craft_font_catalog::Catalog;
use craft_host::Section;
use egui::{Rect, Response, Ui};
use serde::{Deserialize, Serialize};

craft_ui::words! {
    /// The picker's words, English by default.
    #[non_exhaustive]
    pub struct FontPickerLabels {
        search: "fontPicker.search" = "Search fonts";
        recent: "fontPicker.recent" = "Recent";
        favorites: "fontPicker.favorites" = "Show favorite fonts";
        similar: "fontPicker.similar" = "Show similar fonts";
        variable: "fontPicker.variable" = "Show variable fonts";
        classes: "fontPicker.classes" = "Filter by class";
        all_classes: "fontPicker.allClasses" = "All Classes";
        clear_all: "fontPicker.clearAll" = "Clear All";
        sample: "fontPicker.sample" = "Sample";
        no_match: "fontPicker.noMatch" = "No fonts match";
        star: "fontPicker.star" = "Add to favorites";
        unstar: "fontPicker.unstar" = "Remove from favorites";
    }
}

/// Photoshop's font classes, in its filter menu order (the class glyph at the row's right).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FontClass {
    Serif,
    SlabSerif,
    SansSerif,
    Script,
    Monospace,
    Handwritten,
    Blackletter,
    Decorative,
}

impl FontClass {
    /// Every class, in menu order.
    pub const ALL: [FontClass; 8] = [
        FontClass::Serif,
        FontClass::SlabSerif,
        FontClass::SansSerif,
        FontClass::Script,
        FontClass::Monospace,
        FontClass::Handwritten,
        FontClass::Blackletter,
        FontClass::Decorative,
    ];

    /// The class of a family from its PANOSE and OS/2 data, else from its name; `None` when the
    /// face can't be read.
    // pc: crates/ui-egui/src/text/font_filters.rs:44 `traits` and crates/text/src/font_class.rs.
    pub fn of(catalog: &Catalog, family: &str) -> Option<FontClass> {
        todo!()
    }
}

/// Photoshop's Font Preview Size (Type › Font Preview Size): how large the sample in each row is,
/// or none.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FontPreviewSize {
    None,
    Small,
    #[default]
    Medium,
    Large,
    ExtraLarge,
    Huge,
}

impl FontPreviewSize {
    /// The sample's point size, `None` for no sample.
    // pc: crates/ui-egui/src/text/font_preview.rs:40 `sample`.
    pub fn points(self) -> Option<f32> {
        todo!()
    }
}

/// What narrows the list. Every filter off shows every installed family.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FontFilters {
    /// Only families of this class (`None`: All Classes).
    pub class: Option<FontClass>,
    /// Only starred families.
    pub favorites: bool,
    /// Only variable fonts.
    pub variable: bool,
    /// Families like the current one, most alike first ([`SIMILAR_COUNT`] of them).
    pub similar: bool,
}

impl FontFilters {
    /// Whether any filter narrows the list (Clear All is enabled).
    pub fn any(&self) -> bool {
        self.class.is_some() || self.favorites || self.variable || self.similar
    }

    /// Clear All: every filter off.
    pub fn clear(&mut self) {
        *self = FontFilters::default();
    }
}

/// How many families "similar fonts" shows.
pub const SIMILAR_COUNT: usize = 30;

/// The picker's remembered state: starred families and the most recent picks, one settings
/// section shared by every picker of the app (`[font_picker]`).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FontPickerSettings {
    /// Starred families, in star order.
    pub favorites: Vec<String>,
    /// Most recent first, at most [`FontPickerSettings::recent_limit`].
    pub recent: Vec<String>,
    /// How many recent families the list shows (Photoshop: Number of Recent Fonts to Display, 10).
    pub recent_limit: usize,
    pub preview: FontPreviewSize,
    /// Show family names in English rather than their own language (Type › Show Font Names in English).
    pub names_in_english: bool,
}

impl Section for FontPickerSettings {
    const KEY: &'static str = "font_picker";
}

impl FontPickerSettings {
    /// Put `family` first in the recents, dropping a duplicate and anything past the limit.
    // pc: crates/ui-egui/src/app.rs `note_recent_font`.
    pub fn note_recent(&mut self, family: &str) {
        todo!()
    }

    /// Star or unstar `family`.
    pub fn set_favorite(&mut self, family: &str, on: bool) {
        todo!()
    }
}

/// One row of the list, for apps that draw their own list from the picker's filtering
/// ([`FontPicker::rows`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FamilyRow {
    /// The family as the catalog names it.
    pub family: String,
    /// The name shown: the family's name in the UI language unless `names_in_english`.
    pub shown: String,
    pub starred: bool,
    pub class: Option<FontClass>,
    pub variable: bool,
    /// The row is in the Recent group at the top.
    pub recent: bool,
}

/// What the user did this pass.
#[derive(Clone, Debug)]
pub struct FontPickerResponse {
    /// A family was picked: Enter on the typed name, a click or Enter on a row, or the wheel on the
    /// closed field. The caller applies it (and notes it as recent).
    pub picked: Option<String>,
    /// The family under the pointer or the keyboard row while the list is open, for a live preview
    /// of the selected text (Photoshop previews on hover). `None` restores the committed family.
    pub preview: Option<String>,
    /// The field's response, for `changed`, focus and layout.
    pub response: Response,
}

/// The font family picker. Built per pass over the catalog and the app's settings:
///
/// ```ignore
/// let mut settings = host.setting::<FontPickerSettings>();
/// let r = FontPicker::new("type-family", &catalog, &mut family).settings(&mut settings).width(220.0).show(ui);
/// if let Some(picked) = r.picked { settings.note_recent(&picked); host.set_setting(&settings); apply(picked); }
/// ```
///
/// What the user sees and does:
/// - A text field with a chevron. Typing filters the list to families whose name (in any
///   language) contains the text; the list opens on the first keystroke or on the chevron. The
///   current family is scrolled into view when the list opens.
/// - Rows: star, the family name in the UI face, a sample in the family's own face at the
///   preview size (the family name when the face can't draw "Sample"), the class glyph at the
///   right. Recent families form a group at the top, separated by a rule. A row under the pointer
///   or the keyboard row previews on the canvas.
/// - Above the list: the search field is the picker's own field; a row of filter toggles
///   (class list button, star, similar, variable) and Clear All, as Photoshop's font menu. The
///   class list is a collapsible column at the left.
/// - Keys: ↓/↑ move the keyboard row (wrapping), Enter picks it (or the exact typed name), Esc
///   closes and restores the field text, Tab picks and moves on. While the IME composes, Enter,
///   Esc and the arrows belong to the composition; a later Enter picks. On the closed field the
///   wheel steps to the previous or next family in list order and picks it (Photoshop).
/// - Platform: the list is a `Popover` and keeps the stable id `<salt>.popup`; on the web the
///   sample images are rendered from served faces as they arrive.
pub struct FontPicker<'a> {
    salt: &'a str,
    catalog: &'a Arc<Catalog>,
    current: &'a mut String,
    settings: Option<&'a mut FontPickerSettings>,
    filters: Option<&'a mut FontFilters>,
    width: Option<f32>,
    list_height: f32,
    labels: Option<FontPickerLabels>,
}

/// The default height of the open list, in points (PhotoCraft's `FONT_LIST_HEIGHT`).
pub const LIST_HEIGHT: f32 = 440.0;

impl<'a> FontPicker<'a> {
    /// A picker over `catalog` editing `current` (the family name). `salt` keeps its ids apart from
    /// other pickers in the same `Ui`.
    pub fn new(salt: &'a str, catalog: &'a Arc<Catalog>, current: &'a mut String) -> Self {
        FontPicker { salt, catalog, current, settings: None, filters: None, width: None, list_height: LIST_HEIGHT, labels: None }
    }

    /// Stars, recents and the preview size; without it no stars or recents are shown.
    pub fn settings(mut self, settings: &'a mut FontPickerSettings) -> Self {
        self.settings = Some(settings);
        self
    }

    /// The filters, kept by the caller so they survive the list closing; without it the filter
    /// row is hidden.
    pub fn filters(mut self, filters: &'a mut FontFilters) -> Self {
        self.filters = Some(filters);
        self
    }

    /// The field's width; the list is at least as wide.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// The open list's height.
    pub fn list_height(mut self, height: f32) -> Self {
        self.list_height = height;
        self
    }

    pub fn labels(mut self, labels: FontPickerLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The rows the list would show for `query` and the filters, in list order: recents first
    /// (when the query is empty), then the matching families collated in the UI language.
    // pc: crates/ui-egui/src/tools/type_tool/fonts.rs:69 `font_menu_rows` and
    // crates/ui-egui/src/text/font_filters.rs:65 `apply` (similar fonts ranked by trait distance).
    pub fn rows(&self, query: &str) -> Vec<FamilyRow> {
        todo!()
    }

    /// Draw the field and, when open, the list.
    // pc: crates/ui-egui/src/tools/type_tool/fonts.rs:388 `family_picker_in` (field, keys, IME
    // guard, popup, wheel) and `:843 wheel_family_index`; rows through `row`.
    pub fn show(self, ui: &mut Ui) -> FontPickerResponse {
        todo!()
    }

    /// One list row: star, name, sample, class glyph; returns the row's response.
    // pc: crates/ui-egui/src/text/font_preview.rs:132 `paint` (sample images, cached per
    // family and size) and crates/ui-egui/src/tools/type_tool/fonts.rs:600 (the row).
    fn row(&self, ui: &mut Ui, row: &FamilyRow, rect: Rect, selected: bool) -> Response {
        todo!()
    }
}

/// The row of a list the keyboard lands on after typing `text`: the exact match, else the first
/// row, else the row that was selected. Pure, for the tests.
// pc: crates/ui-egui/src/tools/type_tool/fonts.rs:365 `accept_row`.
pub fn accept_row(rows: &[FamilyRow], text: &str, selected: Option<usize>) -> Option<usize> {
    todo!()
}
