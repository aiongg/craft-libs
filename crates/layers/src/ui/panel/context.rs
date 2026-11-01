//! What the panel's parts share in one pass: the tree, the settings, the thumbnails, the words,
//! the style, and a sink for what row closures report while the list draws them.
//!
//! Built by `LayersPanel::show` (L8) once per pass and lent to the row mapping (`row`,
//! `indicators`), the body (`body`), the drag rules (`drag`), the footer and the menus.

use std::cell::RefCell;

use craft_ui::canvas::textures::{ThumbSource, Thumbnails};
use craft_ui::list::ListRowStyle;
use egui::{Context, Id, KeyboardShortcut, Rect, Ui};

use super::events::{LayersEvent, MenuTarget};
use super::labels::LayersLabels;
use super::style::LayersStyle;
use crate::ui::options::LayersOptions;
use crate::ui::thumbs::LayerThumb;
use crate::ui::tree::{LayerInfo, LayerPart, LayerRowKey, LayerSelection, LayerTree};

/// The app's thumbnail cache and source, from [`LayersPanel::thumbnails`](super::LayersPanel::thumbnails).
pub(super) struct ThumbHost<'a, Id: Clone + Eq + std::hash::Hash + Send + Sync + 'static> {
    pub(super) thumbnails: &'a Thumbnails<LayerThumb<Id>>,
    pub(super) source: &'a dyn ThumbSource<LayerThumb<Id>>,
}

impl<Id: Clone + Eq + std::hash::Hash + Send + Sync + 'static> Clone for ThumbHost<'_, Id> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Id: Clone + Eq + std::hash::Hash + Send + Sync + 'static> Copy for ThumbHost<'_, Id> {}

/// The app's furniture at a row's trailing end (Illustrator's target circle, a selection square),
/// from [`LayersPanel::trailing`](super::LayersPanel::trailing): drawn in a `width` wide cell
/// after the indicators, never dropped for the name.
pub(super) struct Trailing<'a, Id> {
    pub(super) width: f32,
    pub(super) add: Box<TrailingAdd<'a, Id>>,
}

/// The closure of a [`Trailing`] cell: `add(ui, id, info)`.
pub(super) type TrailingAdd<'a, Id> = dyn Fn(&mut Ui, &Id, &LayerInfo) + 'a;

/// The host's row menu, from [`LayersPanel::row_menu`](super::LayersPanel::row_menu).
pub(super) type RowMenu<'a, Id> = dyn Fn(&MenuTarget<Id>) -> Vec<craft_ui::menu::MenuEntry<String>> + 'a;

/// What row closures report while the list draws: events of the indicators (the lock
/// indicator's unlock, the effects triangle) and where each thumbnail was drawn, which the body
/// reads after the list (a secondary click on a mask thumbnail opens the mask menu with `part`).
pub(super) struct RowSink<Id> {
    /// Events reported from inside row closures, in the order they happened.
    pub(super) events: Vec<LayersEvent<Id>>,
    /// Each thumbnail drawn this pass: its row, its part and its rect.
    pub(super) thumbs: Vec<(LayerRowKey<Id>, LayerPart, Rect)>,
    /// The first and last display position the list built a row for this pass, so the body can
    /// ask for the thumbnails one viewport beyond at `Priority::NEAR`.
    pub(super) built: Option<(usize, usize)>,
}

impl<Id> Default for RowSink<Id> {
    fn default() -> Self {
        Self { events: Vec::new(), thumbs: Vec::new(), built: None }
    }
}

/// One pass's shared state of the panel.
pub(super) struct PanelContext<'a, T: LayerTree> {
    /// The context, for `Thumbnails::state` inside the list's row builder.
    pub(super) ctx: Context,
    /// The panel's id (from its `id_salt`): the list, the menus and the drag keep state under it.
    pub(super) id: Id,
    /// The app's view.
    pub(super) tree: &'a T,
    /// `tree.selection()`, read once per pass.
    pub(super) selection: LayerSelection<T::Id>,
    /// The Panel Options in force.
    pub(super) options: &'a LayersOptions,
    /// The thumbnail cache and source; `None` draws kind tiles and placeholders only.
    pub(super) thumbs: Option<ThumbHost<'a, T::Id>>,
    /// The panel's words.
    pub(super) labels: &'a LayersLabels,
    /// The panel's own style.
    pub(super) style: LayersStyle,
    /// The list rows' style (sizes for thumbnails, toggle columns, trailing items).
    pub(super) row_style: ListRowStyle,
    /// A lock toggle column after the eye (VectorCraft, DesignCraft).
    pub(super) lock_column: bool,
    /// The app's shortcut that toggles the eyes of the selected rows (PhotoCraft's Cmd+,).
    pub(super) visibility_key: Option<KeyboardShortcut>,
    /// The app's trailing furniture.
    pub(super) trailing: Option<&'a Trailing<'a, T::Id>>,
    /// What row closures report.
    pub(super) sink: RefCell<RowSink<T::Id>>,
}
