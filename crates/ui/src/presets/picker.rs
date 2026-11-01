//! The preset picker: a closed field that shows the current preset and opens a grid or list of presets (`PresetPicker`, `PickerField`, `PickerThumb`).

#![expect(dead_code, unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::rc::Rc;

use egui::{AsIdSalt, IdSalt, Painter, Rect, Response, Ui};

use super::menu::{KeyTest, PresetFiles, PresetFilesOutput};
use super::{CellShape, PresetEvent, PresetRow, PresetView, ThumbInfo};

/// The shape of the thumbnail in a picker's closed field. The sizes are tokens, so an app names
/// a shape, never a size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PickerThumb {
    /// `size.preset.picker_square` (40): patterns, contours.
    #[default]
    Square,
    /// `size.preset.picker_wide` (36 × 22): custom shapes.
    Wide,
}

/// What a picker's closed field shows, always followed by a chevron at the end.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PickerField {
    /// The current preset's thumbnail (PhotoCraft's pattern, shape and Contour pickers).
    Thumbnail(PickerThumb),
    /// The current preset's name, as a `Select` box (curve presets, FilmCraft's export presets).
    Name,
    /// The thumbnail, then the name.
    ThumbnailAndName(PickerThumb),
}

impl Default for PickerField {
    fn default() -> Self {
        PickerField::Thumbnail(PickerThumb::Square)
    }
}

/// When a click in the popup closes it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PickerClose {
    /// A click picks and closes (PhotoCraft's pattern and shape pickers).
    #[default]
    OnSelect,
    /// A click picks and keeps the popup open; a double-click or Enter closes it (PhotoCraft's
    /// Brush Preset picker, #1031).
    OnActivate,
}

crate::words! {
    /// The words of a [`PresetPicker`]. `{label}` stands for the picker's label and `{name}` for
    /// the current preset's name.
    pub struct PresetPickerLabels {
        custom: "presetPicker.custom" = "Custom";
        field: "presetPicker.field" = "{label}: {name}";
    }
}

/// What [`PresetPicker::show`] returns.
#[derive(Debug)]
#[non_exhaustive]
#[must_use = "the app applies the picked preset"]
pub struct PresetPickerResponse<K> {
    /// The closed field.
    pub response: Response,
    /// A preset was picked this pass (a click, Space, or Enter): the app sets it as one edit.
    pub picked: Option<K>,
    /// A double-click or Enter in an [`OnActivate`](PickerClose::OnActivate) picker; it closed.
    pub activated: Option<K>,
    /// With [`split`](PresetPicker::split): the thumbnail area was clicked (Contour opens its
    /// editor); only the chevron toggles the popup.
    pub field_clicked: bool,
    /// The popup browser's Rename and Delete events (its context menu), for the app to carry out.
    pub events: Vec<PresetEvent<K>>,
    /// A file pick of the picker's file entries finished.
    pub files: PresetFilesOutput,
}

/// A field that shows the current preset and opens a popover with a [`PresetBrowser`] of
/// presets: patterns, shapes, contours, curve presets, gallery presets.
///
/// ```ignore
/// let pick = PresetPicker::new(("fill-pattern", id), "Pattern", &rows)
///     .selected(Some(&current))
///     .columns(6)
///     .field(PickerField::Thumbnail(PickerThumb::Square))
///     .show(ui, |painter, rect, info| paint_pattern(painter, rect, info.key));
/// if let Some(key) = pick.picked { set_pattern(key); }
/// ```
///
/// - **Field.** A `Select`-style box (rest, hover, open and focus looks; `field.fill`,
///   `field.border`) holding the thumbnail ([`PickerThumb`] size), the name in `Name` and
///   `ThumbnailAndName` modes ("Custom" with nothing selected), and a chevron at the end. One Tab
///   stop. A click toggles the popover; Down, Alt+Down, Enter or Space open it.
/// - **Popup.** A `Popover` card holding a `PresetBrowser` in grid view (or `view(List)`), no
///   toolbar and no footer, `shrink_to_content`, at most `size.preset.popup_max_height` (320)
///   high, `columns` cells per row of `size.preset.picker_cell` (40) with the browser's cell look
///   (2 pt accent ring on the selected cell). Focus moves into the browser (arrows move the lead,
///   Space selects, Enter activates); Esc and a press outside close it and return focus to the
///   field. Group headers show for grouped rows. Its view state lives in egui memory: the picker
///   has nothing for the app to store.
/// - **Picking.** [`PickerClose::OnSelect`]: a click picks and closes. `OnActivate`: a click
///   picks and stays open, a double-click or Enter reports `activated` and closes. Hover shows
///   the name as a tooltip; right-click opens the browser's context menu (Rename…, Delete) when
///   `can_rename` or `can_delete` allow it.
/// - **Files.** With [`files`](Self::files), the popup ends with a separator and the file
///   entries (Contour's Load, Replace and Save Contours…); their picks finish inside the picker.
/// - **Right to left.** The chevron mirrors to the start side of the end edge; the popover opens
///   from the field's start edge.
/// - **AccessKit.** The field is a `Button` with `has_popup` and `expanded`, named
///   [`PresetPickerLabels::field`] ("Pattern: Bubbles", "Pattern: Custom"); the popover is a
///   non-modal `Dialog` named by the label.
///
/// [`PresetBrowser`]: super::PresetBrowser
#[must_use = "show the picker with `show`"]
pub struct PresetPicker<'a, K> {
    id_salt: IdSalt,
    label: String,
    rows: &'a [PresetRow<K>],
    selected: Option<&'a K>,
    columns: usize,
    field: PickerField,
    close: PickerClose,
    view: PresetView,
    shape: CellShape,
    can_rename: Option<KeyTest<'a, K>>,
    can_delete: Option<KeyTest<'a, K>>,
    files: Option<&'a PresetFiles<'a, K>>,
    split: bool,
    labels: Option<PresetPickerLabels>,
}

impl<'a, K> PresetPicker<'a, K> {
    /// A picker named `label` over `rows`.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, rows: &'a [PresetRow<K>]) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            rows,
            selected: None,
            columns: 6,
            field: PickerField::default(),
            close: PickerClose::default(),
            view: PresetView::Grid,
            shape: CellShape::Square,
            can_rename: None,
            can_delete: None,
            files: None,
            split: false,
            labels: None,
        }
    }

    /// The current preset; `None` shows "Custom".
    pub fn selected(mut self, selected: Option<&'a K>) -> Self {
        self.selected = selected;
        self
    }

    /// Cells per row in the popup's grid (default 6: patterns 6, Contour 4, shapes 7); a count,
    /// not a size.
    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = columns.max(1);
        self
    }

    /// What the closed field shows (default a square thumbnail).
    pub fn field(mut self, field: PickerField) -> Self {
        self.field = field;
        self
    }

    /// When a click closes the popup (default on select).
    pub fn close(mut self, close: PickerClose) -> Self {
        self.close = close;
        self
    }

    /// The popup's view (default grid; names lists use `List`).
    pub fn view(mut self, view: PresetView) -> Self {
        self.view = view;
        self
    }

    /// The shape of a popup cell's thumbnail (default square).
    pub fn cell_shape(mut self, shape: CellShape) -> Self {
        self.shape = shape;
        self
    }

    /// Which presets the popup's context menu may rename (default: none).
    pub fn can_rename(mut self, can_rename: impl Fn(&K) -> bool + 'a) -> Self {
        self.can_rename = Some(Rc::new(can_rename));
        self
    }

    /// Which presets the popup's context menu may delete (default: none; user presets only in
    /// Contour).
    pub fn can_delete(mut self, can_delete: impl Fn(&K) -> bool + 'a) -> Self {
        self.can_delete = Some(Rc::new(can_delete));
        self
    }

    /// The file entries at the popup's end.
    pub fn files(mut self, files: &'a PresetFiles<'a, K>) -> Self {
        self.files = Some(files);
        self
    }

    /// Two zones (Lead call 12): a click on the thumbnail area reports
    /// [`field_clicked`](PresetPickerResponse::field_clicked) and only the chevron toggles the
    /// popup, as `split_button`'s two zones (PhotoCraft's Contour row opens the Contour Editor
    /// from its thumbnail).
    pub fn split(mut self, split: bool) -> Self {
        self.split = split;
        self
    }

    /// The picker's words (default English).
    pub fn labels(mut self, labels: PresetPickerLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Shows the field and, while open, its popover; `thumb` paints a preset's thumbnail (the
    /// field's and the cells').
    pub fn show(self, ui: &mut Ui, thumb: impl FnMut(&Painter, Rect, ThumbInfo<'_, K>)) -> PresetPickerResponse<K> {
        // pc: crates/ui-egui/src/panels/preset_panels.rs:256-310 (`pattern_picker`, `pattern_grid`: 40 pt, 4 pt gaps, 6 columns); crates/ui-egui/src/panels/preset_panels/panels.rs:334-380 (`shape_picker`: 36 × 22, max 320 :356); crates/ui-egui/src/dialogs/layer_style/contour.rs:93-198 (`row` and its popup, 4 columns)
        todo!()
    }
}

#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod tests {
    //! Screen tests of the picker (task PR4).
    #![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

    // Tab to the field, Down opens the popover with focus inside, arrows and Enter pick, focus returns to the field.
    #[test]
    #[ignore = "skeleton"]
    fn picker_opens_with_down_and_picks_with_enter_and_returns_focus() {
        todo!()
    }

    // `OnSelect`: a click picks and closes; `OnActivate`: a click picks and stays open, a double-click reports `activated` and closes.
    #[test]
    #[ignore = "skeleton"]
    fn picker_click_picks_and_closes_unless_it_keeps_open() {
        todo!()
    }

    // With `columns(6)` the popup lays out 6 cells of `size.preset.picker_cell` per row.
    #[test]
    #[ignore = "skeleton"]
    fn picker_popup_shows_columns_cells_per_row() {
        todo!()
    }

    // The popup's context menu offers Delete only where `can_delete` allows it, and reports `PresetEvent::Delete`.
    #[test]
    #[ignore = "skeleton"]
    fn picker_delete_applies_only_to_user_presets() {
        todo!()
    }
}
