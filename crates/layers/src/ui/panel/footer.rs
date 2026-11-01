//! The footer: `LayersFooterButton`, `LayersFooterAction`, press menus and footer drops.

use craft_ui::behavior::ListDrag;
use craft_ui::composites::PanelFooter;
use craft_ui::icons::Icon;
use craft_ui::menu::PressMenu;
use egui::{Id, Ui};

use super::context::PanelContext;
use super::events::LayersEvent;
use crate::ui::tree::{LayerRowKey, LayerTree};

/// A footer button that takes drops of rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LayersFooterButton {
    /// Delete: layers, effects, Smart Filters and mask thumbnails.
    Delete,
    /// Create a new layer: duplicates the dropped layers.
    NewLayer,
    /// Create a new group: groups the dropped layers.
    NewGroup,
}

/// A click on a footer button, with what its modifiers ask (PhotoCraft, Photoshop).
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum LayersFooterAction {
    /// Link layers (enabled by [`LayersFooterState::link_enabled`](crate::ui::tree::LayersFooterState::link_enabled)).
    Link,
    /// Create a new layer above the active one; `ask` (Alt) opens the New Layer dialog first;
    /// `below` (Cmd/Ctrl) puts it below.
    NewLayer {
        /// Alt held: ask first.
        ask: bool,
        /// Cmd/Ctrl held: below the active layer.
        below: bool,
    },
    /// Create a new group; `from_selection` when Shift is held or two or more layers are
    /// selected (Group Layers); `ask` (Alt) opens the New Group dialog; `below` (Cmd/Ctrl) adds an
    /// empty group below.
    NewGroup {
        /// Group the selected layers.
        from_selection: bool,
        /// Alt held: ask first.
        ask: bool,
        /// Cmd/Ctrl held: an empty group below.
        below: bool,
    },
    /// Delete what the button names (the app's `delete_label`); `confirm` is false with Alt
    /// (Photoshop deletes without asking).
    Delete {
        /// Ask before deleting.
        confirm: bool,
    },
    /// Add a mask (the button never replaces one); `invert` (Alt): Hide All or Hide Selection.
    AddMask {
        /// Alt held.
        invert: bool,
    },
    /// A pick in the fill or adjustment menu: the app's command.
    Adjustment(String),
    /// A pick in the layer style menu: the app's command.
    Style(String),
}

/// Draws the footer for the panel whose list has the id `list` (a drop is taken only from that
/// list's drags, not from another panel's) with [`PanelFooter`](craft_ui::composites::PanelFooter) and returns its
/// events in order.
///
/// - **Buttons**, from the reading start: Link, Layer Style (`fx`, a menu button), Mask, Fill or
///   Adjustment (a menu button), New Group, New Layer, Delete (`PanelFooter` puts its first
///   button at the reading end, so they are added Delete first). Tooltips and names: the
///   `LayersLabels` footer words, the app's `mask_label` and `delete_label`; Link greys without
///   `link_enabled`, Mask without a `mask_label`, Delete without `delete_enabled`.
/// - **Clicks** report [`LayersEvent::Footer`] with the modifiers read at the click: New Layer
///   `ask` = Alt, `below` = Cmd/Ctrl; New Group `from_selection` = Shift or two or more selected
///   layers (and an active layer), `ask` = Alt, `below` = Cmd/Ctrl; Delete `confirm` = !Alt;
///   Mask `invert` = Alt.
/// - **Menus** open on the press ([`PressMenu`](craft_ui::menu::PressMenu)) with the app's
///   `adjustment_menu` and `style_menu`; a release over an item picks it: `Adjustment(command)`,
///   `Style(command)`.
/// - **Drops**: Delete is a [`drop_button`](PanelFooter::drop_button): enabled only while an
///   accepted drag is over it when `delete_enabled` is off (Photoshop deletes dragged layers with
///   nothing else selected). Right after New Layer and New Group,
///   `PanelFooter::drop_target::<ListDrag<LayerRowKey<Id>>>`. Both use [`accepts_footer_drop`];
///   a drop reports [`LayersEvent::FooterDrop`] with the payload's items. The list's drag of
///   the same rows ends as `Cancel(Dismissed)` in the same pass.
/// - Keyboard: each button is a Tab stop; the menu buttons open with Space and Enter.
pub(super) fn show_footer<T: LayerTree>(ui: &mut Ui, cx: &PanelContext<'_, T>, list: Id) -> Vec<LayersEvent<T::Id>> {
    // pc: crates/ui-egui/src/panels/layers.rs:371-503 (buttons, modifiers, menus, drops);
    // pc: crates/ui-egui/src/panels/layer_drag.rs:40-48 `dropped_on`.
    let state = cx.tree.footer();
    let labels = cx.labels;
    let (alt, command, shift) = ui.input(|i| (i.modifiers.alt, i.modifiers.command, i.modifiers.shift));
    let group_selection = (shift || cx.selection.layers.len() > 1) && cx.selection.active.is_some();
    let mut events = Vec::new();
    let drop_on = |ui: &Ui, button: &egui::Response, which: LayersFooterButton, events: &mut Vec<LayersEvent<T::Id>>| {
        let accepts = |drag: &ListDrag<LayerRowKey<T::Id>>| accepts_footer_drop(which, drag, list);
        if let Some(drag) = PanelFooter::drop_target::<ListDrag<LayerRowKey<T::Id>>>(ui, button, accepts) {
            events.push(LayersEvent::FooterDrop { button: which, rows: drag.items.clone() });
        }
    };
    PanelFooter::show(ui, |ui| {
        // The first button added sits at the reading end, so Delete goes first.
        let (delete, dropped) =
            PanelFooter::drop_button::<ListDrag<LayerRowKey<T::Id>>>(ui, Icon::Trash, &state.delete_label, state.delete_enabled, |drag| {
                accepts_footer_drop(LayersFooterButton::Delete, drag, list)
            });
        if delete.clicked() {
            events.push(LayersEvent::Footer(LayersFooterAction::Delete { confirm: !alt }));
        }
        if let Some(drag) = dropped {
            events.push(LayersEvent::FooterDrop { button: LayersFooterButton::Delete, rows: drag.items.clone() });
        }

        let new_layer = PanelFooter::button(ui, Icon::NewLayer, &labels.new_layer);
        if new_layer.clicked() {
            events.push(LayersEvent::Footer(LayersFooterAction::NewLayer { ask: alt, below: command }));
        }
        drop_on(ui, &new_layer, LayersFooterButton::NewLayer, &mut events);

        let new_group = PanelFooter::button(ui, Icon::NewFolder, &labels.new_group);
        if new_group.clicked() {
            events.push(LayersEvent::Footer(LayersFooterAction::NewGroup { from_selection: group_selection, ask: alt, below: command }));
        }
        drop_on(ui, &new_group, LayersFooterButton::NewGroup, &mut events);

        let adjustment = PanelFooter::menu_button(ui, Icon::AdjustmentLayer, &labels.add_adjustment, false);
        if let Some(chosen) = PressMenu::new(&state.adjustment_menu).show(ui, &adjustment).chosen {
            events.push(LayersEvent::Footer(LayersFooterAction::Adjustment(chosen)));
        }

        let mask_name = state.mask_label.as_deref().unwrap_or(&labels.add_mask);
        let mask = ui.add_enabled_ui(state.mask_label.is_some(), |ui| PanelFooter::button(ui, Icon::LayerMask, mask_name)).inner;
        if mask.clicked() {
            events.push(LayersEvent::Footer(LayersFooterAction::AddMask { invert: alt }));
        }

        let style = PanelFooter::menu_button(ui, Icon::Fx, &labels.add_style, false);
        if let Some(chosen) = PressMenu::new(&state.style_menu).show(ui, &style).chosen {
            events.push(LayersEvent::Footer(LayersFooterAction::Style(chosen)));
        }

        let link = ui.add_enabled_ui(state.link_enabled, |ui| PanelFooter::button(ui, Icon::Link, &labels.link_layers)).inner;
        if link.clicked() {
            events.push(LayersEvent::Footer(LayersFooterAction::Link));
        }
    });
    events
}

/// Whether `button` takes a drop of `drag`: only a drag of the panel's own list (`list`); Delete
/// takes every row kind and a mask; New Layer and New Group take layer rows only (no effects,
/// filters or masks).
pub(super) fn accepts_footer_drop<K>(button: LayersFooterButton, drag: &ListDrag<LayerRowKey<K>>, list: Id) -> bool {
    // pc: crates/ui-egui/src/panels/layers.rs:345-357 (which drags the footer reads);
    // pc: crates/ui-egui/src/panels/layer_drag.rs:32-62 (`footer_drop`, `fx_trash_drop`).
    if drag.list != list || drag.items.is_empty() {
        return false;
    }
    match button {
        LayersFooterButton::Delete => true,
        LayersFooterButton::NewLayer | LayersFooterButton::NewGroup => drag.items.iter().all(LayerRowKey::is_layer),
    }
}
