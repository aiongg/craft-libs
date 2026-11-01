//! Display order and filtered order, `List` wiring, selection and thumbnail-click rules, rename,
//! activate, reveal, keys, `min_width`.

#![expect(unused_variables, clippy::todo, reason = "skeleton: removed when the bodies land")]

use std::sync::Arc;

use craft_ui::behavior::{Move, Reorder, RowInfo, Selection};
use craft_ui::list::OpenChange;
use egui::{Context, Id, Modifiers, Response, Ui};

use super::context::PanelContext;
use super::events::LayersEvent;
use super::menu::MenuRequest;
use crate::ui::filter::LayerFilter;
use crate::ui::tree::{LayerInfo, LayerPart, LayerRowKey, LayerSelection, LayerTree};

/// The panel's rows in display order: [`LayerTree::children`] from the top level, topmost first,
/// a group above its children; children of a closed group (`is_open` false) left out; under a
/// layer whose `effects_open` is true, its Smart Filters rows (header, then items) and its
/// effects rows ([`LayerTree::sub_rows`]) as leaf rows one level deeper.
///
/// With an [`active`](LayerFilter::active) `filter`, every group's contents are listed whether
/// open or not, rows the filter doesn't [`show`](LayerFilter::shows) are left out (groups too,
/// since no kind matches them), and the rest keep their depth. Sub-rows follow their layer.
///
/// Groups are rows with `has_children` (open as `is_open`, all open while filtered), so the list
/// draws their triangles and takes `Into` drops on them.
pub(super) fn display_rows<T: LayerTree>(tree: &T, filter: &LayerFilter) -> Vec<RowInfo<LayerRowKey<T::Id>>> {
    // pc: crates/ui-egui/src/panels/layer_tree_ui.rs:22-47 `display_rows`, `panel_rows` (every row while filtered);
    // pc: crates/ui-egui/src/panels/layers.rs:276-284 (filtered rows skipped, depth kept).
    todo!()
}

/// [`display_rows`], kept in egui memory under `id` while [`LayerTree::revision`] and the filter
/// are what they were in the last pass; built again when either changes or the tree gives no
/// revision.
pub(super) fn shared_display_rows<T: LayerTree>(ctx: &Context, id: Id, tree: &T, filter: &LayerFilter) -> Arc<Vec<RowInfo<LayerRowKey<T::Id>>>> {
    // new: lead call 38 (`revision` keeps the display order between passes).
    todo!()
}

/// The list's selection for `selection`: the selected layers' rows in selection order, the lead
/// on the active layer's row. The anchor of a Shift range is kept in egui memory under `id` while
/// the app's selection is the one the list produced in the last pass, else it is the lead.
pub(super) fn list_selection<Id: Clone + PartialEq + Send + Sync + 'static>(
    ctx: &Context,
    id: egui::Id,
    selection: &LayerSelection<Id>,
) -> Selection<LayerRowKey<Id>> {
    // new: the panel holds no selection of its own; the app's is the truth each pass.
    todo!()
}

/// What [`show_body`] returns.
pub(super) struct BodyOutput<Id> {
    /// The body's events in order, the row closures' (`cx.sink`) merged in where they happened.
    pub(super) events: Vec<LayersEvent<Id>>,
    /// While a drag is under way, where it would drop ([`ListOutput::drop_target`](craft_ui::list::ListOutput::drop_target)).
    pub(super) drop_target: Option<Move<LayerRowKey<Id>>>,
    /// A row menu or the eye's colour menu asked for this pass.
    pub(super) menu: Option<MenuRequest<Id>>,
    /// The list's response (its focus, its rect; the row menus open on it).
    pub(super) response: Response,
    /// The least width the rows need ([`min_width`]).
    pub(super) min_width: f32,
    /// Whether a group row was shown, for the panel menu's Collapse All Groups.
    pub(super) any_group: bool,
}

/// Draws the rows: a [`List`](craft_ui::list::List) in tree mode over `rows`, named
/// `LayersLabels::list`, Multiple selection from [`list_selection`], renamable where
/// `info.renamable` (`List::can_rename`), heights from [`row_height`](super::row::row_height),
/// each row from [`build_row`](super::row::build_row), with `reorder` when given, and maps what it
/// reports:
///
/// - `Selection` → [`LayersEvent::Select`] with the pick, the row's layer and the thumbnail the
///   click hit (`RowEvent::ThumbClicked` of the same pass: `First` is `Pixels`, `Extra(i)` the
///   `i`-th mask). A click with modifiers on a thumbnail that [`thumb_click_replaces_select`]
///   reports [`LayersEvent::ThumbClick`] instead, and the list's selection change of that click
///   is dropped. A click on the empty space is `Clear`. A right-click outside the selection
///   selects the row first (`Replace`).
/// - `Row(Toggle)` → [`LayersEvent::Toggle`] (`Visibility` for column 0, `Lock` for the lock
///   column); an `Update` or `Set` with Alt on the eye → [`LayersEvent::ShowOnly`] instead.
/// - `Row(Renamed)` → [`LayersEvent::Rename`] with the trimmed text, dropped when empty or
///   unchanged; Tab and Shift+Tab continue on the next and previous renamable row (the list).
/// - `Activate` → [`LayersEvent::Activate`]: `Thumbnail(part)` when a `ThumbClicked` of the same
///   pass names a thumbnail, `Name` for a double click on the label of a non-renamable layer, else
///   `Row` (also Enter).
/// - `Row(LinkToggled)` → [`LayersEvent::MaskLink`] for the mask at that extra thumbnail.
/// - `Open` → [`LayersEvent::Open`] for layer rows.
/// - `ContextMenu` → `menu`: with `toggle: Some(0)` (the eye column,
///   [`ContextMenuRequest::toggle`](craft_ui::list::ContextMenuRequest::toggle)) the colour menu
///   for the selection or the row; any other `toggle` reports nothing; else the row menu with `part` from the thumbnail
///   rects in `cx.sink.thumbs` under the pointer, `on_selection` from the request.
/// - `Reorder` → [`drop_event`](super::drag::drop_event).
/// - `Row(ThumbDrag)` → [`start_mask_drag`](super::drag::start_mask_drag).
/// - **Keys** while the list has the focus, besides the list's own: Shift+→ and Shift+← on the
///   lead report [`LayersEvent::FxOpen`] (Alt: `all`) when the lead layer has effects or Smart
///   Filters; the menu keys open the row menu at the lead.
/// - **Reveal**: when the active layer differs from the last pass's (not on the panel's first
///   pass), [`reveal_opens`] reports `Open` events for its closed ancestors and the list reveals
///   its row; `reveal` does the same for a layer the app names.
/// - **Thumbnails**: after the list, [`ask_near_thumbnails`](super::row::ask_near_thumbnails).
/// - **Cursor**: while Cmd/Ctrl is held over a thumbnail that loads a selection, the pointing hand
///   with `CursorBadge::{Add, Subtract, Intersect}` for Shift, Alt, Shift+Alt.
pub(super) fn show_body<T: LayerTree>(
    ui: &mut Ui,
    cx: &PanelContext<'_, T>,
    rows: &[RowInfo<LayerRowKey<T::Id>>],
    reorder: Option<Reorder<'_, LayerRowKey<T::Id>>>,
    reveal: Option<T::Id>,
) -> BodyOutput<T::Id> {
    // pc: crates/ui-egui/src/panels/layers.rs:236-339 (rows, scroll, empty-space click, rename);
    // pc: crates/ui-egui/src/panels/layer_row.rs:386-467 `select_row_on_click`, `handle_row_double_click`;
    // pc: crates/ui-egui/src/panels/layer_row.rs:18-31 `thumb_load_badge`, `thumb_load_cursor`;
    // pc: crates/ui-egui/src/panels/layer_reveal.rs:26-62 `track`, `scroll_to_row`.
    todo!()
}

/// Whether a click with `modifiers` on the thumbnail of `part` reports
/// [`LayersEvent::ThumbClick`] instead of selecting: Cmd/Ctrl on any thumbnail that loads a
/// selection (`info.thumb_loads_selection` for `Pixels`; masks always); Shift or Alt on a mask
/// (toggle, view); Alt alone on the pixel thumbnail (fit the layer on screen). Cmd/Ctrl on the
/// Background's thumbnail is a plain click (`Replace`).
pub(super) fn thumb_click_replaces_select(part: LayerPart, modifiers: Modifiers, info: &LayerInfo) -> bool {
    // pc: crates/ui-egui/src/panels/layer_row.rs:216-262 (⌘-click loads; the Background's stands in for a plain click);
    // pc: crates/ui-egui/src/panels/mask_thumbs_ui.rs:199-211 `click_command` (Shift toggles, Alt views).
    todo!()
}

/// The `Open` events that reveal `active`: one per closed ancestor group, outermost first, when
/// `active` differs from `previous` and this is not the panel's first pass (`previous` is `None`
/// then). No event when every ancestor is open.
pub(super) fn reveal_opens<T: LayerTree>(tree: &T, active: Option<&T::Id>, previous: Option<Option<&T::Id>>) -> Vec<OpenChange<T::Id>> {
    // pc: crates/ui-egui/src/panels/layer_reveal.rs:26-51 `track`, `closed_ancestors`.
    todo!()
}

/// The least width of the panel's rows: the [`ListRow::min_width`](craft_ui::list::ListRow::min_width)
/// of a group row one level deep with a layer thumbnail and one mask at the current thumbnail
/// size, the lock column and the app's trailing furniture when given. Thumbnails never shrink;
/// below it the name is cut.
pub(super) fn min_width<T: LayerTree>(ui: &Ui, cx: &PanelContext<'_, T>) -> f32 {
    // new: lead call 9 (PhotoCraft's 14 pt thumbnail shrink, `mask_thumbs_ui.rs:69-74`, is dropped).
    todo!()
}
