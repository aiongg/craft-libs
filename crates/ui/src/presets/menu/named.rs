//! The named preset row: a preset field, the name prompt and the New…, Rename…, Delete, Load… and Save… entries (`NamedPresetRow`, `NamedPresetEvent`, `NamedPresetOutput`).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::rc::Rc;

use egui::{AsIdSalt, IdSalt, Painter, Rect, Response, Ui};

use super::KeyTest;
use super::files::{PresetFileEvent, PresetFiles};
use crate::composites::StatusLine;
use crate::presets::model::PresetError;
use crate::presets::picker::PickerField;
use crate::presets::{PresetRow, ThumbInfo};

crate::words! {
    /// The words of a [`NamedPresetRow`]. `{name}` stands for a preset's name.
    pub struct NamedPresetLabels {
        new: "namedPreset.new" = "New…";
        rename: "namedPreset.rename" = "Rename…";
        delete: "namedPreset.delete" = "Delete";
        save_as_new: "namedPreset.saveAsNew" = "Save as New Preset…";
        manage: "namedPreset.manage" = "Manage presets";
        name: "namedPreset.name" = "Name";
        ok: "namedPreset.ok" = "OK";
        custom: "namedPreset.custom" = "Custom";
        built_in: "namedPreset.builtIn" = "{name} (built-in)";
        name_taken: "namedPreset.nameTaken" = "A preset with this name already exists.";
        name_invalid: "namedPreset.nameInvalid" = "This name can't be used.";
    }
}

/// How a [`NamedPresetRow`] lays out its entries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NamedPresetStyle {
    /// Secondary buttons after the field: Load…, Save…, New… (Photoshop's Contour Editor,
    /// PhotoCraft). Rename… and Delete are the field's context menu.
    #[default]
    Buttons,
    /// One "Manage presets" icon button with a menu: Save as New Preset…, Rename…, Delete, then
    /// the file entries (Illustrator's Image Trace, LightCraft, SoundCraft).
    Menu,
}

/// What the user did in a [`NamedPresetRow`], in order. The app does the work and makes one undo
/// step for each (typing in the Name field is a draft, not an edit).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum NamedPresetEvent<K> {
    /// A preset was picked in the field: the app applies it.
    Apply(K),
    /// New… or Save as New Preset… committed `name` (trimmed; an empty name is the suggestion).
    Create { name: String },
    /// Rename… committed `name` for `key`.
    Rename { key: K, name: String },
    /// Delete was chosen for `key`; the row acts at once (the app confirms if it wants to).
    Delete(K),
    /// A file pick finished.
    File(PresetFileEvent),
}

/// Why a typed name cannot be used.
type CheckName<'a> = Box<dyn Fn(&str) -> Option<PresetError> + 'a>;

/// What [`NamedPresetRow::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the app carries out the row's events"]
pub struct NamedPresetOutput<K> {
    /// The row.
    pub response: Response,
    /// What happened this pass, in order.
    pub events: Vec<NamedPresetEvent<K>>,
    /// A file job's status line, for the status bar.
    pub status: Option<StatusLine>,
}

/// One row that names and manages presets of a dialog's settings: a [`PresetPicker`] field
/// showing the current preset ("Custom" when nothing matches), the name prompt, and the entries
/// New…, Rename…, Delete, Load… and Save…. The Contour Editor, Image Trace, curve presets and
/// plugin presets.
///
/// ```ignore
/// let out = NamedPresetRow::new("contour-editor", "Contour preset", &rows)
///     .selected(current.as_ref())                  // None shows "Custom"
///     .style(NamedPresetStyle::Buttons)
///     .suggest_name(format!("Contour {}", user_count + 1))
///     .check_name(|name| library.name_error(name))
///     .built_in(|key| is_built_in(key))
///     .files(&contour_files)
///     .show(ui, |painter, rect, info| paint_contour(painter, rect, info.key));
/// for event in out.events { /* Apply, Create, Rename, Delete, File */ }
/// ```
///
/// - **Naming.** New… (or Save as New Preset…) opens the name row below the field, focused, the
///   suggested name selected; Rename… opens it on the selected user preset with its name. Enter
///   and OK commit; Esc cancels the name row only (the Name field owns Enter and Esc through
///   `focus::key_owner`, so a second Esc reaches the dialog); losing focus any other way keeps
///   the draft. An empty name creates the suggestion (PhotoCraft's "Contour"); a name
///   `check_name` refuses shows the error text under the field (`status.bad.fg`, the field's
///   description, polite live region) and blocks the commit. F2 on the field starts Rename…
///   when a user preset is selected.
/// - **Built-in presets** (`built_in`): Rename… and Delete are disabled; the closed field reads
///   "{name} (built-in)" ([`NamedPresetLabels::built_in`]) only when the app sets
///   `built_in`.
/// - **Files**: the entries of [`PresetFiles`]; their picks finish inside the row and come back
///   as `File` events with a status line.
/// - **Layout.** One row with `preset.toolbar_gap` gaps: field, then while naming the Name
///   label and a `TextField`
///   [`PresetStyle::name_width`](crate::presets::style::PresetStyle::name_width) wide, then the
///   buttons or the Manage button. When the width cannot hold them, the buttons wrap to a second
///   line. Tab visits the field, the Name field and the buttons in reading order; right to left
///   mirrors the order.
/// - **AccessKit.** A group named by the label; the Name field named [`NamedPresetLabels::name`];
///   the Manage button named "Manage presets" with `has_popup`.
///
/// [`PresetPicker`]: crate::presets::picker::PresetPicker
#[must_use = "show the row with `show`"]
pub struct NamedPresetRow<'a, K> {
    id_salt: IdSalt,
    label: String,
    rows: &'a [PresetRow<K>],
    selected: Option<&'a K>,
    style: NamedPresetStyle,
    field: PickerField,
    suggest_name: Option<String>,
    check_name: Option<CheckName<'a>>,
    built_in: Option<KeyTest<'a, K>>,
    files: Option<&'a PresetFiles<'a, K>>,
    labels: Option<NamedPresetLabels>,
}

impl<'a, K> NamedPresetRow<'a, K> {
    /// A row named `label` over the presets `rows` (built-in rows first, then the user's).
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, rows: &'a [PresetRow<K>]) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            rows,
            selected: None,
            style: NamedPresetStyle::default(),
            field: PickerField::Name,
            suggest_name: None,
            check_name: None,
            built_in: None,
            files: None,
            labels: None,
        }
    }

    /// The preset the settings match; `None` shows "Custom".
    pub fn selected(mut self, selected: Option<&'a K>) -> Self {
        self.selected = selected;
        self
    }

    /// Buttons (default) or a Manage menu.
    pub fn style(mut self, style: NamedPresetStyle) -> Self {
        self.style = style;
        self
    }

    /// How the field shows the current preset (default its name; the Contour Editor shows a
    /// square thumbnail).
    pub fn field(mut self, field: PickerField) -> Self {
        self.field = field;
        self
    }

    /// The name New… suggests and an empty name creates ("Contour 3").
    pub fn suggest_name(mut self, name: impl Into<String>) -> Self {
        self.suggest_name = Some(name.into());
        self
    }

    /// Why a typed name cannot be used (`PresetLibrary::name_error`); `None` accepts it.
    pub fn check_name(mut self, check: impl Fn(&str) -> Option<PresetError> + 'a) -> Self {
        self.check_name = Some(Box::new(check));
        self
    }

    /// Which presets are built in: Rename… and Delete are disabled for them.
    pub fn built_in(mut self, built_in: impl Fn(&K) -> bool + 'a) -> Self {
        self.built_in = Some(Rc::new(built_in));
        self
    }

    /// The file entries.
    pub fn files(mut self, files: &'a PresetFiles<'a, K>) -> Self {
        self.files = Some(files);
        self
    }

    /// The row's words (default English).
    pub fn labels(mut self, labels: NamedPresetLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Shows the row across the available width; `thumb` paints a preset's thumbnail (for a
    /// thumbnail field and its popup).
    pub fn show(self, ui: &mut Ui, thumb: impl FnMut(&Painter, Rect, ThumbInfo<'_, K>)) -> NamedPresetOutput<K> {
        // pc: crates/ui-egui/src/dialogs/layer_style/contour.rs:395-518 (`editor`: Load…, Save…, New… :444-454, Name row :408, suggestion :462, empty name "Contour"); the Name field's width is `PresetStyle::name_width`, the buttons size to their labels
        todo!()
    }
}

#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod tests {
    //! Screen tests of the named preset row and of the picker with files (task PR7).
    #![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

    // New… opens the Name field focused with the whole suggestion selected.
    #[test]
    #[ignore = "skeleton"]
    fn named_row_new_opens_the_name_row_with_the_suggestion_selected() {
        todo!()
    }

    // Enter reports `Create`; Esc closes the name row and leaves the dialog's Esc unread.
    #[test]
    #[ignore = "skeleton"]
    fn named_row_enter_creates_and_escape_cancels_only_the_name_row() {
        todo!()
    }

    // A name `check_name` refuses shows `name_taken` under the field and Enter reports nothing.
    #[test]
    #[ignore = "skeleton"]
    fn named_row_taken_name_shows_the_error_and_blocks_create() {
        todo!()
    }

    // Enter on an empty field reports `Create` with the suggested name.
    #[test]
    #[ignore = "skeleton"]
    fn named_row_empty_name_creates_the_suggestion() {
        todo!()
    }

    // With `selected(None)` the field reads "Custom" and its AccessKit name ends in "Custom".
    #[test]
    #[ignore = "skeleton"]
    fn named_row_custom_shows_when_nothing_is_selected() {
        todo!()
    }

    // `Menu` style: the Manage menu lists Save as New Preset…, Rename…, Delete; the last two are disabled on a built-in preset.
    #[test]
    #[ignore = "skeleton"]
    fn named_row_menu_style_lists_save_as_new_rename_delete_and_disables_for_built_in() {
        todo!()
    }

    // At 220 pt wide, naming, the buttons wrap below the field.
    #[test]
    #[ignore = "skeleton"]
    fn named_row_buttons_style_wraps_to_two_lines_when_narrow() {
        todo!()
    }

    // A fake `Files`: Load… and Replace… report `Loaded` with the bytes and `replace`; Save… writes the export; a second pick forgets the first ticket.
    #[test]
    #[ignore = "skeleton"]
    fn picker_files_entries_load_replace_and_save_through_files() {
        todo!()
    }
}
