//! `PresetBrowser`: a library of named presets shown as thumbnails (DS-44), with groups, a grid
//! or list view, a size slider, search, rename and drag to reorder.
//!
//! The host keeps the library and every command; the browser draws, senses and reports
//! [`PresetEvent`]s. This module holds the data it meets the host through: [`PresetRow`] (the
//! host's flat display list), [`PresetState`] (the view settings the host stores),
//! [`PresetEvent`] and [`PresetLabels`]; [`layout`] is the pure geometry of the rows.

#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod bars_tests;
pub mod drag;
pub mod event;
pub mod footer;
pub mod labels;
pub(crate) mod layout;
pub mod menu;
pub mod model;
pub mod rename;
pub mod state;
pub(crate) mod stories;
pub mod style;
pub mod toolbar;
pub mod view;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod view_tests;

use std::fmt::Debug;
use std::hash::Hash;
use std::ops::RangeInclusive;
use std::rc::Rc;

use egui::{AsIdSalt, IdSalt, Painter, Rect, Ui};

use crate::composites::EmptyState;

pub use drag::PresetReorder;
pub use event::{PresetEvent, PresetOutput};
pub use footer::{FooterEvent, PresetFooter};
pub use labels::PresetLabels;
pub use menu::PresetExtra;
pub use model::{CellShape, PresetKind, PresetRow, Preview, ThumbInfo};
pub use state::{DefaultOpen, PresetState, PresetView};
pub use style::PresetStyle;

use menu::KeyTest;

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
    labels: Option<Rc<PresetLabels>>,
    can_rename: Option<KeyTest<'a, K>>,
    can_delete: Option<KeyTest<'a, K>>,
    can_move: Option<KeyTest<'a, K>>,
    extra_items: Vec<PresetExtra>,
    toolbar: bool,
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
            labels: None,
            can_rename: None,
            can_delete: None,
            can_move: None,
            extra_items: Vec::new(),
            toolbar: false,
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
        self.labels = Some(Rc::new(labels));
        self
    }

    /// Where F2 may start a rename (default: everywhere).
    pub fn can_rename(mut self, can_rename: impl Fn(&K) -> bool + 'a) -> Self {
        self.can_rename = Some(Rc::new(can_rename));
        self
    }

    /// Which presets and groups the context menu may delete (default: all); the entry is
    /// disabled for the others.
    pub fn can_delete(mut self, can_delete: impl Fn(&K) -> bool + 'a) -> Self {
        self.can_delete = Some(Rc::new(can_delete));
        self
    }

    /// Which presets the context menu offers "Move to" for (default: all); a library without
    /// groups has no such entry.
    pub fn can_move(mut self, can_move: impl Fn(&K) -> bool + 'a) -> Self {
        self.can_move = Some(Rc::new(can_move));
        self
    }

    /// Entries added to the context menu after the standard ones; choosing one reports
    /// [`PresetEvent::Extra`].
    pub fn extra_items(mut self, items: impl IntoIterator<Item = PresetExtra>) -> Self {
        self.extra_items = items.into_iter().collect();
        self
    }

    /// A row above the list with the search field and the grid and list buttons (the buttons
    /// are left out under [`view_lock`](PresetBrowser::view_lock)). Without it there is no
    /// search; the host puts the view entries in its panel menu with
    /// [`PresetState::view_menu_items`].
    pub fn toolbar(mut self, toolbar: bool) -> Self {
        self.toolbar = toolbar;
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

    /// Shows the browser across the available width: the toolbar if asked for, the rename bar
    /// while a rename is open, then the presets, or a message when there are none to show.
    /// `thumb` paints the thumbnail of a preset into a rect, clipped to it; it runs for the cells
    /// and rows in view and for the drag's ghost.
    pub fn show(mut self, ui: &mut Ui, thumb: impl FnMut(&Painter, Rect, ThumbInfo<'_, K>)) -> PresetOutput<K> {
        let id = ui.make_persistent_id(self.id_salt);
        let rows = self.rows;
        let labels = self.labels.clone().unwrap_or_else(|| Rc::new(<PresetLabels as crate::localized::Labels>::of(ui)));
        let (can_rename, can_delete, can_move) = (self.can_rename.clone(), self.can_delete.clone(), self.can_move.clone());
        let extras = std::mem::take(&mut self.extra_items);
        if self.toolbar {
            toolbar::show(ui, self.state, &labels, self.view_lock.is_none());
        }
        let renamed = bars(ui, id, rows, &labels, self.start_rename.take());
        let filtering = self.state.filtering();
        let message = if rows.is_empty() && !filtering {
            Some(&labels.empty)
        } else if filtering && layout::shown(rows, self.state, self.default_open).is_empty() {
            Some(&labels.no_match)
        } else {
            None
        };
        let mut out = match message {
            Some(text) => {
                let response = EmptyState::new(text.as_str()).status_if(filtering).id_salt(id.with("empty")).show(ui).response;
                PresetOutput { response, events: Vec::new(), drag: None }
            }
            None => view::show(self, id, ui, thumb),
        };
        if message.is_none() {
            let spec = menu::Spec {
                rows,
                labels: &labels,
                can_rename: can_rename.as_ref(),
                can_delete: can_delete.as_ref(),
                can_move: can_move.as_ref(),
                extras: &extras,
            };
            menu::show(ui, id, &out.response, &spec, &mut out.events);
        }
        out.events.splice(0..0, renamed);
        out
    }
}

/// The rename bar of the browser `id`, when a rename is open (or `start` opens one): it ends
/// with the `Rename` event if the name changed.
fn bars<K>(ui: &mut Ui, id: egui::Id, rows: &[PresetRow<K>], labels: &PresetLabels, start: Option<K>) -> Option<PresetEvent<K>>
where
    K: Clone + Eq + Hash + Debug + Send + Sync + 'static,
{
    let mut st = view::State::<K>::load(ui.ctx(), id);
    if let Some(key) = start.filter(|key| rows.iter().any(|r| r.key == *key)) {
        st.renaming = Some(key);
    }
    let row = st.renaming.as_ref().and_then(|key| rows.iter().find(|r| r.key == *key));
    let Some(row) = row else {
        st.renaming = None;
        st.store(ui.ctx(), id);
        return None;
    };
    let ended = rename::show(ui, id.with("rename-bar"), id.with(&row.key), &row.label, labels);
    let mut event = None;
    if let Some(ended) = ended {
        let key = st.renaming.take();
        if let (rename::Ended::Commit(text), Some(key)) = (ended, key) {
            let text = text.trim().to_owned();
            if !text.is_empty() && text != row.label {
                event = Some(PresetEvent::Rename { key, text });
            }
        }
        ui.memory_mut(|m| m.request_focus(id));
    }
    st.store(ui.ctx(), id);
    event
}
