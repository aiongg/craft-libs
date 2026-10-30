//! `PresetBrowser`: a library of named presets shown as thumbnails (DS-44), with groups, a grid
//! or list view, a size slider, search, rename and drag to reorder.
//!
//! The host keeps the library and every command; the browser draws, senses and reports
//! [`PresetEvent`]s. This module holds the data it meets the host through: [`PresetRow`] (the
//! host's flat display list), [`PresetState`] (the view settings the host stores),
//! [`PresetEvent`] and [`PresetLabels`]; [`layout`] is the pure geometry of the rows.

pub mod drag;
pub mod event;
pub mod footer;
pub mod labels;
pub(crate) mod layout;
pub mod menu;
pub mod model;
pub mod rename;
pub mod state;
pub mod style;
pub mod toolbar;
pub mod view;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod view_tests;

use std::fmt::Debug;
use std::hash::Hash;
use std::ops::RangeInclusive;

use egui::{AsIdSalt, IdSalt, Painter, Rect, Ui};

pub use drag::PresetReorder;
pub use event::{PresetEvent, PresetOutput};
pub use labels::PresetLabels;
pub use model::{CellShape, PresetKind, PresetRow, Preview, ThumbInfo};
pub use state::{DefaultOpen, PresetState, PresetView};
pub use style::PresetStyle;

/// A library of named presets shown as thumbnails: groups (with folders), a grid or a list, a
/// thumbnail size, drag to reorder. The host keeps the library; the browser draws, senses and
/// reports [`PresetEvent`]s.
///
/// ```
/// use craft_ui::presets::{PresetBrowser, PresetRow, PresetState};
/// # egui::__run_test_ui(|ui| {
/// let mut state = PresetState::new();
/// let rows = [PresetRow::group("g", "Default", 0), PresetRow::item("red", "Red", 1)];
/// let out = PresetBrowser::new("swatches", "Swatches", &mut state, &rows)
///     .selected(Some(&"red"))
///     .show(ui, |painter, rect, info| {
///         painter.rect_filled(rect, 0.0, egui::Color32::RED);
///     });
/// for event in out.events {
///     // The host applies `Select`, `Activate`, `Reorder`, ...
/// }
/// # });
/// ```
///
/// - **Rows.** Headers are tree rows; in list view presets are list rows, in grid view they
///   wrap in cells of the thumbnail size, a run of presets of one depth to a block. Only the
///   rows in view are laid out, and the thumbnail closure runs for those cells and rows.
/// - **Keys.** One Tab stop. The arrows move the lead, which is where the focus ring sits,
///   without selecting; Space selects it, Enter activates it (a header: toggles it).
/// - **Drag.** With [`reorder`](PresetBrowser::reorder), presets and groups drag; the drop
///   target shows a vertical line at a cell's edge, a line along a row, or an outline for a drop
///   into a group, and the result arrives as `Reorder`.
/// - **AccessKit.** A `Tree` (a `ListBox` without groups) of `TreeItem`s (`ListBoxOption`s) with
///   `selected`, `level`, `expanded` and positions in their set.
#[must_use = "show the browser with `show`"]
pub struct PresetBrowser<'a, K> {
    id_salt: IdSalt,
    label: String,
    state: &'a mut PresetState<K>,
    rows: &'a [PresetRow<K>],
    selected: Option<&'a K>,
    shape: CellShape,
    flush: bool,
    thumb_range: RangeInclusive<f32>,
    default_open: DefaultOpen,
    reorder: Option<PresetReorder<'a, K>>,
    labels: PresetLabels,
    can_rename: Option<Box<dyn Fn(&K) -> bool + 'a>>,
    start_rename: Option<K>,
    view_lock: Option<PresetView>,
    max_height: Option<f32>,
}

impl<'a, K: Clone + Eq + Hash + Debug + Send + Sync + 'static> PresetBrowser<'a, K> {
    /// A browser of `rows`, the host's flat display list, keeping its view settings in `state`.
    /// `label` names it for a screen reader.
    pub fn new(id_salt: impl AsIdSalt, label: impl Into<String>, state: &'a mut PresetState<K>, rows: &'a [PresetRow<K>]) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            label: label.into(),
            state,
            rows,
            selected: None,
            shape: CellShape::Square,
            flush: false,
            thumb_range: 16.0..=96.0,
            default_open: DefaultOpen::All,
            reorder: None,
            labels: PresetLabels::default(),
            can_rename: None,
            start_rename: None,
            view_lock: None,
            max_height: None,
        }
    }

    /// The selected preset, which the host keeps: its cell or row shows the selection.
    pub fn selected(mut self, selected: Option<&'a K>) -> Self {
        self.selected = selected;
        self
    }

    /// The shape of a grid cell's thumbnail (default square).
    pub fn cell_shape(mut self, shape: CellShape) -> Self {
        self.shape = shape;
        self
    }

    /// A grid without gaps, inset or frames, as Photoshop's Swatches; only the selected and
    /// hovered cell are outlined.
    pub fn flush(mut self, flush: bool) -> Self {
        self.flush = flush;
        self
    }

    /// The range the thumbnail width may take, in points (default 16 to 96): the state's width is
    /// clamped to it.
    pub fn thumb_range(mut self, range: RangeInclusive<f32>) -> Self {
        self.thumb_range = range;
        self
    }

    /// Which groups are open before the user toggles any (default all).
    pub fn default_open(mut self, default_open: DefaultOpen) -> Self {
        self.default_open = default_open;
        self
    }

    /// Presets (and groups) can be dragged and moved with Cmd/Ctrl+] and [ by `reorder`'s rules.
    pub fn reorder(mut self, reorder: PresetReorder<'a, K>) -> Self {
        self.reorder = Some(reorder);
        self
    }

    /// The browser's words (default English).
    pub fn labels(mut self, labels: PresetLabels) -> Self {
        self.labels = labels;
        self
    }

    /// Where F2 may start a rename (default: everywhere).
    pub fn can_rename(mut self, can_rename: impl Fn(&K) -> bool + 'a) -> Self {
        self.can_rename = Some(Box::new(can_rename));
        self
    }

    /// Opens the rename of the preset or group `key`, as the host asks after creating one.
    pub fn start_rename(mut self, key: K) -> Self {
        self.start_rename = Some(key);
        self
    }

    /// Fixes the view, whatever [`PresetState::view`] says.
    pub fn view_lock(mut self, view: PresetView) -> Self {
        self.view_lock = Some(view);
        self
    }

    /// The browser's greatest height; it fills the height available otherwise.
    pub fn max_height(mut self, height: f32) -> Self {
        self.max_height = Some(height);
        self
    }

    /// Shows the browser across the available width. `thumb` paints the thumbnail of a preset
    /// into a rect, clipped to it; it runs for the cells and rows in view and for the drag's
    /// ghost.
    pub fn show(self, ui: &mut Ui, thumb: impl FnMut(&Painter, Rect, ThumbInfo<'_, K>)) -> PresetOutput<K> {
        let id = ui.make_persistent_id(self.id_salt);
        view::show(self, id, ui, thumb)
    }
}
