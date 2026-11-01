//! `LayersPanel`: the whole panel, its output and events (F11.01).
//!
//! The panel shows the app's [`LayerTree`] and reports what the user did as [`LayersEvent`]s in
//! the order they happened; the app applies them with its own commands and undo (owner
//! question 1). It holds no document and no selection of its own.

mod body;
#[cfg(test)]
mod body_tests;
mod context;
mod drag;
#[cfg(test)]
mod drag_tests;
mod events;
mod footer;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod footer_tests;
mod indicators;
mod labels;
mod menu;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod panel_tests;
mod row;
#[cfg(all(test, feature = "screen-testing", not(target_arch = "wasm32")))]
mod row_tests;
mod style;

pub use events::{ActivateOn, LayerColumn, LayerDrop, LayerMove, LayerSelect, LayerToggle, LayersEvent, MenuTarget, RowTransfer, SubRowMove};
pub use footer::{LayersFooterAction, LayersFooterButton};
pub use labels::LayersLabels;
pub use menu::LayersMenuAction;
pub use style::LayersStyle;

use std::cell::RefCell;

use craft_ui::behavior::ListDrag;
use craft_ui::canvas::textures::{ThumbSource, Thumbnails};
use craft_ui::composites::PanelFooter;
use craft_ui::list::{List, ListRow, paint_ghost};
use craft_ui::localized::Labels as _;
use craft_ui::menu::MenuEntry;
use craft_ui::theme::{Tokens, TokensOf as _};
use egui::{AsIdSalt, DragAndDrop, Id, IdSalt, KeyboardShortcut, Response, Ui};

use body::{list_id, min_width, reveal_plan, shared_display_rows, show_body};
use context::{PanelContext, RowMenu, RowSink, ThumbHost, Trailing};
use drag::{drop_preview, reorder};
use footer::show_footer;
use menu::{note_drawn, show_row_menus};

use super::filter::{LayerFilter, LayerFilterBar};
use super::options::{LayersOptions, LayersSettings};
use super::rows::BlendRows;
use super::thumbs::LayerThumb;
use super::tree::{LayerInfo, LayerRowKey, LayerTree};

/// The buttons of the footer: Delete, New Layer, New Group, Adjustment, Mask, `fx`, Link.
const FOOTER_BUTTONS: usize = 7;

/// The Layers panel over the app's [`LayerTree`].
///
/// ```ignore
/// let tree = DocLayers::new(&doc, &ui_state);
/// let out = LayersPanel::new("layers", &tree)
///     .settings(&mut prefs.layers)
///     .thumbnails(&app.layer_thumbs, &DocLayerThumbs::new(&doc))
///     .filter(&mut ui_state.layer_filter)
///     .visibility_key(keys.layer_visibility)
///     .row_menu(|target| layer_menu_entries(&doc, target))
///     .show(ui);
/// for event in out.events { app.apply(event) }
/// if let Some(preview) = out.drop_preview { canvas.preview(preview) }
/// ```
///
/// Top to bottom: the filter bar ([`LayerFilterBar`](super::filter::LayerFilterBar), while
/// Show Filters is on and [`filter_bar`](Self::filter_bar) is not off), the blend, Opacity, Lock
/// and Fill rows ([`BlendRows`](super::rows::BlendRows) over `tree.properties()`), the rows (a
/// [`List`](craft_ui::list::List) in tree mode filling the height left), the footer. Row heights
/// and thumbnail sizes come from the Size preset's `list.*` tokens and the Panel Options'
/// thumbnail size.
///
/// - **Focus and keys**: the rows are one Tab stop (the list's keys: ↑ ↓ Home End Page, with
///   Shift; Ctrl+↑ ↓ moves the lead; ← → close, open, go to parent or first child, Alt for every
///   group; Shift+→ ← open and close the lead's effects, Alt for every layer; Enter activates;
///   F2 renames; Cmd/Ctrl+] [ moves the selection, Shift to the ends; the Menu key and Shift+F10
///   open the row menu; the app's visibility key toggles the selected rows' eyes as one gesture).
///   Delete is not claimed. The filter bar's, blend rows' and footer's controls are their own
///   stops.
/// - **Pointer**: see the spec's behaviour table (`step6a-layers.md`): clicks select (Cmd/Ctrl
///   toggles, Shift extends, the empty space clears), the eye and lock columns sweep, Alt-click
///   on an eye shows only that layer, the eye's right-click opens its colour menu, a modified
///   thumbnail click is a [`LayersEvent::ThumbClick`], double clicks rename or activate, rows drag
///   to reorder (Alt copies) and onto the footer.
/// - **Right to left**: the eye column and the disclosure sit at the reading start, thumbnails
///   and indicators mirror, the footer starts at the reading end.
/// - **AccessKit**: a `Tree` named "Layers", `multiselectable`, rows `TreeItem` with `level`,
///   `expanded` for groups and their description ([`LayersLabels`] kind and flag words).
#[must_use = "show the panel with `show`"]
pub struct LayersPanel<'a, T: LayerTree> {
    id_salt: IdSalt,
    tree: &'a T,
    settings: Option<&'a mut LayersSettings>,
    thumbnails: Option<ThumbHost<'a, T::Id>>,
    filter: Option<&'a mut LayerFilter>,
    labels: Option<LayersLabels>,
    visibility_key: Option<KeyboardShortcut>,
    lock_column: bool,
    blend_rows: bool,
    filter_bar: bool,
    row_menu: Option<Box<RowMenu<'a, T::Id>>>,
    trailing: Option<Trailing<'a, T::Id>>,
    reveal: Option<T::Id>,
    rename: Option<T::Id>,
}

/// What [`LayersPanel::show`] returns.
#[derive(Debug)]
#[must_use = "the app applies the panel's events"]
pub struct LayersOutput<Id> {
    /// What the user did this pass, in order.
    pub events: Vec<LayersEvent<Id>>,
    /// While rows are dragged and Preview on canvas when reordering layers is on, the layer move
    /// the drag would make now, for the app's canvas preview; `None` otherwise.
    pub drop_preview: Option<LayerMove<Id>>,
    /// The panel.
    pub response: Response,
    /// The least width the panel needs ([`LayersPanel::min_width`]).
    pub min_width: f32,
}

impl<'a, T: LayerTree> LayersPanel<'a, T> {
    /// The panel's own sizes and colours for `tokens`: the `layers` family (the list's and the
    /// footer's come from [`ListRow::style`](craft_ui::list::ListRow::style) and the footer's).
    pub fn style(tokens: &craft_ui::theme::UiTokens) -> LayersStyle {
        LayersStyle::of(tokens)
    }

    /// The panel showing `tree`; `id_salt` keeps its scroll position, rename, drag and menus.
    pub fn new(id_salt: impl AsIdSalt, tree: &'a T) -> Self {
        Self {
            id_salt: IdSalt::new(id_salt),
            tree,
            settings: None,
            thumbnails: None,
            filter: None,
            labels: None,
            visibility_key: None,
            lock_column: false,
            blend_rows: true,
            filter_bar: true,
            row_menu: None,
            trailing: None,
            reveal: None,
            rename: None,
        }
    }

    /// The panel's persisted section: the Panel Options and Show Filters. Without it the panel
    /// uses [`LayersSettings::default`].
    pub fn settings(mut self, settings: &'a mut LayersSettings) -> Self {
        self.settings = Some(settings);
        self
    }

    /// The thumbnail cache and the app's source of the rows' pictures (one
    /// [`LayerThumb`] key per thumbnail). Without it the rows draw kind tiles and placeholders.
    pub fn thumbnails(mut self, thumbnails: &'a Thumbnails<LayerThumb<T::Id>>, source: &'a dyn ThumbSource<LayerThumb<T::Id>>) -> Self {
        self.thumbnails = Some(ThumbHost { thumbnails, source });
        self
    }

    /// The filter bar's criteria, which the app stores; the bar edits it. Without it the panel
    /// keeps its own in egui memory.
    pub fn filter(mut self, filter: &'a mut LayerFilter) -> Self {
        self.filter = Some(filter);
        self
    }

    /// The panel's words; the installed words by default.
    pub fn labels(mut self, labels: LayersLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// The app's shortcut that toggles the selected rows' eyes as one gesture (PhotoCraft's
    /// Cmd+,), shown in the eye's tooltip.
    pub fn visibility_key(mut self, key: KeyboardShortcut) -> Self {
        self.visibility_key = Some(key);
        self
    }

    /// A lock toggle column after the eye (VectorCraft, DesignCraft; default off): its edits are
    /// [`LayersEvent::Toggle`] with [`LayerColumn::Lock`].
    pub fn lock_column(mut self, lock_column: bool) -> Self {
        self.lock_column = lock_column;
        self
    }

    /// Shows the blend, Opacity, Lock and Fill rows (default on; VectorCraft and DesignCraft turn
    /// them off).
    pub fn blend_rows(mut self, blend_rows: bool) -> Self {
        self.blend_rows = blend_rows;
        self
    }

    /// Allows the filter bar (default on; it shows while Show Filters is on). VectorCraft turns
    /// it off and filters `children()` itself.
    pub fn filter_bar(mut self, filter_bar: bool) -> Self {
        self.filter_bar = filter_bar;
        self
    }

    /// The host's row menu: called with where it was opened (a layer row, a mask thumbnail with
    /// its `part`, an effects row) each pass while the menu is open; a pick is
    /// [`LayersEvent::Menu`]. A right-click outside the selection selects the row first.
    pub fn row_menu(mut self, menu: impl Fn(&MenuTarget<T::Id>) -> Vec<MenuEntry<String>> + 'a) -> Self {
        self.row_menu = Some(Box::new(menu));
        self
    }

    /// The app's furniture at each layer row's trailing end, in a `width` wide cell after the
    /// indicators (Illustrator's target circle, a selection square): `add(ui, id, info)`. It is
    /// never dropped for the name; the panel's minimum width includes it.
    pub fn trailing(mut self, width: f32, add: impl Fn(&mut Ui, &T::Id, &LayerInfo) + 'a) -> Self {
        self.trailing = Some(Trailing { width, add: Box::new(add) });
        self
    }

    /// Reveals `layer` this pass: its closed ancestors open ([`LayersEvent::Open`]) and its row
    /// scrolls into view when hidden. The active layer is revealed by itself when it changes.
    pub fn reveal(mut self, layer: T::Id) -> Self {
        self.reveal = Some(layer);
        self
    }

    /// Opens the rename field of `layer`'s row this pass, where the layer is renamable: the app
    /// calls it in the pass it handles its Rename Layer… command from the row menu, as it calls
    /// [`List::start_rename`](craft_ui::list::List::start_rename).
    pub fn start_rename(mut self, layer: T::Id) -> Self {
        self.rename = Some(layer);
        self
    }

    /// The least width the panel needs at the current settings: a group row one level deep with
    /// a layer thumbnail and one mask, the lock column and the trailing furniture when set, and
    /// the footer's buttons. A host sets its dock's minimum from it.
    pub fn min_width(&self, ui: &Ui) -> f32 {
        // new: lead call 9; body::min_width for the rows, `PanelFooter`'s buttons for the footer.
        let fallback = LayersSettings::default();
        let settings = self.settings.as_deref().unwrap_or(&fallback);
        let labels = self.labels.clone().unwrap_or_else(|| LayersLabels::of(ui));
        let cx = self.context(ui, Id::NULL.with(self.id_salt), &settings.options, &labels);
        panel_min_width(ui, min_width(ui, &cx))
    }

    fn context<'b>(&'b self, ui: &Ui, id: Id, options: &'b LayersOptions, labels: &'b LayersLabels) -> PanelContext<'b, T>
    where
        'a: 'b,
    {
        let tokens = Tokens::of(ui);
        PanelContext {
            ctx: ui.ctx().clone(),
            id,
            tree: self.tree,
            selection: self.tree.selection(),
            options,
            thumbs: self.thumbnails,
            labels,
            style: LayersStyle::of(&tokens),
            row_style: ListRow::<LayerRowKey<T::Id>>::style(&tokens),
            lock_column: self.lock_column,
            visibility_key: self.visibility_key,
            trailing: self.trailing.as_ref(),
            sink: RefCell::new(RowSink::default()),
        }
    }

    /// Draws the panel over the available rect.
    ///
    /// Builds the pass's `PanelContext`, then in order: the filter bar (its edits change the
    /// filter; no event), [`BlendRows`](super::rows::BlendRows) (its edits are
    /// [`LayersEvent::Properties`]), the rows (`body::show_body` over
    /// `body::shared_display_rows`, with `drag::reorder`; the reveal's opens are listed open in the same pass), the row menus (`menu::show_row_menus`),
    /// the footer (`footer::show_footer`). Events are reported in that order; the drop preview
    /// is `drag::drop_preview` of the body's drop target.
    pub fn show(mut self, ui: &mut Ui) -> LayersOutput<T::Id> {
        // pc: crates/ui-egui/src/panels/layers.rs:104-503 `layers` (filter bar, blend rows, rows, footer).
        let fallback = LayersSettings::default();
        let settings = self.settings.take();
        let given_filter = self.filter.take();
        let settings_ref: &LayersSettings = settings.as_deref().unwrap_or(&fallback);
        let labels = self.labels.clone().unwrap_or_else(|| LayersLabels::of(ui));
        let id = Id::NULL.with(self.id_salt);
        let cx = self.context(ui, id, &settings_ref.options, &labels);

        let own_key = id.with("filter");
        let mut own_filter =
            if given_filter.is_none() { ui.data(|d| d.get_temp::<LayerFilter>(own_key)).unwrap_or_default() } else { LayerFilter::default() };
        let uses_own_filter = given_filter.is_none();
        let filter: &mut LayerFilter = match given_filter {
            Some(filter) => filter,
            None => &mut own_filter,
        };

        let tokens = Tokens::of(ui);
        let footer_height = PanelFooter::reserved_height(&PanelFooter::style(&tokens, ui.pixels_per_point())) + ui.spacing().item_spacing.y;
        let mut min = 0.0;
        let mut events = Vec::new();
        let mut preview = None;
        let inner = ui.scope(|ui| {
            if self.filter_bar && settings_ref.show_filters {
                let modes = self.tree.blend_modes(true, &filter.mode);
                let _ = LayerFilterBar::new(id.with("filter-bar"), filter).blend_modes(&modes).show(ui);
                ui.add_space(ui.spacing().item_spacing.y);
            }
            if self.blend_rows {
                let properties = self.tree.properties();
                let modes =
                    self.tree.blend_modes(properties.as_ref().is_some_and(|p| p.group), properties.as_ref().map_or("normal", |p| &p.blend_key));
                let out = BlendRows::new(id.with("blend"), properties.as_ref(), &modes).show(ui);
                events.extend(out.events.into_iter().map(LayersEvent::Properties));
                ui.add_space(ui.spacing().item_spacing.y);
            }

            let reveal = reveal_plan(&cx, self.reveal.clone());
            let display = shared_display_rows(&cx.ctx, id, self.tree, filter, &reveal.opened());
            let list_height = (ui.available_height() - footer_height).max(0.0);
            let (body, list) = ui
                .scope(|ui| {
                    ui.set_max_height(list_height);
                    let body = show_body(ui, &cx, &display, Some(reorder(&cx)), reveal, self.rename.clone());
                    let list = list_id(ui, id);
                    paint_mask_ghost(ui, &cx, list, body.response.rect.width());
                    (body, list)
                })
                .inner;
            preview = drop_preview(body.drop_target.as_ref(), cx.options);
            min = panel_min_width(ui, body.min_width);
            events.extend(body.events);
            events.extend(show_row_menus(ui, &cx, &body.response, self.row_menu.as_deref(), body.menu));
            events.extend(show_footer(ui, &cx, list));
        });

        note_drawn(&cx.ctx, self.id_salt);
        if uses_own_filter {
            ui.data_mut(|d| d.insert_temp(own_key, own_filter));
        }
        LayersOutput {
            events,
            drop_preview: preview.filter(|_| cx.options.preview_on_canvas_when_reordering),
            response: inner.response,
            min_width: min,
        }
    }
}

/// The footer's buttons side by side.
fn footer_width(ui: &Ui) -> f32 {
    let style = PanelFooter::style(&Tokens::of(ui), ui.pixels_per_point());
    FOOTER_BUTTONS as f32 * style.button + (FOOTER_BUTTONS - 1) as f32 * style.gap
}

fn panel_min_width(ui: &Ui, rows: f32) -> f32 {
    rows.max(footer_width(ui))
}

/// While a mask thumbnail is dragged, its ghost beside the pointer: the list paints only the
/// ghost of its own row drags, and the mask's payload carries the label.
fn paint_mask_ghost<T: LayerTree>(ui: &Ui, cx: &PanelContext<'_, T>, list: Id, width: f32) {
    let Some(drag) = DragAndDrop::payload::<ListDrag<LayerRowKey<T::Id>>>(ui.ctx()) else { return };
    let is_mask = drag.list == list && matches!(drag.items.as_slice(), [LayerRowKey::Mask(..)]);
    if !is_mask || !ui.input(|i| i.pointer.any_down()) {
        return;
    }
    if let Some(pointer) = ui.input(|i| i.pointer.interact_pos()) {
        paint_ghost(ui, list, &cx.row_style, &drag.label, pointer, width, None);
    }
}
