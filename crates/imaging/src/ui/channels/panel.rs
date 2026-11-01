//! `ChannelsPanel`: list wiring, events, selection and thumbnails.

use std::fmt::Debug;
use std::hash::Hash;

use craft_ui::behavior::{Axis, Reorder, RowInfo, Selection, SelectionMode, SelectionPick};
use craft_ui::canvas::SelectionOperation;
use craft_ui::canvas::textures::{ThumbSource, Thumbnails};
use craft_ui::composites::PanelFooter;
use craft_ui::list::{List, ListEvent, ListRow, ListRowStyle, RowEvent, ThumbSize};
use craft_ui::localized::SharedLabels as _;
use craft_ui::menu::OpenAt;
use craft_ui::theme::{Tokens, TokensOf as _};
use egui::{AsIdSalt, Context, Id, IdSalt, Key, Modifiers, Ui, Vec2};

use super::footer::show_footer;
use super::labels::{ChannelsLabels, shortcut_numbers};
use super::menu::{menu_panel_id, show_row_menu, take_pending_rename};
use super::model::{ChannelKind, ChannelRow, ChannelsEvent, ChannelsOutput, targeted_row};
use super::rows::channel_row;
use super::style::ChannelsStyle;

/// The Channels panel over the app's rows: composite, colour, layer-mask, alpha and spot
/// channels and Quick Mask, with thumbnails, eyes, the footer and the menus. It holds no
/// document; what the user did comes back as [`ChannelsEvent`](super::ChannelsEvent)s.
///
/// ```ignore
/// let rows = channel_rows(&doc, &view);                     // the app, once per pass
/// let out = ChannelsPanel::new("channels", &rows).thumbnails(&thumbs, &DocChannelThumbs { doc }).show(ui);
/// for event in out.events {
///     match event {
///         ChannelsEvent::Target(key) => app.run("channel.target", key),
///         ChannelsEvent::LoadSelection { key, operation } => app.load_selection(key, operation.name()),
///         _ => {}
///     }
/// }
/// ```
///
/// - **Layout**: the rows fill the panel's height above a
///   [`PanelFooter`](craft_ui::composites::PanelFooter); a [`List`](craft_ui::list::List) of
///   `ListRow`s in the layer form (`rows::channel_row`), every row one height
///   (`ListRowStyle::height` of the thumbnail side). The thumbnail side is
///   [`thumbnail_size`](Self::thumbnail_size)'s, else `channels.thumb` (28 at Compact, 30 at
///   Comfortable).
/// - **Selection**: the list shows the rows the app marks `targeted` as selected (several while
///   the composite is targeted), so it runs in Multiple mode over a selection rebuilt from the
///   rows each pass; its own changes are not kept. A click (Shift or not) reports
///   [`Target`](super::ChannelsEvent::Target); Cmd/Ctrl-click reports
///   [`LoadSelection`](super::ChannelsEvent::LoadSelection) with
///   `SelectionOperation::from_modifiers` instead; ↑ ↓ report `Target` for the new lead;
///   Cmd/Ctrl+Enter on the lead reports `LoadSelection`.
/// - **Eyes**: the list's toggle column; a click or a sweep reports
///   [`Visibility`](super::ChannelsEvent::Visibility); the eye's click doesn't target.
/// - **Double click and Enter**: on the thumbnail of a row with options, or Enter on it,
///   [`Options`](super::ChannelsEvent::Options); on the name of an alpha or spot row, rename in
///   place (also F2): Enter or a click elsewhere commits, Esc cancels; an empty or unchanged name
///   reports nothing, else [`Rename`](super::ChannelsEvent::Rename).
/// - **Drag**: any row drags with the list's ghost. Alpha and spot rows reorder: the list's
///   marker shows before or after another alpha or spot row and before Quick Mask, and nowhere
///   else ([`Reorder::accepts`](craft_ui::behavior::Reorder::accepts));
///   [`Reorder`](super::ChannelsEvent::Reorder) reports it (Cmd/Ctrl+] and [ too). Any row drops
///   on the footer's Delete, New and Load buttons. A release elsewhere does nothing; Esc cancels.
/// - **Menus**: a right-click or the Menu key opens the row's context menu
///   (`menu::row_menu_entries`); the panel menu is
///   [`panel_menu`](Self::panel_menu).
/// - **Tab**: one stop for the list, then each footer button.
/// - **AccessKit**: a `ListBox` named "Channels"; rows named by the channel name, selected from
///   `targeted`, described "Hidden" when not visible; the eye cells named "Visibility {row}";
///   thumbnails add no nodes.
/// - **Right to left**: the eye, thumbnail and chord follow the list's direction rules; the
///   chord sits at the trailing end.
#[must_use = "show the panel with `show`"]
pub struct ChannelsPanel<'a, K: Clone + Eq + Hash + Debug + Send + Sync + 'static> {
    id_salt: IdSalt,
    rows: &'a [ChannelRow<K>],
    thumbnails: Option<(&'a Thumbnails<K>, &'a dyn ThumbSource<K>)>,
    thumbnail_size: Option<ThumbSize>,
    labels: Option<ChannelsLabels>,
}

/// One pass's shared state of the panel, lent to the rows.
pub(super) struct ChannelsContext<'a, K: Clone + Eq + Hash + Send + Sync + 'static> {
    /// The context, for `Thumbnails::state` inside the list's row builder.
    pub(super) ctx: Context,
    /// The panel's id: the list, the menu and a pending rename keep state under it.
    pub(super) id: Id,
    /// The thumbnail cache and source; `None` draws placeholders only.
    pub(super) thumbnails: Option<(&'a Thumbnails<K>, &'a dyn ThumbSource<K>)>,
    /// The thumbnail side in points.
    pub(super) thumb_side: f32,
    /// The panel's words.
    pub(super) labels: &'a ChannelsLabels,
    /// The panel's style.
    pub(super) style: ChannelsStyle,
}

impl<'a, K: Clone + Eq + Hash + Debug + Send + Sync + 'static> ChannelsPanel<'a, K> {
    /// The panel showing `rows` in their order; `id_salt` keeps its scroll position, rename, drag
    /// and menu.
    pub fn new(id_salt: impl AsIdSalt, rows: &'a [ChannelRow<K>]) -> Self {
        Self { id_salt: IdSalt::new(id_salt), rows, thumbnails: None, thumbnail_size: None, labels: None }
    }

    /// The thumbnail cache and the app's source of the channels' pictures, keyed by the rows'
    /// keys. Without it the rows draw placeholders.
    pub fn thumbnails(mut self, thumbnails: &'a Thumbnails<K>, source: &'a dyn ThumbSource<K>) -> Self {
        self.thumbnails = Some((thumbnails, source));
        self
    }

    /// The user's Panel Options thumbnail size (Photoshop's; PhotoCraft sets none): `Small`,
    /// `Medium` and `Large` use `list.thumb.*`, `None` hides the thumbnails. Without it the side is
    /// `channels.thumb`.
    pub fn thumbnail_size(mut self, size: ThumbSize) -> Self {
        self.thumbnail_size = Some(size);
        self
    }

    /// The panel's words; the installed words by default.
    pub fn labels(mut self, labels: ChannelsLabels) -> Self {
        self.labels = Some(labels);
        self
    }

    /// Draws the panel over the available rect.
    pub fn show(self, ui: &mut Ui) -> ChannelsOutput<K> {
        // pc: crates/ui-egui/src/panels/channels_panel.rs:285-515 `show` (rows, clicks, drag, footer);
        // pc: crates/ui-egui/src/panels/channels_panel.rs:91-157 (click, Cmd-click, double click, rename);
        // pc: crates/ui-egui/src/panels/channels_panel.rs:207-215 `drop_insertion` (alpha targets).
        let Self { id_salt, rows, thumbnails, thumbnail_size, labels } = self;
        let tokens = Tokens::of(ui);
        let labels = labels.map_or_else(|| ChannelsLabels::shared(ui), std::sync::Arc::new);
        let style = ChannelsStyle::of(&tokens);
        let row_style = ListRow::<K>::style(&tokens);
        let id = menu_panel_id(id_salt);
        let cx = ChannelsContext {
            ctx: ui.ctx().clone(),
            id,
            thumbnails,
            thumb_side: thumb_side(thumbnail_size, &style, &row_style),
            labels: &labels,
            style,
        };
        let footer = PanelFooter::reserved_height(&PanelFooter::style(&tokens, ui.pixels_per_point()));
        let list_height = (ui.available_height() - footer).max(0.0);
        let inner = ui.scope(|ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            let mut events = show_list(ui, &cx, rows, list_height);
            events.extend(show_footer(ui, rows, &labels));
            events
        });
        ChannelsOutput { events: inner.inner, response: inner.response }
    }
}

/// The kind of the row `key`.
fn kind_of<K: PartialEq>(rows: &[ChannelRow<K>], key: &K) -> Option<ChannelKind> {
    rows.iter().find(|row| &row.key == key).map(|row| row.kind)
}

/// Whether a drop of rows `items` at `target` reorders channels as Photoshop does: only alpha and
/// spot rows move, before or after another one or before Quick Mask.
fn reorder_accepts<K: PartialEq>(rows: &[ChannelRow<K>], mv: &craft_ui::behavior::Move<K>) -> bool {
    use craft_ui::behavior::DropPosition;
    let moves = mv.items.iter().all(|key| kind_of(rows, key).is_some_and(ChannelKind::moves));
    let slot = match kind_of(rows, &mv.target) {
        Some(ChannelKind::Alpha { .. } | ChannelKind::Spot { .. }) => mv.position != DropPosition::Into,
        Some(ChannelKind::QuickMask { .. }) => mv.position == DropPosition::Before,
        _ => false,
    };
    moves && slot && !mv.copy
}

/// The list of rows, the row context menu and the keyboard load; returns the events in order.
fn show_list<K: Clone + Eq + Hash + Debug + Send + Sync + 'static>(
    ui: &mut Ui,
    cx: &ChannelsContext<'_, K>,
    rows: &[ChannelRow<K>],
    height: f32,
) -> Vec<ChannelsEvent<K>> {
    let order: Vec<RowInfo<K>> = rows.iter().map(|row| RowInfo::leaf(row.key.clone())).collect();
    let numbers = shortcut_numbers(rows);
    let lead = targeted_row(rows).map(|row| row.key.clone());
    let mut selection =
        Selection { items: rows.iter().filter(|row| row.targeted).map(|row| row.key.clone()).collect(), lead: lead.clone(), anchor: lead };
    let thumbs = if cx.thumb_side > 0.0 { ThumbSize::Px(cx.thumb_side.round().clamp(0.0, f32::from(u16::MAX)) as u16) } else { ThumbSize::None };
    let mut list = List::new((cx.id, "list"), cx.labels.list.clone(), order.as_slice())
        .selection(&mut selection)
        .mode(SelectionMode::Multiple)
        .allow_empty(false)
        .thumbnails(thumbs)
        .max_height(height)
        .can_rename(|key| kind_of(rows, key).is_some_and(ChannelKind::renames))
        .reorder(Reorder::new(Axis::Vertical).accepts(|mv, _| reorder_accepts(rows, mv)));
    // Rename Channel… leaves the row under the panel's id (panel menu) or the list's (row menu).
    let list_id = ui.make_persistent_id((cx.id, "list"));
    if let Some(key) = take_pending_rename::<K>(&cx.ctx, list_id).or_else(|| take_pending_rename::<K>(&cx.ctx, cx.id)) {
        list = list.start_rename(key);
    }
    let out = list.show(ui, |at, _| channel_row(cx, &rows[at], numbers[at]));
    let focused = out.response.has_focus();
    let mut events = list_events(ui, rows, &out.events);
    if focused
        && let Some(key) = selection.lead.clone()
        && let Some(operation) = keyboard_load(ui)
    {
        events.push(ChannelsEvent::LoadSelection { key, operation });
    }
    let request = out.events.iter().find_map(|event| match event {
        ListEvent::ContextMenu(request) => {
            request.row.clone().map(|row| (row, request.pointer.map_or(OpenAt::Rect(request.anchor), OpenAt::Pointer)))
        }
        _ => None,
    });
    events.extend(show_row_menu(ui, &out.response, rows, cx.labels, request));
    events
}

/// Cmd/Ctrl+Enter on the focused list: the operation of Shift and Alt held with it.
fn keyboard_load(ui: &Ui) -> Option<SelectionOperation> {
    ui.input_mut(|input| input.consume_key(Modifiers::COMMAND, Key::Enter).then(|| SelectionOperation::from_modifiers(input.modifiers)))
}

/// What the list's events mean for the app: targets, loads, eyes, renames, options and moves.
fn list_events<K: Clone + PartialEq>(ui: &Ui, rows: &[ChannelRow<K>], events: &[ListEvent<K>]) -> Vec<ChannelsEvent<K>> {
    let modifiers = ui.input(|input| input.modifiers);
    let operation = SelectionOperation::from_modifiers(modifiers);
    let clicked = |key: &K| events.iter().any(|event| matches!(event, ListEvent::Clicked(k) if k == key));
    let mut out = Vec::new();
    for event in events {
        match event {
            ListEvent::Clicked(key) if modifiers.command => out.push(ChannelsEvent::LoadSelection { key: key.clone(), operation }),
            ListEvent::Clicked(key) => out.push(ChannelsEvent::Target(key.clone())),
            ListEvent::Selection(change) if change.pick != SelectionPick::Toggle => {
                // A click reports its own `Clicked`; a key move reports only the selection.
                if let Some(key) = change.row.as_ref().filter(|key| !clicked(key)) {
                    out.push(ChannelsEvent::Target(key.clone()));
                }
            }
            ListEvent::Activate(key) if modifiers.command => out.push(ChannelsEvent::LoadSelection { key: key.clone(), operation }),
            ListEvent::Activate(key) if kind_of(rows, key).is_some_and(ChannelKind::has_options) => out.push(ChannelsEvent::Options(key.clone())),
            ListEvent::Row(RowEvent::Toggle(edit)) => out.push(ChannelsEvent::Visibility(edit.clone())),
            ListEvent::Row(RowEvent::Renamed { row, text }) => {
                let name = text.trim();
                if !name.is_empty() && rows.iter().find(|r| &r.key == row).is_some_and(|r| r.name != name) {
                    out.push(ChannelsEvent::Rename { key: row.clone(), name: name.to_owned() });
                }
            }
            ListEvent::Reorder(edit) => out.push(ChannelsEvent::Reorder(edit.clone())),
            _ => {}
        }
    }
    out
}

/// The thumbnail side for `size` (`None`: the style's `channels.thumb`): `list.thumb.*` for
/// `Small`, `Medium` and `Large`, 0 for `ThumbSize::None`, the points of `Px`.
pub(super) fn thumb_side(size: Option<ThumbSize>, style: &ChannelsStyle, row_style: &ListRowStyle) -> f32 {
    // pc: crates/ui-egui/src/panels/channels_panel.rs:375 (28 Pro, 30 Studio: `channels.thumb`).
    size.map_or(style.thumb, |size| row_style.thumb_side(size))
}

#[cfg(test)]
mod tests {
    use craft_ui::behavior::{DropPosition, Move};

    use super::*;
    use crate::ui::channels::ColorIndicates;

    fn mv(items: &[u8], target: u8, position: DropPosition) -> Move<u8> {
        Move { items: items.to_vec(), target, position, copy: false }
    }

    // Colour and mask rows are not targets and do not move; Quick Mask takes only a drop above it.
    #[test]
    fn reorder_accepts_alpha_slots_only() {
        let indicates = ColorIndicates::MaskedAreas;
        let rows = vec![
            ChannelRow::new(0, ChannelKind::Color, "R"),
            ChannelRow::new(1, ChannelKind::Alpha { indicates }, "A1"),
            ChannelRow::new(2, ChannelKind::Alpha { indicates }, "A2"),
            ChannelRow::new(3, ChannelKind::QuickMask { indicates }, "Q"),
        ];
        assert!(reorder_accepts(&rows, &mv(&[1], 2, DropPosition::After)));
        assert!(reorder_accepts(&rows, &mv(&[2], 3, DropPosition::Before)));
        assert!(!reorder_accepts(&rows, &mv(&[2], 3, DropPosition::After)));
        assert!(!reorder_accepts(&rows, &mv(&[1], 0, DropPosition::After)));
        assert!(!reorder_accepts(&rows, &mv(&[0], 2, DropPosition::After)));
    }
}
