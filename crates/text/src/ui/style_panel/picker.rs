//! [`StyleChip`] (N3.04): a design tool's named style field (FigCraft's, after Figma's): a chip
//! showing the style applied to the selection (or "Mixed", or nothing), with a detach button,
//! opening a picker popover of the document's styles as a list (preview, name, meta) or a tile
//! grid, grouped by folder, with a create button.

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use egui::{Color32, Response, Ui};

craft_ui::words! {
    /// The chip's and picker's words (FigCraft's strings).
    #[non_exhaustive]
    pub struct StylePickerLabels {
        mixed: "stylePicker.mixed" = "Mixed";
        detach: "stylePicker.detach" = "Detach style";
        apply_style: "stylePicker.apply" = "Apply style";
        create_style: "stylePicker.create" = "Create style";
        list_view: "stylePicker.listView" = "List view";
        grid_view: "stylePicker.gridView" = "Grid view";
        no_styles: "stylePicker.noStyles" = "No styles";
        edit_style: "stylePicker.edit" = "Edit style";
    }
}

/// What a style row shows before its name.
#[derive(Clone, Debug, PartialEq)]
pub enum StylePreview {
    /// A swatch (colour and fill styles).
    Color(Color32),
    /// A short sample drawn in the style ("Ag"), with the app's family and size already applied
    /// through a font id.
    Text { sample: String, font: egui::FontId },
    /// An icon from craft-ui's set (effect and grid styles).
    Icon(craft_ui::icons::Icon),
}

/// One style in the picker; its folder is the name's ([`crate::styles::style_folder`]).
#[derive(Clone, Debug, PartialEq)]
pub struct NamedStyleRow {
    pub id: u64,
    pub name: String,
    /// The second line ("Inter 16/24", "#1473E6").
    pub meta: String,
    pub preview: StylePreview,
}

/// How the picker lays styles out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum StylePickerView {
    #[default]
    List,
    /// Tiles of previews (colour styles), the name in the tooltip.
    Grid,
}

/// What the chip did this pass.
#[derive(Debug)]
pub struct StyleChipResponse {
    /// A style picked in the popover: apply it.
    pub picked: Option<u64>,
    /// The detach button: keep the values, drop the link.
    pub detach: bool,
    /// The create button: a new style from the selection's values (named
    /// [`crate::styles::next_style_name`]).
    pub create: bool,
    /// Edit from a row's context menu.
    pub edit: Option<u64>,
    pub response: Response,
}

/// The chip, built per pass.
///
/// - Shows the applied style's preview and leaf name, "Mixed" for several, or the app's own value
///   field when no style is applied ([`StyleChip::show`] returns and the app draws it).
/// - A click opens the picker popover under the chip: a header with the list/grid toggle and
///   create, then the styles by folder (top-level first, folder captions), the applied one
///   checked; ↑/↓ move, Enter applies, Esc closes; typing filters by name.
/// - Rows: preview, name and meta on two lines; a right-click opens Edit style.
pub struct StyleChip<'a> {
    id_salt: egui::IdSalt,
    styles: &'a [NamedStyleRow],
    applied: Option<Option<u64>>,
    view: &'a mut StylePickerView,
    labels: Option<StylePickerLabels>,
}

impl<'a> StyleChip<'a> {
    /// `applied`: `Some(Some(id))` one style, `Some(None)` mixed, `None` no style.
    pub fn new(id_salt: impl egui::AsIdSalt, styles: &'a [NamedStyleRow], applied: Option<Option<u64>>, view: &'a mut StylePickerView) -> Self {
        StyleChip { id_salt: egui::IdSalt::new(id_salt), styles, applied, view, labels: None }
    }

    pub fn labels(mut self, labels: StylePickerLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    // figcraft: crates/ui-egui/src/widgets/style_picker.rs:211 `chip`, :154 `Picker`, :341
    // `by_folder`.
    pub fn show(self, ui: &mut Ui) -> StyleChipResponse {
        todo!()
    }
}
