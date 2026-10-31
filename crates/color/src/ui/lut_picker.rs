//! [`LutPicker`] (F04.04): the 3D LUT choice of a Color Lookup adjustment, as PhotoCraft's: Load
//! 3D LUT…, None, the built-in looks ([`craft_cms::lutfile::BUILTIN`]), the files of the app's
//! LUT folder, and the loaded LUT.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::path::PathBuf;

use egui::{AsIdSalt, IdSalt, Ui};

use craft_ui::edit::EditResponse;

craft_ui::words! {
    /// The LUT picker's rows, English by default. The built-in looks' names are [`LUT_WORDS`].
    #[non_exhaustive]
    pub struct LutLabels {
        label: "lut.label" = "3D LUT File";
        load: "lut.load" = "Load 3D LUT…";
        none: "lut.none" = "None";
        /// The loaded LUT's row; `{name}` its title, `{size}` its grid size ("Film (33³)").
        custom: "lut.custom" = "{name} ({size}³)";
    }
}

/// The built-in looks' names as words, keyed `lut.builtin.<id>` with
/// [`BUILTIN`](craft_cms::lutfile::BUILTIN)'s English, in its order.
pub const LUT_WORDS: &[craft_ui::localized::Word] = &[
    craft_ui::localized::Word::keyed("lut.builtin.warm", "Warm Filter"),
    craft_ui::localized::Word::keyed("lut.builtin.cool", "Cool Filter"),
    craft_ui::localized::Word::keyed("lut.builtin.tealOrange", "Teal & Orange"),
    craft_ui::localized::Word::keyed("lut.builtin.bleachBypass", "Bleach Bypass"),
    craft_ui::localized::Word::keyed("lut.builtin.fadedFilm", "Faded Film"),
    craft_ui::localized::Word::keyed("lut.builtin.dayForNight", "Day for Night"),
    craft_ui::localized::Word::keyed("lut.builtin.monoContrast", "Mono High Contrast"),
    craft_ui::localized::Word::keyed("lut.builtin.crossProcess", "Cross Process"),
];

/// The chosen LUT.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LutChoice {
    None,
    /// A built-in look by id ([`craft_cms::lutfile::builtin`]).
    Builtin(&'static str),
    /// A file of the LUT folder; the app reads and embeds it.
    Folder(PathBuf),
    /// The LUT the layer holds, loaded before: its title and grid size.
    Loaded {
        name: String,
        size: usize,
    },
}

/// What [`LutPicker::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the host applies the choice"]
pub struct LutPickerResponse {
    /// A row was chosen ([`Set`](craft_ui::edit::EditEvent::Set)); while the wheel or the arrow
    /// keys step through the built-ins it is a gesture the host previews (Begin, Update, then
    /// Commit when the stepping stops).
    pub edit: EditResponse<LutChoice>,
    /// Load 3D LUT… was chosen: the app opens a `.cube`, `.3dl` or `.look` file (its file
    /// dialogs are craft-dialogs', which this crate does not depend on) and parses it with
    /// [`craft_cms::lutfile::parse`]; the choice does not change.
    pub load_requested: bool,
}

/// A [`Select`](craft_ui::controls::Select) of LUTs, 170 points wide in PhotoCraft.
///
/// - Rows: Load 3D LUT…, None, the eight built-in looks, the LUT folder's files by name (sorted),
///   and the loaded LUT ("name (33³)") when the layer holds one.
/// - The folder is scanned on a worker thread at most every 2 seconds (a network share can be
///   slow); until a scan ends the list shows the last one. Another folder drops the old list at
///   once. Web has no folder.
/// - The wheel and the arrow keys over the closed control step through None and the built-ins
///   only, as a previewed gesture (a folder file would be read from disk each step).
pub struct LutPicker<'a> {
    id_salt: IdSalt,
    value: &'a LutChoice,
    folder: Option<PathBuf>,
    enabled: bool,
}

impl<'a> LutPicker<'a> {
    pub fn new(id_salt: impl AsIdSalt, value: &'a LutChoice) -> LutPicker<'a> {
        LutPicker { id_salt: IdSalt::new(id_salt), value, folder: None, enabled: true }
    }

    /// The app's LUT folder (Preferences › File Handling › LUT Folder).
    pub fn folder(mut self, folder: Option<PathBuf>) -> LutPicker<'a> {
        self.folder = folder;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> LutPicker<'a> {
        self.enabled = enabled;
        self
    }

    // pc: crates/ui-egui/src/panels/adjust_ui.rs:175 `color_lookup_editor` (rows at :191, wheel
    // cycling at :200), :113 `folder_luts` (2 s rescan), :137 `start_folder_scan`, :96 `FOLDER_LUT`,
    // :85 `browse_color_lookup` (the app's side).
    pub fn show(self, ui: &mut Ui) -> LutPickerResponse {
        todo!()
    }
}
